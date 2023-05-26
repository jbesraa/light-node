use crate::{
    types::{ChannelManager, NetworkGraph, OnionMessenger, PaymentInfoStorage, PeerManager},
    utils::disk,
};
use bitcoin::Network;
use lightning::chain::keysinterface::KeysManager;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub struct HttpServerState {
    pub peer_manager: Arc<PeerManager>,
    pub channel_manager: Arc<ChannelManager>,
    pub keys_manager: Arc<KeysManager>,
    pub network_graph: Arc<NetworkGraph>,
    pub onion_messenger: Arc<OnionMessenger>,
    pub inbound_payments: PaymentInfoStorage,
    pub outbound_payments: PaymentInfoStorage,
    pub ldk_data_dir: String,
    pub network: Network,
    pub logger: Arc<disk::FilesystemLogger>,
    pub port: u16,
    pub announced_listen_addr: String,
    pub node_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub pubkey: bitcoin::secp256k1::PublicKey,
    pub network: bitcoin::network::constants::Network,
    pub port: u16,
    pub node_name: String,
    pub announced_listen_addr: String,
    pub num_usable_channels: usize,
    pub num_channels: usize,
    pub local_balance_msat: u64,
    pub num_peers: usize,
}

impl HttpServerState {
    pub fn node_info(&self) -> NodeInfo {
        NodeInfo {
            pubkey: self.channel_manager.get_our_node_id(),
            network: self.network,
            port: self.port,
            node_name: self.node_name.to_string(),
            announced_listen_addr: self.announced_listen_addr.to_string(),
            num_usable_channels: self
                .channel_manager
                .list_channels()
                .iter()
                .filter(|c| c.is_usable)
                .count(),
            num_channels: self.channel_manager.list_channels().len(),
            local_balance_msat: self
                .channel_manager
                .list_channels()
                .iter()
                .map(|c| c.balance_msat)
                .sum(),
            num_peers: self.peer_manager.get_peer_node_ids().len(),
        }
    }
}
