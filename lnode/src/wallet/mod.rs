use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{DerivationPath, KeySource};
use bdk::bitcoin::Network;
use bdk::bitcoincore_rpc::bitcoincore_rpc_json::{
    GetBalancesResult, GetWalletInfoResult, SignRawTransactionResult,
};
use bdk::bitcoincore_rpc::{RawTx, RpcApi};
use bdk::blockchain::rpc::{Auth, RpcBlockchain, RpcConfig};
use bdk::blockchain::ConfigurableBlockchain;
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::DescriptorKey::Secret;
use bdk::keys::{DerivableKey, DescriptorKey, ExtendedKey, GeneratableKey};
use bdk::miniscript::miniscript::Segwitv0;
use bdk::sled;
use bdk::wallet::{wallet_name_from_descriptor, AddressIndex};
use bdk::wallet::{AddressInfo, SyncOptions};
use bdk::Wallet;
use bitcoin::{Address, Amount, Transaction, Txid};
use lightning::chain::chaininterface::{ConfirmationTarget, FeeEstimator};
use lightning_block_sync::http::JsonResponse;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

const MIN_FEERATE: u32 = 253;

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum Target {
    Background,
    Normal,
    HighPriority,
}

#[derive(Debug)]
pub struct BitcoinRPC {
    client: RpcBlockchain,
}

impl BitcoinRPC {
    fn new(wallet_name: &str) -> Self {
        let blockchain: RpcBlockchain = RpcBlockchain::from_config(&RpcConfig {
            url: "http://127.0.0.1:18443".to_string(),
            auth: Auth::UserPass {
                username: "admin".to_string(),
                password: "password".to_string(),
            },
            network: Network::Regtest,
            wallet_name: wallet_name.to_string(),
            sync_params: None,
        })
        .unwrap();
        Self { client: blockchain }
    }
}

#[derive(Debug)]
pub struct BitcoinWallet {
    pub rpc: BitcoinRPC,
    pub wallet_name: String,
    pub receive_desc: String,
    pub change_desc: String,
    pub wallet: Wallet<bdk::sled::Tree>,
    fees: Arc<HashMap<Target, AtomicU32>>,
}

impl BitcoinWallet {
    pub fn new_wallet() -> Self {
        let (receive_desc, change_desc) = Self::generate_descx();
        let wallet_name: String = wallet_name_from_descriptor(
            &receive_desc,
            Some(&change_desc),
            Network::Regtest,
            &Secp256k1::new(),
        )
        .unwrap();
        let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
        fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
        fees.insert(Target::Normal, AtomicU32::new(2000));
        fees.insert(Target::HighPriority, AtomicU32::new(5000));

        Self {
            rpc: BitcoinRPC::new(&wallet_name),
            wallet_name: wallet_name.to_string(),
            wallet: Wallet::new(
                &receive_desc,
                Some(&change_desc),
                Network::Regtest,
                Self::create_db_tree(&wallet_name),
            )
            .unwrap(),
            receive_desc,
            change_desc,
            fees: Arc::new(fees),
        }
    }

    pub fn load_wallet(receive_desc: String, change_desc: String) -> Self {
        let wallet_name: String = wallet_name_from_descriptor(
            &receive_desc,
            Some(&change_desc),
            Network::Regtest,
            &Secp256k1::new(),
        )
        .unwrap();
        let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
        fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
        fees.insert(Target::Normal, AtomicU32::new(2000));
        fees.insert(Target::HighPriority, AtomicU32::new(5000));
        Self {
            rpc: BitcoinRPC::new(&wallet_name),
            wallet_name: wallet_name.to_string(),
            wallet: Wallet::new(
                &receive_desc,
                Some(&change_desc),
                Network::Regtest,
                Self::create_db_tree(&wallet_name),
            )
            .unwrap(),
            receive_desc,
            change_desc,
            fees: Arc::new(fees),
        }
    }

    pub fn sync_wallet(&self) {
        self.wallet
            .sync(&self.rpc.client, SyncOptions { progress: None })
            .unwrap();
    }

    pub fn create_raw_tx(
        &self,
        outputs: HashMap<String, Amount>,
    ) -> Result<Transaction, bdk::bitcoincore_rpc::Error> {
        self.rpc
            .client
            .create_raw_transaction(&[], &outputs, None, None)
    }

