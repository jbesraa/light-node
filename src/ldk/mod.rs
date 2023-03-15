use bitcoin::blockdata::constants::genesis_block;
use lightning_net_tokio;
use lightning_net_tokio::SocketDescriptor;

use bitcoin::network::constants::Network;
use lightning::chain::keysinterface::{InMemorySigner, KeysInterface, Recipient};
use lightning::chain::{self, chainmonitor, ChannelMonitorUpdateStatus, Filter, Watch};
use lightning::ln::channelmanager::{
    ChainParameters, ChannelManagerReadArgs, SimpleArcChannelManager,
};

use lightning::ln::peer_handler::{IgnoringMessageHandler, MessageHandler, SimpleArcPeerManager};
use lightning::onion_message::OnionMessenger;
use lightning::routing::gossip::{NetworkGraph, P2PGossipSync};
use lightning::util::config::UserConfig;
use lightning::util::ser::ReadableArgs;
use lightning_background_processor::{BackgroundProcessor, GossipSync};
use lightning_block_sync::init;
use lightning_block_sync::poll;
use lightning_block_sync::UnboundedCache;
use lightning_persister::FilesystemPersister;
use std::convert::TryInto;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

pub mod block_sync;
// pub mod broadcast;
use bitcoin::BlockHash;
// pub mod chain_monitor;
pub mod core;
pub mod keys_manager;
pub mod logger;
pub mod persister;
use rand::Rng;

pub type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<core::CoreLDK>,
    Arc<core::CoreLDK>,
    Arc<logger::MyLogger>,
    Arc<FilesystemPersister>,
>;

pub(crate) type ChannelManager = SimpleArcChannelManager<
    ChainMonitor,
    core::CoreLDK, // broadcast
    core::CoreLDK, // fee estimate
    logger::MyLogger,
>;

type PeerManager = SimpleArcPeerManager<
    SocketDescriptor,
    ChainMonitor,
    core::CoreLDK, // broadcast
    core::CoreLDK, //fee estimate
    dyn chain::Access + Send + Sync,
    logger::MyLogger,
>;

fn read_network(
    path: &Path,
    genesis_hash: BlockHash,
    logger: Arc<logger::MyLogger>,
) -> NetworkGraph<Arc<logger::MyLogger>> {
    if let Ok(file) = File::open(path) {
        if let Ok(graph) = NetworkGraph::read(&mut BufReader::new(file), logger.clone()) {
            return graph;
        }
    }
    NetworkGraph::new(genesis_hash, logger)
}

