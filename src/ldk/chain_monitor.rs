use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::{
    chain::{chaininterface::FeeEstimator, chainmonitor, keysinterface::InMemorySigner, Filter},
    util::logger::Logger,
};
use lightning_persister::FilesystemPersister;
use std::sync::Arc;

type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<BroadcasterInterface>,
    Arc<FeeEstimator>,
    Arc<Logger>,
    Arc<FilesystemPersister>,
>;

pub async fn new(
    broadcaster_interface: Arc<dyn BroadcasterInterface>,
    logger: Arc<dyn Logger>,
    fee_estimator: Arc<dyn FeeEstimator>,
    persister: Arc<FilesystemPersister>,
) -> Arc<ChainMonitor> {
    let monitor: Arc<ChainMonitor> = Arc::new(chainmonitor::ChainMonitor::new(
        None,
        broadcaster_interface.clone(),
        logger.clone(),
        fee_estimator.clone(),
        persister.clone(),
    ));

    monitor
}
