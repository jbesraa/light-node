use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::{
    chain::{chaininterface::FeeEstimator, chainmonitor, keysinterface::InMemorySigner, Filter},
    util::logger::Logger,
};
use lightning_persister::FilesystemPersister;
use std::sync::Arc;

pub mod broadcast;
pub mod fee_estimator;
pub mod logger;
pub mod persister;

type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<dyn BroadcasterInterface>,
    Arc<dyn Logger>,
    Arc<dyn FeeEstimator>,
    Arc<FilesystemPersister>,
>;

pub async fn start_node() {
    let ldk_data_dir = format!("{}/.ldk", ".");
    let fee_estimator = fee_estimator::MyFeeEstimator::default(); // 1
    let logger = logger::MyLogger(); // 2
    let broadcaster_interface = broadcast::MyBroadcastInterface::new().await.unwrap(); // 3
    let persister = persister::persister(ldk_data_dir); // 4
    let filter: Option<Box<dyn Filter>> = None;
    let chain_monitor: Arc<ChainMonitor> = Arc::new(chainmonitor::ChainMonitor::new(
        Arc::new(filter),
        Arc::new(broadcaster_interface),
        Arc::( logger ),
        Arc::new(fee_estimator),
        Arc::new(persister),
    ));
}
