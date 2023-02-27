use base64;
use bitcoin::blockdata::transaction::Transaction;
use bitcoin::consensus::encode;
use bitcoin::hash_types::Txid;
use lightning::chain::chaininterface::BroadcasterInterface;
use lightning_block_sync::http::HttpEndpoint;
use lightning_block_sync::rpc::RpcClient;
use serde_json;
use std::sync::Arc;

pub struct MyBroadcastInterface{
    bitcoind_rpc_client: Arc<RpcClient>,
    host: String,
    port: u16,
    rpc_user: String,
    rpc_password: String,
    handle: tokio::runtime::Handle,
}

fn default_rpc_details() -> (
    tokio::runtime::Handle,
    std::string::String,
    u16,
    std::string::String,
    std::string::String,
    HttpEndpoint,
    std::string::String,
) {
    let handle = tokio::runtime::Handle::current();
    let host = "http://127.0.0.1".to_string();
    let port = 18443;
    let rpc_user: String = "admin".to_string();
    let rpc_password: String = "password".to_string();
    let http_endpoint = HttpEndpoint::for_host(host.clone()).with_port(port);
    let rpc_credentials = base64::encode(format!("{}:{}", rpc_user, rpc_password));
    return (
        handle,
        host,
        port,
        rpc_user,
        rpc_password,
        http_endpoint,
        rpc_credentials,
    );
}

impl MyBroadcastInterface {
    pub async fn new() -> std::io::Result<Self> {
        let (handle, host, port, rpc_user, rpc_password, http_endpoint, rpc_credentials) =
            default_rpc_details();
        let bitcoind_rpc_client = RpcClient::new(&rpc_credentials, http_endpoint)?;
        let client = Self {
            bitcoind_rpc_client: Arc::new(bitcoind_rpc_client),
            host,
            port,
            rpc_user,
            rpc_password,
            handle: handle.clone(),
        };
        Ok(client)
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub enum Target {
    Background,
    Normal,
    HighPriority,
}

impl BroadcasterInterface for MyBroadcastInterface {
    fn broadcast_transaction(&self, tx: &Transaction) {
        let bitcoind_rpc_client = self.bitcoind_rpc_client.clone();
        let tx_serialized = serde_json::json!(encode::serialize_hex(tx));
        self.handle.spawn(async move {
            // This may error due to RL calling `broadcast_transaction` with the same transaction
            // multiple times, but the error is safe to ignore.
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
