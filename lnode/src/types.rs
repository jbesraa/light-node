use crate::ldk::core::CoreLDK;
use crate::utils::disk::FilesystemLogger;
use lightning::chain::keysinterface::InMemorySigner;
use lightning::chain::{chainmonitor, Filter};
use lightning::ln::channelmanager::SimpleArcChannelManager;
use lightning::ln::peer_handler::SimpleArcPeerManager;
use lightning::ln::{PaymentHash, PaymentPreimage, PaymentSecret};
use lightning::onion_message::SimpleArcOnionMessenger;
use lightning::routing::gossip;
use lightning_net_tokio::SocketDescriptor;
use lightning_persister::FilesystemPersister;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type NetworkGraph = gossip::NetworkGraph<Arc<FilesystemLogger>>;

pub enum HTLCStatus {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Debug)]
pub struct MillisatAmount(pub Option<u64>);

impl std::fmt::Display for MillisatAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(msat) => write!(f, "{}", msat),
            None => write!(f, "N/A"),
        }
    }
}

pub struct PaymentInfo {
    pub preimage: Option<PaymentPreimage>,
    pub secret: Option<PaymentSecret>,
    pub status: HTLCStatus,
    pub amt_msat: MillisatAmount,
}

pub type OnionMessenger = SimpleArcOnionMessenger<FilesystemLogger>;

pub type PaymentInfoStorage = Arc<Mutex<HashMap<PaymentHash, PaymentInfo>>>;

pub type ChainMonitor = chainmonitor::ChainMonitor<
    InMemorySigner,
    Arc<dyn Filter + Send + Sync>,
    Arc<CoreLDK>,
    Arc<CoreLDK>,
    Arc<FilesystemLogger>,
    Arc<FilesystemPersister>,
>;

pub(crate) type ChannelManager = SimpleArcChannelManager<
    ChainMonitor,
    CoreLDK, // broadcast
    CoreLDK, // fee estimate
    FilesystemLogger,
>;

pub type PeerManager = SimpleArcPeerManager<
    SocketDescriptor,
    ChainMonitor,
    CoreLDK,
    CoreLDK,
    CoreLDK,
    FilesystemLogger,
>;
