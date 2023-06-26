use base64;
use bitcoin::{hashes::hex::FromHex, BlockHash};
use std::{convert::TryInto, sync::Arc};

use lightning_block_sync::{
    http::{HttpEndpoint, JsonResponse},
    rpc::RpcClient,
};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BlockchainInfo {
    pub latest_height: usize,
    pub latest_blockhash: BlockHash,
    pub chain: String,
}

impl TryInto<BlockchainInfo> for JsonResponse {
    type Error = std::io::Error;
    fn try_into(self) -> std::io::Result<BlockchainInfo> {
        Ok(BlockchainInfo {
            latest_height: self.0["blocks"].as_u64().unwrap() as usize,
            latest_blockhash: BlockHash::from_hex(self.0["bestblockhash"].as_str().unwrap())
                .unwrap(),
            chain: self.0["chain"].as_str().unwrap().to_string(),
        })
    }
}

pub struct BlockchainHandler {
    rpc_client: Arc<RpcClient>,
}

impl BlockchainHandler {
    pub async fn new() -> std::io::Result<Self> {
        let host = "127.0.0.1".to_string();
        let rpc_user: String = "admin".to_string();
        let port = 18443;
        let rpc_password: String = "password".to_string();
        let http_endpoint = HttpEndpoint::for_host(host.clone()).with_port(port);
        let rpc_credentials =
            base64::encode(format!("{}:{}", rpc_user.clone(), rpc_password.clone()));
        let bitcoind_rpc_client = RpcClient::new(&rpc_credentials, http_endpoint)?;
        let _dummy = bitcoind_rpc_client
            .call_method::<BlockchainInfo>("getblockchaininfo", &vec![])
            .await
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied,
				"Failed to make initial call to bitcoind - please check your RPC user/password and access settings")
            })?;
        Ok(Self {
            rpc_client: Arc::new(bitcoind_rpc_client),
        })
    }

    pub async fn get_blockchain_info(&self) -> std::io::Result<BlockchainInfo> {
        let blockchain_info = self
            .rpc_client
            .call_method::<BlockchainInfo>("getblockchaininfo", &vec![])
            .await
            .map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::PermissionDenied,
                "Failed to make initial call to bitcoind - please check your RPC user/password and access settings")
            })?;
        Ok(blockchain_info)
    }

    pub async fn list_wallets(&self) -> std::io::Result<WalletList> {
        let wallet_info = self
            .rpc_client
            .call_method::<WalletList>("listwallets", &vec![])
            .await
            .map_err(|e| {
                dbg!(&e.to_string());
                std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string())
            })?;
        Ok(wallet_info)
    }
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct WalletInfo {
    wallet_name: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct WalletList(Vec<String>);

impl TryInto<WalletList> for JsonResponse {
    type Error = std::io::Error;
    fn try_into(self) -> std::io::Result<WalletList> {
        Ok(WalletList(
            self.0
                .as_array()
                .unwrap()
                .into_iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect(),
        ))
    }
}

impl TryInto<WalletInfo> for JsonResponse {
    type Error = std::io::Error;
    fn try_into(self) -> std::io::Result<WalletInfo> {
        dbg!(&self.0);
        Ok(WalletInfo {
            wallet_name: self.0["walletName"].as_str().unwrap().to_string(),
        })
    }
}
