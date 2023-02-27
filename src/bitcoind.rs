#![allow(unused_imports)]
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{DerivationPath, KeySource};
use bdk::bitcoin::Amount;
use bdk::bitcoin::Network;
use bdk::bitcoincore_rpc::bitcoincore_rpc_json::{
    GetBalancesResult, GetBlockchainInfoResult, GetMiningInfoResult, GetNetworkInfoResult,
    LoadWalletResult,
};
use bdk::bitcoincore_rpc::{Auth as rpc_auth, Client, RpcApi};
use bdk::blockchain::rpc::{Auth, RpcBlockchain, RpcConfig};
use bdk::blockchain::{ConfigurableBlockchain, NoopProgress};
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::DescriptorKey::Secret;
use bdk::keys::{DerivableKey, DescriptorKey, ExtendedKey, GeneratableKey, GeneratedKey};
use bdk::miniscript::miniscript::Segwitv0;
use bdk::sled;
use bdk::wallet::wallet_name_from_descriptor;
use bdk::wallet::{signer::SignOptions, AddressIndex};
use bdk::Wallet;
use lightning::chain::chaininterface::{ConfirmationTarget, FeeEstimator};
use rand::prelude::*;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct BitcoindClient;

pub fn connect(query: Option<&str>) -> bdk::bitcoincore_rpc::Client {
    let rpc_auth = rpc_auth::UserPass("admin".to_string(), "password".to_string());
    let rpc_url: String = "http://127.0.0.1:18443".to_string();
    let rpc_url = match query {
        Some(q) => format!("{}/{}", rpc_url, q),
        None => rpc_url,
    };
    let core_rpc = Client::new(&rpc_url, rpc_auth).unwrap();
    core_rpc
}

pub fn get_wallets_info() -> Vec<std::string::String> {
    let core_rpc = connect(None);
    core_rpc.list_wallets().unwrap()
}

pub fn get_network_info() -> GetNetworkInfoResult {
    let core_rpc = connect(None);
    core_rpc.get_network_info().unwrap()
}

pub fn get_blockchain_info() -> GetBlockchainInfoResult {
    let core_rpc = connect(None);
    core_rpc.get_blockchain_info().unwrap()
}

pub fn get_balances() -> GetBalancesResult {
    let core_rpc = connect(Some("wallet/test"));
    core_rpc.get_balances().unwrap()
}

pub fn get_mining_info() -> GetMiningInfoResult {
    let core_rpc = connect(Some("wallet/test"));
    core_rpc.get_mining_info().unwrap()
}

pub fn create_wallet(wallet_name: &str) -> LoadWalletResult {
    let core_rpc = connect(None);
    core_rpc
        .create_wallet(wallet_name, None, None, None, None)
        .unwrap()
}

pub fn load_wallet(wallet_name: &str) -> LoadWalletResult {
    let core_rpc = connect(None);
    core_rpc.load_wallet(wallet_name).unwrap()
}

pub fn generate_to_address(
    address: bdk::bitcoin::Address,
    block_num: u64,
) -> Vec<bdk::bitcoin::BlockHash> {
    let core_rpc = connect(None);
    core_rpc.generate_to_address(block_num, &address).unwrap()
}

impl FeeEstimator for BitcoindClient {
    fn get_est_sat_per_1000_weight(&self, confirmation_target: ConfirmationTarget) -> u32 {
        match confirmation_target {
            ConfirmationTarget::Background => self
                .fees
                .get(&Target::Background)
                .unwrap()
                .load(Ordering::Acquire),
            ConfirmationTarget::Normal => self
                .fees
                .get(&Target::Normal)
                .unwrap()
                .load(Ordering::Acquire),
            ConfirmationTarget::HighPriority => self
                .fees
                .get(&Target::HighPriority)
                .unwrap()
                .load(Ordering::Acquire),
        }
    }
}
