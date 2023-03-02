use std::sync::Arc;

pub mod broadcast;
pub mod chain_monitor;
pub mod fee_estimator;
pub mod logger;
pub mod persister;

pub async fn start_node() {
    let ldk_data_dir = format!("{}/.ldk", ".");
    let fee_estimator = Arc::new(fee_estimator::MyFeeEstimator::default()); // 1
    let logger = Arc::new(logger::MyLogger()); // 2
    let broadcaster_interface = Arc::new(broadcast::MyBroadcastInterface::new().await.unwrap()); // 3
    let persister = Arc::new(persister::persister(ldk_data_dir)); // 4
    let chain_monitor = chain_monitor::new(broadcaster_interface, logger, fee_estimator, persister);
    // let chain_monitor: Arc<ChainMonitor> = Arc::new()
    // let chain_monitor: Arc<ChainMonitor> = Arc::new(chainmonitor::ChainMonitor::new(
    //     filter.clone(),
    //     broadcaster_interface.clone(),
    //     logger.clone(),
    //     fee_estimator.clone(),
    //     persister.clone(),
    // ));
}