pub async fn start_node() {
    let ldk_data_dir = format!("{}/.ldk", ".");
    let handle = tokio::runtime::Handle::current();
    let core_ldk: Arc<core::CoreLDK> = match core::CoreLDK::new(handle).await {
        Ok(client) => Arc::new(client),
        Err(e) => {
            println!("FAILED TO START CORELDK: {}", e);
            return;
        }
    };
    let fee_estimator = core_ldk.clone();
    let logger = Arc::new(logger::MyLogger {});
    let broadcaster_interface = core_ldk.clone(); // 3
    let keys_manager = Arc::new(keys_manager::new(&ldk_data_dir)); // 6
    let persister = Arc::new(persister::persister(&ldk_data_dir)); // 4

    let chain_monitor: Arc<ChainMonitor> = Arc::new(chainmonitor::ChainMonitor::new(
        None,
        broadcaster_interface.clone(),
        logger.clone(),
        fee_estimator.clone(),
        persister.clone(),
    ));

    // let chain_monitor: Arc<chain_monitor::ChainMonitor> = Arc::new(chain_monitor::new(
    //     broadcaster_interface.clone(),
    //     logger.clone(),
    //     fee_estimator.clone(),
    //     persister.clone(),
    // )); //5

    // Start step 7: Read ChannelMonitor state from Disk
    let mut channel_monitors = persister
        .read_channelmonitors(keys_manager.clone())
        .unwrap();
    // End step 7

    // Start step 8
    let user_config = UserConfig::default();

    /* RESTARTING */

    let (channel_manager_blockhash, mut channel_manager) = {
        let mut channel_manager_file =
            File::open(format!("{}/manager", ldk_data_dir.clone())).unwrap();

        // Use the `ChannelMonitors` we read from disk in Step 7.
        let mut channel_monitor_mut_references = Vec::new();
        for (_, channel_monitor) in channel_monitors.iter_mut() {
            channel_monitor_mut_references.push(channel_monitor);
        }
        let read_args = ChannelManagerReadArgs::new(
            keys_manager.clone(),
            fee_estimator.clone(),
            chain_monitor.clone(),
            broadcaster_interface.clone(),
            logger.clone(),
            user_config,
            channel_monitor_mut_references,
        );
        <(BlockHash, ChannelManager)>::read(&mut channel_manager_file, read_args).unwrap()
    };

    /* FRESH CHANNELMANAGER */

    let block_source = Arc::new(block_sync::BitcoindClientLight::new().await.unwrap());
    let polled_chain_tip = init::validate_best_block_header(block_source.clone())
        .await
        .expect("Failed to fetch best block header and best block");
    let best_block = polled_chain_tip.to_best_block();
    let best_block_hash = best_block.block_hash();

    let (channel_manager_blockhash, mut channel_manager) = {
        // let best_blockhash = // insert the best blockhash you know of
        // let best_chain_height = // insert the height corresponding to best_blockhash
        let chain_params = ChainParameters {
            network: Network::Regtest, // substitute this with your network
            best_block,
        };
        let fresh_channel_manager = ChannelManager::new(
            fee_estimator.clone(),
            chain_monitor.clone(),
            broadcaster_interface.clone(),
            logger.clone(),
            keys_manager.clone(),
            user_config,
            chain_params,
        );
        (best_block_hash, fresh_channel_manager)
    };
    // End step 8
    let mut chain_listener_channel_monitors = Vec::new();
    let mut cache = UnboundedCache::new();
    let restarting_node = false;
    let mut chain_tip: Option<poll::ValidatedBlockHeader> = None;
    let mut chain_listeners = vec![(
        channel_manager_blockhash,
        &channel_manager as &dyn chain::Listen,
    )];

    for (blockhash, channel_monitor) in channel_monitors.drain(..) {
        let outpoint = channel_monitor.get_funding_txo().0;
        chain_listener_channel_monitors.push((
            blockhash,
            (
                channel_monitor,
                broadcaster_interface.clone(),
                fee_estimator.clone(),
                logger.clone(),
            ),
            outpoint,
        ));
    }

    for monitor_listener_info in chain_listener_channel_monitors.iter_mut() {
        chain_listeners.push((
            monitor_listener_info.0,
            &mut monitor_listener_info.1 as &mut dyn chain::Listen,
        ));
    }

    // Save the chain tip to be used in Step 14.
    chain_tip = Some(
        init::synchronize_listeners(
            block_source.clone().as_ref(),
            Network::Testnet,
            &mut cache,
            chain_listeners,
        )
        .await
        .unwrap(),
    );

    // Step 10: Give ChannelMonitors to ChainMonitor
    for item in chain_listener_channel_monitors.drain(..) {
        let channel_monitor = item.1 .0;
        let funding_outpoint = item.2;
        assert_eq!(
            chain_monitor
                .clone()
                .watch_channel(funding_outpoint, channel_monitor),
            ChannelMonitorUpdateStatus::Completed
        );
    }

    // Step 12: Optional: Initialize the P2PGossipSync
    let genesis = genesis_block(Network::Regtest).header.block_hash();
    let network_graph_path = format!("{}/network_graph", ldk_data_dir.clone());
    let network_graph = Arc::new(read_network(
        Path::new(&network_graph_path),
        genesis,
        logger.clone(),
    ));
    let gossip_sync = Arc::new(P2PGossipSync::new(
        Arc::clone(&network_graph),
        None::<Arc<dyn chain::Access + Send + Sync>>,
        logger.clone(),
    ));
    //

    let mut ephemeral_bytes = [0; 32];
    rand::thread_rng().fill_bytes(&mut ephemeral_bytes);
    let onion_messenger = Arc::new(OnionMessenger::new(
        Arc::clone(&keys_manager),
        Arc::clone(&logger),
        IgnoringMessageHandler {},
    ));
    let lightning_msg_handler = MessageHandler {
        chan_handler: Arc::new(channel_manager),
        route_handler: gossip_sync.clone(),
        onion_message_handler: onion_messenger.clone(),
    };
    let ignoring_custom_msg_handler = IgnoringMessageHandler {};

    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let peer_manager: Arc<PeerManager> = Arc::new(PeerManager::new(
        lightning_msg_handler,
        keys_manager.get_node_secret(Recipient::Node).unwrap(),
        current_time.try_into().unwrap(),
        &ephemeral_bytes,
        logger.clone(),
        ignoring_custom_msg_handler,
    ));

    // Networking step 13
    let peer_manager_connection_handler = peer_manager.clone();
    let listen_port = 9735;
    let stop_listen_connect = Arc::new(AtomicBool::new(false));
    let stop_listen = Arc::clone(&stop_listen_connect);
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", listen_port))
            .await
            .expect("Hey. im down");
        loop {
            let peer_mgr = peer_manager_connection_handler.clone();
            let tcp_stream = listener.accept().await.unwrap().0;
            if stop_listen.load(Ordering::Acquire) {
                return;
            }
            tokio::spawn(async move {
                lightning_net_tokio::setup_inbound(
                    peer_mgr.clone(),
                    tcp_stream.into_std().unwrap(),
                )
                .await;
            });
        }
    });
}
