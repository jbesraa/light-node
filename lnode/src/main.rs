use crate::types::{ChainMonitor, ChannelManager, OnionMessenger, PaymentInfoStorage, PeerManager};
use actix_web::web::Data;
use actix_web::{App, HttpServer};
use bitcoin::blockdata::constants::genesis_block;
use bitcoin::network::constants::Network;
use bitcoin::BlockHash;
use http_server::routes::{
    blockchain_info, lightning_node_info, lightning_peers_connect, lightning_peers_list, wallet_info,
};
use http_server::state::HttpServerState;
use ldk::core::CoreLDK;
use ldk::event_handler::handle_ldk_events;
use lightning::chain::keysinterface::EntropySource;
use lightning::chain::{self, chainmonitor, ChannelMonitorUpdateStatus, Watch};
use lightning::events::Event;
use lightning::ln::channelmanager::{self, ChainParameters, ChannelManagerReadArgs};
use lightning::ln::peer_handler::{IgnoringMessageHandler, MessageHandler};
use lightning::routing::gossip::P2PGossipSync;
use lightning::routing::router::DefaultRouter;
use lightning::routing::scoring::ProbabilisticScorer;
use lightning::routing::scoring::ProbabilisticScoringParameters;
use lightning::util::config::UserConfig;
use lightning::util::ser::ReadableArgs;
use lightning_background_processor::{process_events_async, GossipSync};
use lightning_block_sync::poll;
use lightning_block_sync::UnboundedCache;
use lightning_block_sync::{init, SpvClient};
use lightning_net_tokio;
use lightning_persister::FilesystemPersister;
use rand::Rng;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use utils::disk::FilesystemLogger;
use utils::hex::{ipv_addr, str_to_u8};
use utils::{disk, read_network, sweep};

pub mod cli;
pub mod http_server;
pub mod ldk;
pub mod types;
pub mod utils;
pub mod wallet;

