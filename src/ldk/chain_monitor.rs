use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::{
    chain::{chaininterface::FeeEstimator, chainmonitor, keysinterface::InMemorySigner, Filter},
    util::logger::Logger,
};
use lightning_persister::FilesystemPersister;
use std::sync::Arc;

pub type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<BroadcasterInterface>,
    Arc<FeeEstimator>,
    Arc<Logger>,
    Arc<FilesystemPersister>,
>;

type Type = ChainMonitor;

impl Type {
    pub fn new(
        broadcaster_interface: Arc<dyn BroadcasterInterface>,
        logger: Arc<dyn Logger>,
        fee_estimator: Arc<dyn FeeEstimator>,
        persister: Arc<FilesystemPersister>,
    ) -> ChainMonitor {
        let monitor: ChainMonitor = chainmonitor::ChainMonitor::new(
            None,
            broadcaster_interface.clone(),
            logger.clone(),
            fee_estimator.clone(),
            persister.clone(),
        );

        monitor
    }
}
