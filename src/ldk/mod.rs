use lightning_block_sync::init;
use bitcoin::network::constants::Network;
use lightning::ln::channelmanager::{
    ChainParameters, ChannelManagerReadArgs, SimpleArcChannelManager,
};
use lightning::util::config::UserConfig;
use lightning::util::ser::ReadableArgs;
use std::fs::File;
use std::sync::Arc;

pub mod block_sync;
pub mod broadcast;
use bitcoin::BlockHash;
pub mod chain_monitor;
pub mod fee_estimator;
pub mod keys_manager;
pub mod logger;
pub mod persister;

pub(crate) type ChannelManager = SimpleArcChannelManager<
    chain_monitor::ChainMonitor,
    broadcast::MyBroadcastInterface,
    fee_estimator::MyFeeEstimator,
    logger::MyLogger,
>;

pub async fn start_node() {
    let ldk_data_dir = format!("{}/.ldk", ".");
    let fee_estimator = Arc::new(fee_estimator::MyFeeEstimator::default()); // 1
    let logger = Arc::new(logger::MyLogger()); // 2
    let broadcaster_interface = Arc::new(broadcast::MyBroadcastInterface::new().await.unwrap()); // 3
    let keys_manager = Arc::new(keys_manager::new(&ldk_data_dir)); // 6
    let persister = Arc::new(persister::persister(&ldk_data_dir)); // 4
    let chain_monitor: Arc<chain_monitor::ChainMonitor> = Arc::new(chain_monitor::new(
        broadcaster_interface.clone(),
        logger.clone(),
        fee_estimator.clone(),
        persister.clone(),
    )); //5

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

    let block_source = block_sync::BitcoindClientLight::new().await.unwrap();
    let polled_chain_tip = init::validate_best_block_header(&block_source)
        .await
        .expect("Failed to fetch best block header and best block");
    let best_block = polled_chain_tip.to_best_block();
    let best_block_hash = best_block.block_hash();

    let (channel_manager_blockhash, mut channel_manager) = {
        // let best_blockhash = // insert the best blockhash you know of
        // let best_chain_height = // insert the height corresponding to best_blockhash
        let chain_params = ChainParameters {
            network: Network::Testnet, // substitute this with your network
            best_block: best_block
        };
        let fresh_channel_manager = ChannelManager::new(
            fee_estimator,
            chain_monitor,
            broadcaster_interface,
            logger,
            keys_manager,
            user_config,
            chain_params,
        );
        (best_block_hash, fresh_channel_manager)
    };
    // End step 8
}
