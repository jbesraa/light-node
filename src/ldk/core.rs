use base64;
use bitcoin::BlockHash;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::consensus::encode;
use bitcoin::hash_types::Txid;
use lightning::chain::chaininterface::BroadcasterInterface;
use lightning::chain::chaininterface::{ConfirmationTarget, FeeEstimator};
use lightning::util::logger::{Logger, Record};
use lightning_block_sync::{BlockSource, AsyncBlockSourceResult, BlockHeaderData, BlockData};
use lightning_block_sync::http::HttpEndpoint;
use lightning_block_sync::rpc::RpcClient;
use serde_json;
use tokio::runtime::Handle;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub struct CoreLDK {
    bitcoind_rpc_client: Arc<RpcClient>,
    host: String,
    port: u16,
    rpc_user: String,
    rpc_password: String,
    handle: tokio::runtime::Handle,
    fees: Arc<HashMap<Target, AtomicU32>>,
}

fn default_rpc_details() -> (
    std::string::String,
    u16,
    std::string::String,
    std::string::String,
    HttpEndpoint,
    std::string::String,
) {
    let host = "http://127.0.0.1".to_string();
    let port = 18443;
    let rpc_user: String = "admin".to_string();
    let rpc_password: String = "password".to_string();
    let http_endpoint = HttpEndpoint::for_host(host.clone()).with_port(port);
    let rpc_credentials = base64::encode(format!("{}:{}", rpc_user, rpc_password));
    return (
        host,
        port,
        rpc_user,
        rpc_password,
        http_endpoint,
        rpc_credentials,
    );
}

impl CoreLDK {
    pub async fn new(handle: Handle) -> std::io::Result<Self> {
        let (host, port, rpc_user, rpc_password, http_endpoint, rpc_credentials) =
            default_rpc_details();
        let bitcoind_rpc_client = RpcClient::new(&rpc_credentials, http_endpoint)?;
        let fees = default_fees();
        let client = Self {
            bitcoind_rpc_client: Arc::new(bitcoind_rpc_client),
            host,
            port,
            rpc_user,
            rpc_password,
            handle: handle.clone(),
            fees: Arc::new(fees),
        };
        Ok(client)
    }
}

impl BroadcasterInterface for CoreLDK {
    fn broadcast_transaction(&self, tx: &Transaction) {
        let bitcoind_rpc_client = self.bitcoind_rpc_client.clone();
        let tx_serialized = serde_json::json!(encode::serialize_hex(tx));
        self.handle.spawn(async move {
            match bitcoind_rpc_client
                .call_method::<Txid>("sendrawtransaction", &vec![tx_serialized])
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    let err_str = e.get_ref().unwrap().to_string();
                    if !err_str.contains("Transaction already in block chain")
                        && !err_str.contains("Inputs missing or spent")
                        && !err_str.contains("bad-txns-inputs-missingorspent")
                        && !err_str.contains("txn-mempool-conflict")
                        && !err_str.contains("non-BIP68-final")
                        && !err_str.contains("insufficient fee, rejecting replacement ")
                    {
                        panic!("{}", e);
                    }
                }
            }
        });
    }
}

// Fees Management
const MIN_FEERATE: u32 = 253;

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum Target {
    Background,
    Normal,
    HighPriority,
}

fn default_fees() -> HashMap<Target, AtomicU32> {
    let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
    fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
    fees.insert(Target::Normal, AtomicU32::new(2000));
    fees.insert(Target::HighPriority, AtomicU32::new(5000));

    fees
}

impl FeeEstimator for CoreLDK {
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


impl BlockSource for CoreLDK {
	fn get_header<'a>(
		&'a self, header_hash: &'a BlockHash, height_hint: Option<u32>,
	) -> AsyncBlockSourceResult<'a, BlockHeaderData> {
		Box::pin(async move { self.bitcoind_rpc_client.get_header(header_hash, height_hint).await })
	}

	fn get_block<'a>(
		&'a self, header_hash: &'a BlockHash,
	) -> AsyncBlockSourceResult<'a, BlockData> {
		Box::pin(async move { self.bitcoind_rpc_client.get_block(header_hash).await })
	}

	fn get_best_block<'a>(&'a self) -> AsyncBlockSourceResult<(BlockHash, Option<u32>)> {
		Box::pin(async move { self.bitcoind_rpc_client.get_best_block().await })
	}
}
