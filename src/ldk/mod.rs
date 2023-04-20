use bitcoin::blockdata::constants::genesis_block;
use bitcoin::consensus::encode;
use bitcoin::secp256k1::Secp256k1;
use bitcoin_bech32::WitnessProgram;
use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::chain::chaininterface::ConfirmationTarget;
use lightning::chain::chaininterface::FeeEstimator;
use lightning::ln::{PaymentHash, PaymentPreimage, PaymentSecret};
use lightning::routing::router::DefaultRouter;
use lightning::routing::scoring::ProbabilisticScorer;
use lightning::routing::scoring::ProbabilisticScoringParameters;
use lightning::util::events::{Event, PaymentPurpose};
use lightning_invoice::payment;
use lightning_invoice::payment::InvoicePayer;
use lightning_net_tokio;
use lightning_net_tokio::SocketDescriptor;
use rand::{thread_rng, Rng};

use bitcoin::network::constants::Network;
use lightning::chain::keysinterface::{InMemorySigner, KeysInterface, KeysManager, Recipient};
use lightning::chain::{self, chainmonitor, ChannelMonitorUpdateStatus, Filter, Watch};
use lightning::ln::channelmanager::{
    ChainParameters, ChannelManagerReadArgs, SimpleArcChannelManager,
};

use lightning::ln::peer_handler::{IgnoringMessageHandler, MessageHandler, SimpleArcPeerManager};
use lightning::onion_message::OnionMessenger;
use lightning::routing::gossip::{NodeId, P2PGossipSync};
use lightning::util::config::UserConfig;
use lightning::util::ser::ReadableArgs;
use lightning_background_processor::{BackgroundProcessor, GossipSync};
use lightning_block_sync::poll;
use lightning_block_sync::UnboundedCache;
use lightning_block_sync::{init, SpvClient};
use lightning_persister::FilesystemPersister;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
use std::io::{self, BufReader, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

pub mod block_sync;
// pub mod broadcast;
use bitcoin::{BlockHash, Transaction};

use self::disk::FilesystemLogger;
// pub mod chain_monitor;
pub mod core;
pub mod disk;
pub mod hex_utils;
pub mod keys_manager;
pub mod logger;
pub mod persister;
// use rand::Rng;

type NetworkGraph = lightning::routing::gossip::NetworkGraph<Arc<FilesystemLogger>>;

async fn handle_ldk_events(
    channel_manager: &Arc<ChannelManager>,
    bitcoind_client: &core::CoreLDK,
    network_graph: &NetworkGraph,
    keys_manager: &KeysManager,
    inbound_payments: &PaymentInfoStorage,
    outbound_payments: &PaymentInfoStorage,
    network: Network,
    event: &Event,
) {
    match event {
        Event::FundingGenerationReady {
            temporary_channel_id,
            counterparty_node_id,
            channel_value_satoshis,
            output_script,
            ..
        } => {
            // Construct the raw transaction with one output, that is paid the amount of the
            // channel.
            let addr = WitnessProgram::from_scriptpubkey(
                &output_script[..],
                match network {
                    Network::Bitcoin => bitcoin_bech32::constants::Network::Bitcoin,
                    Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
                    Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
                    Network::Signet => bitcoin_bech32::constants::Network::Signet,
                },
            )
            .expect("Lightning funding tx should always be to a SegWit output")
            .to_address();
            let mut outputs = vec![HashMap::with_capacity(1)];
            outputs[0].insert(addr, *channel_value_satoshis as f64 / 100_000_000.0);
            let raw_tx = bitcoind_client.create_raw_transaction(outputs).await;

            // Have your wallet put the inputs into the transaction such that the output is
            // satisfied.
            let funded_tx = bitcoind_client.fund_raw_transaction(raw_tx).await;

            // Sign the final funding transaction and broadcast it.
            let signed_tx = bitcoind_client
                .sign_raw_transaction_with_wallet(funded_tx.hex)
                .await;
            assert_eq!(signed_tx.complete, true);
            let final_tx: Transaction =
                encode::deserialize(&hex_utils::to_vec(&signed_tx.hex).unwrap()).unwrap();
            // Give the funding transaction back to LDK for opening the channel.
            if channel_manager
                .funding_transaction_generated(
                    &temporary_channel_id,
                    counterparty_node_id,
                    final_tx,
                )
                .is_err()
            {
                println!(
					"\nERROR: Channel went away before we could fund it. The peer disconnected or refused the channel.");
                print!("> ");
                io::stdout().flush().unwrap();
            }
        }
        Event::PaymentClaimable {
            payment_hash,
            purpose,
            amount_msat,
            receiver_node_id: _,
            via_channel_id: _,
            via_user_channel_id: _,
        } => {
            println!(
                "\nEVENT: received payment from payment hash {} of {} millisatoshis",
                hex_utils::hex_str(&payment_hash.0),
                amount_msat,
            );
            print!("> ");
            io::stdout().flush().unwrap();
            let payment_preimage = match purpose {
                PaymentPurpose::InvoicePayment {
                    payment_preimage, ..
                } => *payment_preimage,
                PaymentPurpose::SpontaneousPayment(preimage) => Some(*preimage),
            };
            channel_manager.claim_funds(payment_preimage.unwrap());
        }
        Event::PaymentClaimed {
            payment_hash,
            purpose,
            amount_msat,
            receiver_node_id: _,
        } => {
            println!(
                "\nEVENT: claimed payment from payment hash {} of {} millisatoshis",
                hex_utils::hex_str(&payment_hash.0),
                amount_msat,
            );
            print!("> ");
            io::stdout().flush().unwrap();
            let (payment_preimage, payment_secret) = match purpose {
                PaymentPurpose::InvoicePayment {
                    payment_preimage,
                    payment_secret,
                    ..
                } => (*payment_preimage, Some(*payment_secret)),
                PaymentPurpose::SpontaneousPayment(preimage) => (Some(*preimage), None),
            };
            let mut payments = inbound_payments.lock().unwrap();
            match payments.entry(*payment_hash) {
                Entry::Occupied(mut e) => {
                    let payment = e.get_mut();
                    payment.status = HTLCStatus::Succeeded;
                    payment.preimage = payment_preimage;
                    payment.secret = payment_secret;
                }
                Entry::Vacant(e) => {
                    e.insert(PaymentInfo {
                        preimage: payment_preimage,
                        secret: payment_secret,
                        status: HTLCStatus::Succeeded,
                        amt_msat: MillisatAmount(Some(*amount_msat)),
                    });
                }
            }
        }
        Event::PaymentSent {
            payment_preimage,
            payment_hash,
            fee_paid_msat,
            ..
        } => {
            let mut payments = outbound_payments.lock().unwrap();
            for (hash, payment) in payments.iter_mut() {
                if *hash == *payment_hash {
                    payment.preimage = Some(*payment_preimage);
                    payment.status = HTLCStatus::Succeeded;
                    println!(
                        "\nEVENT: successfully sent payment of {:#?} millisatoshis{:#?} from \
								 payment hash {:#?} with preimage {:#?}",
                        payment.amt_msat,
                        if let Some(fee) = fee_paid_msat {
                            format!(" (fee {:#?} msat)", fee)
                        } else {
                            "".to_string()
                        },
                        hex_utils::hex_str(&payment_hash.0),
                        hex_utils::hex_str(&payment_preimage.0)
                    );
                    print!("> ");
                    io::stdout().flush().unwrap();
                }
            }
        }
        Event::OpenChannelRequest { .. } => {
            // Unreachable, we don't set manually_accept_inbound_channels
        }
        Event::PaymentPathSuccessful { .. } => {}
        Event::PaymentPathFailed { .. } => {}
        Event::ProbeSuccessful { .. } => {}
        Event::ProbeFailed { .. } => {}
        Event::PaymentFailed { payment_hash, .. } => {
            print!(
				"\nEVENT: Failed to send payment to payment hash {:?}: exhausted payment retry attempts",
				hex_utils::hex_str(&payment_hash.0)
			);
            print!("> ");
            io::stdout().flush().unwrap();

            let mut payments = outbound_payments.lock().unwrap();
            if payments.contains_key(&payment_hash) {
                let payment = payments.get_mut(&payment_hash).unwrap();
                payment.status = HTLCStatus::Failed;
            }
        }
        Event::PaymentForwarded {
            prev_channel_id,
            next_channel_id,
            fee_earned_msat,
            claim_from_onchain_tx,
        } => {
            let read_only_network_graph = network_graph.read_only();
            let nodes = read_only_network_graph.nodes();
            let channels = channel_manager.list_channels();

            let node_str = |channel_id: &Option<[u8; 32]>| match channel_id {
                None => String::new(),
                Some(channel_id) => match channels.iter().find(|c| c.channel_id == *channel_id) {
                    None => String::new(),
                    Some(channel) => {
                        match nodes.get(&NodeId::from_pubkey(&channel.counterparty.node_id)) {
                            None => "private node".to_string(),
                            Some(node) => match &node.announcement_info {
                                None => "unnamed node".to_string(),
                                Some(announcement) => {
                                    format!("node {}", announcement.alias)
                                }
                            },
                        }
                    }
                },
            };
            let channel_str = |channel_id: &Option<[u8; 32]>| {
                channel_id
                    .map(|channel_id| format!(" with channel {}", hex_utils::hex_str(&channel_id)))
                    .unwrap_or_default()
            };
            let from_prev_str = format!(
                " from {}{}",
                node_str(prev_channel_id),
                channel_str(prev_channel_id)
            );
            let to_next_str = format!(
                " to {}{}",
                node_str(next_channel_id),
                channel_str(next_channel_id)
            );

            let from_onchain_str = if *claim_from_onchain_tx {
                "from onchain downstream claim"
            } else {
                "from HTLC fulfill message"
            };
            if let Some(fee_earned) = fee_earned_msat {
                println!(
                    "\nEVENT: Forwarded payment{}{}, earning {} msat {}",
                    from_prev_str, to_next_str, fee_earned, from_onchain_str
                );
            } else {
                println!(
                    "\nEVENT: Forwarded payment{}{}, claiming onchain {}",
                    from_prev_str, to_next_str, from_onchain_str
                );
            }
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::HTLCHandlingFailed { .. } => {}
        Event::PendingHTLCsForwardable { time_forwardable } => {
            let forwarding_channel_manager = channel_manager.clone();
            let min = time_forwardable.as_millis() as u64;
            tokio::spawn(async move {
                let millis_to_sleep = thread_rng().gen_range(min, min * 5) as u64;
                tokio::time::sleep(Duration::from_millis(millis_to_sleep)).await;
                forwarding_channel_manager.process_pending_htlc_forwards();
            });
        }
        Event::SpendableOutputs { outputs } => {
            let destination_address = bitcoind_client.get_new_address().await;
            let output_descriptors = &outputs.iter().map(|a| a).collect::<Vec<_>>();
            let tx_feerate =
                bitcoind_client.get_est_sat_per_1000_weight(ConfirmationTarget::Normal);
            let spending_tx = keys_manager
                .spend_spendable_outputs(
                    output_descriptors,
                    Vec::new(),
                    destination_address.script_pubkey(),
                    tx_feerate,
                    &Secp256k1::new(),
                )
                .unwrap();
            bitcoind_client.broadcast_transaction(&spending_tx);
        }
        Event::ChannelReady {
            ref channel_id,
            user_channel_id: _,
            ref counterparty_node_id,
            channel_type: _,
        } => {
            println!(
                "\nEVENT: Channel {} with peer {} is ready to be used!",
                hex_utils::hex_str(channel_id),
                hex_utils::hex_str(&counterparty_node_id.serialize()),
            );
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::ChannelClosed {
            channel_id,
            reason,
            user_channel_id: _,
        } => {
            println!(
                "\nEVENT: Channel {} closed due to: {:?}",
                hex_utils::hex_str(channel_id),
                reason
            );
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::DiscardFunding { .. } => {
            // A "real" node should probably "lock" the UTXOs spent in funding transactions until
            // the funding transaction either confirms, or this event is generated.
        }
        Event::HTLCIntercepted { .. } => {}
    }
}

enum HTLCStatus {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Debug)]
struct MillisatAmount(Option<u64>);

pub(crate) struct PaymentInfo {
    preimage: Option<PaymentPreimage>,
    secret: Option<PaymentSecret>,
    status: HTLCStatus,
    amt_msat: MillisatAmount,
}

type PaymentInfoStorage = Arc<Mutex<HashMap<PaymentHash, PaymentInfo>>>;

pub type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<core::CoreLDK>,
    Arc<core::CoreLDK>,
    Arc<FilesystemLogger>,
    Arc<FilesystemPersister>,
>;

pub(crate) type ChannelManager = SimpleArcChannelManager<
    ChainMonitor,
    core::CoreLDK, // broadcast
    core::CoreLDK, // fee estimate
    FilesystemLogger,
>;

type PeerManager = SimpleArcPeerManager<
    SocketDescriptor,
    ChainMonitor,
    core::CoreLDK, // broadcast
    core::CoreLDK, //fee estimate
    dyn chain::Access + Send + Sync,
    FilesystemLogger,
>;

fn read_network(
    path: &Path,
    genesis_hash: BlockHash,
    logger: Arc<FilesystemLogger>,
) -> NetworkGraph {
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

    let logger = Arc::new(FilesystemLogger::new(ldk_data_dir.clone()));
    // let logger = Arc::new(FilesystemLogger {});
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

    // Handle Events
    let channel_manager_event_listener = channel_manager.clone();
    let keys_manager_listener = keys_manager.clone();
    let inbound_payments: PaymentInfoStorage = Arc::new(Mutex::new(HashMap::new()));
    let outbound_payments: PaymentInfoStorage = Arc::new(Mutex::new(HashMap::new()));
    let inbound_pmts_for_events = inbound_payments.clone();
    let outbount_pmts_for_events = outbound_payments.clone();
    let bitcoind_rpc = core_ldk.clone();
    let network_graph_events = network_graph.clone();
    let handle = tokio::runtime::Handle::current();
    let event_handler = move |event: Event| {
        handle.block_on(handle_ldk_events(
            &channel_manager_event_listener,
            &bitcoind_rpc,
            &network_graph_events,
            &keys_manager_listener,
            &inbound_pmts_for_events,
            &outbount_pmts_for_events,
            Network::Regtest,
            &event,
        ));
    };

    // Step 16. Initialize the ProbabilisticScorer
    let scorer_path = format!("{}/scorer", ldk_data_dir.clone());
    let params = ProbabilisticScoringParameters::default();
    let scorer = Arc::new(Mutex::new(ProbabilisticScorer::new(
        params,
        Arc::clone(&network_graph),
        Arc::clone(&logger),
    )));

    // Step 17. Initialize the InvoicePayer
    let router = DefaultRouter::new(
        Arc::clone(&network_graph),
        Arc::clone(&logger),
        keys_manager.get_secure_random_bytes(),
        Arc::clone(&scorer),
    );

    let invoice_payer = Arc::new(InvoicePayer::new(
        Arc::clone(&channel_manager),
        router,
        Arc::clone(&logger),
        event_handler,
        payment::Retry::Attempts(5),
    ));

    // Step 18. Initialize the Persister
    let persister = Arc::new(FilesystemPersister::new(ldk_data_dir.clone()));

    // Step 19. Start Background Processing
    let background_processor = BackgroundProcessor::start(
        persister,
        invoice_payer.clone(),
        Arc::clone(&chain_monitor),
        Arc::clone(&channel_manager),
        GossipSync::P2P(gossip_sync.clone()),
        Arc::clone(&peer_manager),
        Arc::clone(&logger),
        Some(scorer.clone()),
    );
}