    pub fn send_raw_tx<R: RawTx>(&self, tx: R) -> Result<Txid, bdk::bitcoincore_rpc::Error> {
        self.rpc.client.send_raw_transaction(tx)
    }

    pub fn sign_raw_tx<R: RawTx>(
        &self,
        tx: R,
    ) -> Result<SignRawTransactionResult, bdk::bitcoincore_rpc::Error> {
        self.rpc
            .client
            .sign_raw_transaction_with_wallet(tx, None, None)
    }

    pub fn fund_raw_tx<R: RawTx>(&self, tx: R) {
        let options = serde_json::json!({
            // LDK gives us feerates in satoshis per KW but Bitcoin Core here expects fees
            // denominated in satoshis per vB. First we need to multiply by 4 to convert weight
            // units to virtual bytes, then divide by 1000 to convert KvB to vB.
            "fee_rate": self.get_est_sat_per_1000_weight(ConfirmationTarget::Normal) as f64 / 250.0,
            // While users could "cancel" a channel open by RBF-bumping and paying back to
            // themselves, we don't allow it here as its easy to have users accidentally RBF bump
            // and pay to the channel funding address, which results in loss of funds. Real
            // LDK-based applications should enable RBF bumping and RBF bump either to a local
            // change address or to a new channel output negotiated with the same node.
            "replaceable": false,
        });
        self.rpc.client.fund_raw_transaction(tx, None, None);
    }

    pub fn wallet_info(&self) -> Result<GetWalletInfoResult, bdk::bitcoincore_rpc::Error> {
        self.rpc.client.get_wallet_info()
    }

    pub fn get_balances(&self) -> Result<GetBalancesResult, bdk::bitcoincore_rpc::Error> {
        self.rpc.client.get_balances()
    }

    pub fn list_wallets_1(&self) -> Vec<std::string::String> {
        self.rpc.client.list_wallets().unwrap()
    }

    pub fn list_wallets(&self) -> Vec<std::string::String> {
        self.rpc.client.list_wallets().unwrap()
    }

    pub fn generate_address(&self) -> AddressInfo {
        self.wallet.get_address(AddressIndex::New).unwrap()
    }

    pub fn generate_to_address(
        &self,
        count: u64,
        address: Address,
    ) -> Result<Vec<bitcoin::BlockHash>, bdk::bitcoincore_rpc::Error> {
        self.rpc.client.generate_to_address(count, &address)
    }

    fn create_db_tree(wallet_name: &str) -> sled::Tree {
        // Create the datadir to store wallet data
        let mut datadir = dirs_next::home_dir().unwrap();
        datadir.push(".bdk-example");
        let database = sled::open(datadir).unwrap();
        let db_tree = database.open_tree(wallet_name.clone()).unwrap();
        db_tree
    }

    fn generate_descx() -> (String, String) {
        let secp = Secp256k1::new();
        let passphrase = None;
        let mnemonic = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
        println!(
            "Create new wallet with mnemonic: {:#}",
            &mnemonic.to_string()
        );
        let xkey: ExtendedKey = (mnemonic, passphrase).into_extended_key().unwrap();
        let xprv = xkey.into_xprv(Network::Regtest).unwrap();
        // Create derived privkey from the above master privkey
        // We use the following derivation paths for receive and change keys
        // receive: "m/84h/1h/0h/0"
        // change: "m/84h/1h/0h/1"
        let mut keys = Vec::new();
        for path in ["m/84h/1h/0h/0", "m/84h/1h/0h/1"] {
            let deriv_path: DerivationPath = DerivationPath::from_str(path).unwrap();
            let derived_xprv = &xprv.derive_priv(&secp, &deriv_path).unwrap();
            let origin: KeySource = (xprv.fingerprint(&secp), deriv_path);
            let derived_xprv_desc_key: DescriptorKey<Segwitv0> = derived_xprv
                .into_descriptor_key(Some(origin), DerivationPath::default())
                .unwrap();
            // Wrap the derived key with the wpkh() string to produce a descriptor string
            if let Secret(key, _, _) = derived_xprv_desc_key {
                let mut desc = "wpkh(".to_string();
                desc.push_str(&key.to_string());
                desc.push_str(")");
                keys.push(desc);
            }
        }
        (keys[0].clone(), keys[1].clone())
    }
}

impl FeeEstimator for BitcoinWallet {
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