pub async fn start_node() {
    let ldk_data_dir = format!("{}/.ldk", ".");
    let port = 9735;
    let network = Network::Regtest;
    let node_name = "nodenamehjo";
    let announced_listen_addr = "46.116.222.94";
    let n_core_ldk: CoreLDK = match CoreLDK::new().await {
        Ok(client) => client,
        Err(e) => {
            println!("FAILED TO START CORELDK: {}", e);
            return;
        }
    };
    let core_ldk = Arc::new(n_core_ldk.clone());
    let fee_estimator = core_ldk.clone();

    let logger = Arc::new(FilesystemLogger::new(ldk_data_dir.clone()));
    // let logger = Arc::new(FilesystemLogger {});
    let broadcaster_interface = core_ldk.clone(); // 3
    let keys_manager = Arc::new(ldk::keys_manager::new(&ldk_data_dir)); // 6
    let persister = Arc::new(ldk::persister::persister(&ldk_data_dir)); // 4

    let chain_monitor: Arc<ChainMonitor> = Arc::new(chainmonitor::ChainMonitor::new(
        None,
        broadcaster_interface.clone(),
        logger.clone(),
        fee_estimator.clone(),
        persister.clone(),
    ));

    // Start step 7: Read ChannelMonitor state from Disk
    let mut channel_monitors = persister
        .read_channelmonitors(keys_manager.clone(), keys_manager.clone())
        .unwrap();
    // End step 7

    // Start step 8
    let user_config = UserConfig::default();

    let genesis = genesis_block(Network::Regtest).header.block_hash();
    let network_graph_path = format!("{}/network_graph", ldk_data_dir.clone());
    let network_graph = Arc::new(read_network(
        Path::new(&network_graph_path),
        genesis,
        logger.clone(),
    ));

    let gossip_sync = Arc::new(P2PGossipSync::new(
        Arc::clone(&network_graph),
        None::<Arc<CoreLDK>>,
        logger.clone(),
    ));

    // Step 16. Initialize the ProbabilisticScorer
    let scorer_path = format!("{}/scorer", ldk_data_dir.clone());
    let params = ProbabilisticScoringParameters::default();
    let scorer = Arc::new(Mutex::new(ProbabilisticScorer::new(
        params,
        Arc::clone(&network_graph),
        Arc::clone(&logger),
    )));

    let router = Arc::new(DefaultRouter::new(
        Arc::clone(&network_graph),
        Arc::clone(&logger),
        keys_manager.get_secure_random_bytes(),
        Arc::clone(&scorer),
    ));
    /* RESTARTING */
    let mut restarting_node = false;
    let polled_chain_tip = init::validate_best_block_header(core_ldk.as_ref())
        .await
        .expect("Failed to fetch best block header and best block");

    let (channel_manager_blockhash, channel_manager) = {
        if let Ok(mut f) = fs::File::open(format!("{}/manager", ldk_data_dir.clone())) {
            let mut channel_monitor_mut_references = Vec::new();
            for (_, channel_monitor) in channel_monitors.iter_mut() {
                channel_monitor_mut_references.push(channel_monitor);
            }
            let read_args = ChannelManagerReadArgs::new(
                keys_manager.clone(),
                keys_manager.clone(),
                keys_manager.clone(),
                fee_estimator.clone(),
                chain_monitor.clone(),
                broadcaster_interface.clone(),
                router.clone(),
                logger.clone(),
                user_config,
                channel_monitor_mut_references,
            );
            <(BlockHash, ChannelManager)>::read(&mut f, read_args).unwrap()
        } else {
            // We're starting a fresh node.
            restarting_node = false;

            let polled_best_block = polled_chain_tip.to_best_block();
            let polled_best_block_hash = polled_best_block.block_hash();
            let chain_params = ChainParameters {
                network: Network::Regtest,
                best_block: polled_best_block,
            };
            let fresh_channel_manager = channelmanager::ChannelManager::new(
                fee_estimator.clone(),
                chain_monitor.clone(),
                broadcaster_interface.clone(),
                router.clone(),
                logger.clone(),
                keys_manager.clone(),
                keys_manager.clone(),
                keys_manager.clone(),
                user_config,
                chain_params,
            );
            (polled_best_block_hash, fresh_channel_manager)
        }
    };

    /* FRESH CHANNELMANAGER */

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
            router.clone(),
            logger.clone(),
            keys_manager.clone(),
            keys_manager.clone(),
            keys_manager.clone(),
            user_config,
            chain_params,
        );
        (best_block_hash, fresh_channel_manager)
    };
    // End step 8
    let mut chain_listener_channel_monitors = Vec::new();
    let mut cache = UnboundedCache::new();
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
            core_ldk.clone().as_ref(),
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

    //

    let mut ephemeral_bytes = [0; 32];
    rand::thread_rng().fill_bytes(&mut ephemeral_bytes);
    let onion_messenger = Arc::new(OnionMessenger::new(
        Arc::clone(&keys_manager),
        Arc::clone(&keys_manager),
        Arc::clone(&logger),
        IgnoringMessageHandler {},
    ));
    let channel_manager: Arc<ChannelManager> = Arc::new(channel_manager);
    let lightning_msg_handler = MessageHandler {
        chan_handler: channel_manager.clone(),
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
        current_time.try_into().unwrap(),
        &ephemeral_bytes,
        logger.clone(),
        ignoring_custom_msg_handler,
        Arc::clone(&keys_manager),
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

    // block data
    let channel_manager_listener = channel_manager.clone();
    let chain_monitor_listener = chain_monitor.clone();
    let bitcoind_block_source = core_ldk.clone();
    tokio::spawn(async move {
        let chain_poller = poll::ChainPoller::new(bitcoind_block_source.as_ref(), Network::Regtest);
        let chain_listener = (chain_monitor_listener, channel_manager_listener);
        let mut spv_client = SpvClient::new(
            chain_tip.unwrap(),
            chain_poller,
            &mut cache,
            &chain_listener,
        );
        loop {
            spv_client.poll_best_tip().await.unwrap();
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    });
    let inbound_payments: PaymentInfoStorage = Arc::new(Mutex::new(HashMap::new()));
    let outbound_payments: PaymentInfoStorage = Arc::new(Mutex::new(HashMap::new()));
    let bdk_wallet = Arc::new(wallet::BitcoinWallet::new_wallet());
    // Step 18: Handle LDK Events
    let arc_bdk_wallet = Arc::clone(&bdk_wallet);
    let channel_manager_event_listener = Arc::clone(&channel_manager);
    let bitcoind_client_event_listener = Arc::clone(&core_ldk);
    let network_graph_event_listener = Arc::clone(&network_graph);
    let keys_manager_event_listener = Arc::clone(&keys_manager);
    let inbound_payments_event_listener = Arc::clone(&inbound_payments);
    let outbound_payments_event_listener = Arc::clone(&outbound_payments);
    let persister_event_listener = Arc::clone(&persister);

    // Handle Events
    let event_handler = move |event: Event| {
        let channel_manager_event_listener = Arc::clone(&channel_manager_event_listener);
        let bitcoind_client_event_listener = Arc::clone(&bitcoind_client_event_listener);
        let bitcoind_client_event_listener_1 = Arc::clone(&arc_bdk_wallet);
        let network_graph_event_listener = Arc::clone(&network_graph_event_listener);
        let keys_manager_event_listener = Arc::clone(&keys_manager_event_listener);
        let inbound_payments_event_listener = Arc::clone(&inbound_payments_event_listener);
        let outbound_payments_event_listener = Arc::clone(&outbound_payments_event_listener);
        let persister_event_listener = Arc::clone(&persister_event_listener);

        async move {
            handle_ldk_events(
                &channel_manager_event_listener,
                &bitcoind_client_event_listener,
                &network_graph_event_listener,
                &keys_manager_event_listener,
                &inbound_payments_event_listener,
                &outbound_payments_event_listener,
                &persister_event_listener,
                network,
                event,
            )
            .await;
        }
    };

    // Step 18. Initialize the Persister
    let persister = Arc::new(FilesystemPersister::new(ldk_data_dir.clone()));

    // Step 19. Start Background Processing
    let (bp_exit, bp_exit_check) = tokio::sync::watch::channel(());
    let background_processor = tokio::spawn(process_events_async(
        Arc::clone(&persister),
        event_handler,
        chain_monitor.clone(),
        channel_manager.clone(),
        GossipSync::p2p(gossip_sync.clone()),
        peer_manager.clone(),
        logger.clone(),
        Some(scorer.clone()),
        move |t| {
            let mut bp_exit_fut_check = bp_exit_check.clone();
            Box::pin(async move {
                tokio::select! {
                    _ = tokio::time::sleep(t) => false,
                    _ = bp_exit_fut_check.changed() => true,
                }
            })
        },
        false,
    ));

    // Regularly reconnect to channel peers.
    let connect_cm = Arc::clone(&channel_manager);
    let connect_pm = Arc::clone(&peer_manager);
    let peer_data_path = format!("{}/channel_peer_data", ldk_data_dir.clone());
    let stop_connect = Arc::clone(&stop_listen_connect);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            match disk::read_channel_peer_data(Path::new(&peer_data_path)) {
                Ok(info) => {
                    let peers = connect_pm.get_peer_node_ids();
                    for node_id in connect_cm
                        .list_channels()
                        .iter()
                        .map(|chan| chan.counterparty.node_id)
                        .filter(|id| !peers.iter().any(|(pk, _)| id == pk))
                    {
                        if stop_connect.load(Ordering::Acquire) {
                            return;
                        }
                        for (pubkey, peer_addr) in info.iter() {
                            if *pubkey == node_id {
                                let _ = cli::do_connect_peer(
                                    *pubkey,
                                    peer_addr.clone(),
                                    Arc::clone(&connect_pm),
                                )
                                .await;
                            }
                        }
                    }
                }
                Err(e) => println!(
                    "ERROR: errored reading channel peer info from disk: {:?}",
                    e
                ),
            }
        }
    });

    // Regularly broadcast our node_announcement. This is only required (or possible) if we have
    // some public channels.
    let peer_man = Arc::clone(&peer_manager);
    let chan_man = Arc::clone(&channel_manager);
    tokio::spawn(async move {
        // First wait a minute until we have some peers and maybe have opened a channel.
        tokio::time::sleep(Duration::from_secs(60)).await;
        // Then, update our announcement once an hour to keep it fresh but avoid unnecessary churn
        // in the global gossip network.
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            // Don't bother trying to announce if we don't have any public channls, though our
            // peers should drop such an announcement anyway. Note that announcement may not
            // propagate until we have a channel with 6+ confirmations.
            if chan_man.list_channels().iter().any(|chan| chan.is_public) {
                peer_man.broadcast_node_announcement(
                    [0; 3],
                    str_to_u8(node_name),
                    ipv_addr(announced_listen_addr, port),
                );
            }
        }
    });

    tokio::spawn(sweep::periodic_sweep(
        ldk_data_dir.clone(),
        Arc::clone(&keys_manager),
        Arc::clone(&logger),
        Arc::clone(&persister),
        Arc::clone(&core_ldk),
    ));
    // Disconnect our peers and stop accepting new connections. This ensures we don't continue
    // updating our channel data after we've stopped the background processor.
    stop_listen_connect.store(true, Ordering::Release);
    peer_manager.disconnect_all_peers();

    // Stop the background processor.
    bp_exit.send(()).unwrap();
    background_processor.await.unwrap().unwrap();

    // let listener = TcpListener::bind("127.0.0.1:8181").await.unwrap();
    let httpdata = Data::new(Mutex::new(HttpServerState {
        peer_manager: peer_manager.clone(),
        keys_manager: keys_manager.clone(),
        logger: logger.clone(),
        inbound_payments: inbound_payments.clone(),
        outbound_payments: outbound_payments.clone(),
        onion_messenger: onion_messenger.clone(),
        network_graph: network_graph.clone(),
        channel_manager: channel_manager.clone(),
        network: network.clone(),
        port: port.clone(),
        ldk_data_dir: ldk_data_dir.clone(),
        announced_listen_addr: announced_listen_addr.to_string(),
        node_name: node_name.to_string(),
    }));

    let my_wall = Data::new(Mutex::new(bdk_wallet.clone()));
    let state_ldk = Data::new(Mutex::new(n_core_ldk));
    let _httpres = HttpServer::new(move || {
        App::new()
            .app_data(Data::clone(&my_wall))
            .app_data(Data::clone(&httpdata))
            .app_data(Data::clone(&state_ldk))
            .service(lightning_node_info)
            .service(blockchain_info)
            .service(lightning_peers_connect)
            .service(lightning_peers_list)
            .service(wallet_info)
    })
    .bind(("127.0.0.1", 8181))
    .unwrap()
    .run()
    .await;
}

#[tokio::main]
pub async fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    #[cfg(not(target_os = "windows"))]
    {
        // Catch Ctrl-C with a dummy signal handler.
        unsafe {
            let mut new_action: libc::sigaction = core::mem::zeroed();
            let mut old_action: libc::sigaction = core::mem::zeroed();

            extern "C" fn dummy_handler(
                _: libc::c_int,
                _: *const libc::siginfo_t,
                _: *const libc::c_void,
            ) {
            }

            new_action.sa_sigaction = dummy_handler as libc::sighandler_t;
            new_action.sa_flags = libc::SA_SIGINFO;

            libc::sigaction(
                libc::SIGINT,
                &new_action as *const libc::sigaction,
                &mut old_action as *mut libc::sigaction,
            );
        }
    }

    start_node().await;
}
