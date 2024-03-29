use crate::mmc;
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{DerivationPath, KeySource};
use bdk::bitcoin::Network;
use bdk::bitcoincore_rpc::bitcoincore_rpc_json::{
    FundRawTransactionResult, GetBalancesResult, GetWalletInfoResult, SignRawTransactionResult,
};
use bdk::bitcoincore_rpc::{Client, RawTx, RpcApi};
use bdk::blockchain::rpc::{Auth, RpcBlockchain, RpcConfig};
use bdk::blockchain::ConfigurableBlockchain;
use bdk::blockchain::{noop_progress, Blockchain};
use bdk::database::SqliteDatabase;
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::DescriptorKey::Secret;
use bdk::keys::{DerivableKey, DescriptorKey, ExtendedKey, GeneratableKey};
use bdk::miniscript::miniscript::Segwitv0;
use bdk::template::Bip84;
use bdk::wallet::{wallet_name_from_descriptor, AddressIndex};
use bdk::wallet::{AddressInfo, SyncOptions};
use bdk::{sled, KeychainKind, LocalUtxo, SignOptions, TransactionDetails};
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::util::bip32::ExtendedPrivKey;
use bitcoin::{Address, Amount, Transaction, Txid};
use lightning::chain::chaininterface::{ConfirmationTarget, FeeEstimator};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

const MIN_FEERATE: u32 = 253;
const NETWORK: Network = Network::Regtest;

#[derive(Debug)]
struct TryFromSliceError(());

fn _slice_to_array_64<T>(slice: &[T]) -> Result<&[T; 64], TryFromSliceError> {
    if slice.len() == 64 {
        let ptr = slice.as_ptr() as *const [T; 64];
        unsafe { Ok(&*ptr) }
    } else {
        Err(TryFromSliceError(()))
    }
}

#[derive(Clone, Eq, Hash, PartialEq, Debug)]
pub enum Target {
    Background,
    Normal,
    HighPriority,
}

#[derive(Debug)]
pub struct BitcoinRPC {
    rpc: RpcBlockchain,
}

fn _generate_descx(mnemonic: Option<String>) -> (String, String) {
    let secp = Secp256k1::new();
    let xkey: ExtendedKey = match mnemonic {
        Some(mnemonic) => Mnemonic::from_str(&mnemonic)
            .unwrap()
            .into_extended_key()
            .unwrap(),
        None => Mnemonic::generate((WordCount::Words12, Language::English))
            .unwrap()
            .into_extended_key()
            .unwrap(),
    };

    let xprv: ExtendedPrivKey = xkey.into_xprv(NETWORK).unwrap();
    let mut keys = Vec::new();
    for path in ["m/84h/1h/0h/0", "m/84h/1h/0h/1"] {
        let deriv_path: DerivationPath = DerivationPath::from_str(path).unwrap();
        let derived_xprv = &xprv.derive_priv(&secp, &deriv_path).unwrap();
        let origin: KeySource = (xprv.fingerprint(&secp), deriv_path);
        let derived_xprv_desc_key: DescriptorKey<Segwitv0> = derived_xprv
            .into_descriptor_key(Some(origin), DerivationPath::default())
            .unwrap();
        if let Secret(key, _, _) = derived_xprv_desc_key {
            let mut desc = "wpkh(".to_string();
            desc.push_str(&key.to_string());
            desc.push_str(")");
            keys.push(desc);
        }
    }
    (keys[0].clone(), keys[1].clone())
}

fn extended_keys(mnemonic: &str, network: Network) -> ExtendedPrivKey {
    let xkey: ExtendedKey = Mnemonic::from_str(&mnemonic)
        .unwrap()
        .into_extended_key()
        .unwrap();
    let xprv: ExtendedPrivKey = xkey.into_xprv(network).unwrap();

    xprv
}

fn wallet_dir(wallet_name: &str) -> (SqliteDatabase, File) {
    let mut datadir = dirs_next::home_dir().unwrap();
    let file = File::create(format!("/Users/esraa/.bdk-example/{}", &wallet_name)).unwrap();
    // file.write_all(mnemonic.as_bytes()).unwrap();

    let database_path = format!("{}.sqlite", wallet_name);
    datadir.push(".bdk-example");
    datadir.push(database_path.clone());
    let database = SqliteDatabase::new(datadir);
    (database, file)
}

pub fn load_with_mmc(mnemonic: String) -> (bdk::Wallet<SqliteDatabase>, String) {
    dbg!("Loading wallet with mnemonic: {}", mnemonic.clone());
    let xprv: ExtendedPrivKey = extended_keys(&mnemonic, NETWORK);
    let wallet_name = wallet_name_from_descriptor(
        Bip84(xprv, KeychainKind::External),
        Some(Bip84(xprv, KeychainKind::Internal)),
        NETWORK,
        &Secp256k1::new(),
    )
    .expect("Failed to derive on-chain wallet name");
    let (database, mut file) = wallet_dir(&wallet_name);
    file.write_all(mnemonic.as_bytes()).unwrap();
    let bdk_wallet = bdk::Wallet::new(
        Bip84(xprv, KeychainKind::External),
        Some(Bip84(xprv, KeychainKind::Internal)),
        NETWORK,
        database,
    )
    .expect("Failed to set up on-chain wallet");
    (bdk_wallet, wallet_name)
}

pub fn tomato() {
    // let mnemonic: String = mmc::generate_mnemonic();
    let mnemonic: String =
        "hero wet visit similar quantum width capable rare genius jewel hood obey".to_string();
    let (wallet, wallet_name) = load_with_mmc(mnemonic.clone());
    dbg!(wallet.get_balance().unwrap());
    let blockchain: RpcBlockchain = RpcBlockchain::from_config(&RpcConfig {
        url: "http://127.0.0.1:18443".to_string(),
        auth: Auth::UserPass {
            username: "admin".to_string(),
            password: "password".to_string(),
        },
        network: NETWORK,
        wallet_name: wallet_name.clone(),
        sync_params: None,
    })
    .unwrap();

    let address = wallet.get_address(AddressIndex::New).unwrap();
    blockchain
        .generate_to_address(101, &address.address)
        .unwrap();

    wallet
        .sync(&blockchain, SyncOptions { progress: None })
        .unwrap();
    dbg!(wallet.get_balance().unwrap());
}

impl BitcoinRPC {
    fn new(wallet_name: &str) -> Self {
        let blockchain: RpcBlockchain = RpcBlockchain::from_config(&RpcConfig {
            url: "http://127.0.0.1:18443".to_string(),
            auth: Auth::UserPass {
                username: "admin".to_string(),
                password: "password".to_string(),
            },
            network: NETWORK,
            wallet_name: wallet_name.to_string(),
            sync_params: None,
        })
        .unwrap();
        Self { rpc: blockchain }
    }

    // fn new_esplora(wallet_name: &str) {
    //     let blockchain: EsploraBlockchain = EsploraBlockchain::from_config(&EsploraConfig {
    //         base_url: "https://blockstream.info/testnet/api".to_string(),
    //         network: NETWORK,
    //         wallet_name: wallet_name.to_string(),
    //         sync_params: None,
    //     });
    // }
}

#[derive(Debug)]
pub struct BitcoinWallet {
    pub blockchain: BitcoinRPC,
    pub wallet_name: String,
    pub fees: Arc<HashMap<Target, AtomicU32>>,
    pub wallet: Mutex<bdk::Wallet<SqliteDatabase>>,
}

impl BitcoinWallet {
    pub fn sync_wallet(&self) -> Result<(), bdk::Error> {
        self.wallet
            .lock()
            .unwrap()
            .sync(&self.blockchain.rpc, SyncOptions { progress: None })
    }

    pub fn create_raw_tx(
        &self,
        outputs: HashMap<String, Amount>,
    ) -> Result<Transaction, bdk::bitcoincore_rpc::Error> {
        self.blockchain
            .rpc
            .create_raw_transaction(&[], &outputs, None, None)
    }

    pub fn send_raw_tx<R: RawTx>(&self, tx: R) -> Result<Txid, bdk::bitcoincore_rpc::Error> {
        self.blockchain.rpc.send_raw_transaction(tx)
    }

    pub fn sign_raw_tx<R: RawTx>(
        &self,
        tx: R,
    ) -> Result<SignRawTransactionResult, bdk::bitcoincore_rpc::Error> {
        self.blockchain
            .rpc
            .sign_raw_transaction_with_wallet(tx, None, None)
    }

    pub fn fund_raw_tx<R: RawTx>(
        &self,
        tx: R,
    ) -> Result<FundRawTransactionResult, bdk::bitcoincore_rpc::Error> {
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
        self.blockchain.rpc.fund_raw_transaction(tx, None, None)
    }

    pub fn wallet_info(&self) -> Result<GetWalletInfoResult, bdk::bitcoincore_rpc::Error> {
        self.blockchain.rpc.get_wallet_info()
    }

    pub fn specific_wallet_info(
        wallet_name: &str,
    ) -> Result<GetWalletInfoResult, bdk::bitcoincore_rpc::Error> {
        let wallet_rpc = BitcoinRPC::new(&wallet_name);

        wallet_rpc.rpc.get_wallet_info()
    }
    // pub fn specific_generate_address(
    //     wallet_name: &str,
    // ) -> Result<GetWalletInfoResult, bdk::bitcoincore_rpc::Error> {
    //     let wallet_rpc = BitcoinRPC::new(&wallet_name);

    //     wallet_rpc.client.generate_to_address
    // }

    // pub fn get_balances(&self) -> Result<GetBalancesResult, bdk::bitcoincore_rpc::Error> {
    //     self.blockchain
    // }

    pub fn list_wallets(&self) -> Result<Vec<std::string::String>, bdk::bitcoincore_rpc::Error> {
        self.blockchain.rpc.list_wallets()
    }

    pub fn generate_address(&self) -> Result<AddressInfo, bdk::Error> {
        self.wallet.lock().unwrap().get_address(AddressIndex::New)
    }

    // let mut tx_builder = w1.inner.lock().unwrap().build_tx();
    // let (mut psbt, _) = tx_builder
    //     .add_recipient(w2_address.script_pubkey(), 1000)
    //     .finish()
    //     .unwrap();

    pub fn sign_psbt(&self, mut psbt: PartiallySignedTransaction) -> Result<bool, bdk::Error> {
        let wallet = self.wallet.lock().unwrap();
        let signopt = SignOptions {
            assume_height: None,
            ..Default::default()
        };
        let res = wallet.sign(&mut psbt, signopt).unwrap();
        match res {
            true => {
                let tx = psbt.extract_tx();
                // Broadcast the transaction
                match self.blockchain.rpc.broadcast(&tx) {
                    Ok(_txid) => Ok(true),
                    Err(e) => Ok(false),
                }
            }
            false => Ok(false),
        }
    }

    pub fn send_tx(&self, recipient: Address, amount: u64) -> Result<bool, bdk::Error> {
        let psbt = match self.create_psbt(recipient, amount).unwrap() {
            psbt => psbt,
            _ => return Ok(false),
        };
        match self.sign_psbt(psbt) {
            Ok(res) => Ok(res),
            Err(e) => Ok(false),
        }
    }

    pub fn create_psbt(
        &self,
        recipient: Address,
        amount: u64,
    ) -> Result<PartiallySignedTransaction, bdk::Error> {
        let wallet = self.wallet.lock().unwrap();
        let mut tx_builder = wallet.build_tx();
        tx_builder.set_recipients(vec![(recipient.script_pubkey(), amount)]);
        let (psbt, _) = tx_builder.finish().unwrap();
        Ok(psbt)
    }

    pub fn generate_to_address(&self, count: u64) -> Vec<bitcoin::BlockHash> {
        let address_info = self.generate_address().unwrap();
        let hashes = self
            .blockchain
            .rpc
            .generate_to_address(count, &address_info.address)
            .unwrap();
        self.sync_wallet().unwrap();
        hashes
    }

    fn create_db_tree(wallet_name: &str) -> sled::Tree {
        // Create the datadir to store wallet data
        let mut datadir = dirs_next::home_dir().unwrap();
        datadir.push(".bdk-example");
        let database = sled::open(datadir).unwrap();
        let db_tree = database.open_tree(wallet_name.clone()).unwrap();
        db_tree
    }

    pub fn list_txs(&self) -> Result<Vec<TransactionDetails>, bdk::Error> {
        self.sync_wallet().unwrap();
        let txs = self
            .wallet
            .lock()
            .unwrap()
            .list_transactions(false)
            .unwrap();
        Ok(txs)
    }

    pub fn list_unspent(&self) -> Result<Vec<LocalUtxo>, bdk::Error> {
        self.sync_wallet().unwrap();
        let utxos = self.wallet.lock().unwrap().list_unspent().unwrap();
        Ok(utxos)
    }

    pub fn load_with_mmc(mnemonic: String) -> Self {
        dbg!("Loading wallet with mnemonic: {}", mnemonic.clone());
        let xkey: ExtendedKey = Mnemonic::from_str(&mnemonic)
            .unwrap()
            .into_extended_key()
            .unwrap();
        let xprv: ExtendedPrivKey = xkey.into_xprv(NETWORK).unwrap();

        let wallet_name = bdk::wallet::wallet_name_from_descriptor(
            Bip84(xprv, bdk::KeychainKind::External),
            Some(Bip84(xprv, bdk::KeychainKind::Internal)),
            NETWORK,
            &Secp256k1::new(),
        )
        .expect("Failed to derive on-chain wallet name");
        // let bdk_data_dir = format!("{}/bdk", "./ldk_node");
        let mut datadir = dirs_next::home_dir().unwrap();
        let mut file = File::create(format!("/home/ecode/.bdk-example/{}", &wallet_name)).unwrap();
        file.write_all(mnemonic.as_bytes()).unwrap();

        let database_path = format!("{}.sqlite", wallet_name);
        datadir.push(".bdk-example");
        datadir.push(database_path.clone());
        let database = SqliteDatabase::new(datadir);

        let bdk_wallet = bdk::Wallet::new(
            Bip84(xprv, bdk::KeychainKind::External),
            Some(Bip84(xprv, bdk::KeychainKind::Internal)),
            NETWORK,
            database,
        )
        .expect("Failed to set up on-chain wallet");
        let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
        fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
        fees.insert(Target::Normal, AtomicU32::new(2000));
        fees.insert(Target::HighPriority, AtomicU32::new(5000));

        Self {
            blockchain: BitcoinRPC::new(&wallet_name),
            wallet_name: wallet_name.to_string(),
            wallet: Mutex::new(bdk_wallet),
            fees: Arc::new(fees),
        }
    }
    // Initialize the on-chain wallet and chain access

    pub fn load_by_wallet_name(wallet_name: String) -> Self {
        let mut datadir = dirs_next::home_dir().unwrap();
        let database_path = format!("{}.sqlite", wallet_name);
        datadir.push(".bdk-example");
        datadir.push(database_path.clone());
        let database = SqliteDatabase::new(datadir);
        let mut file = match File::open(format!("/home/ecode/.bdk-example/{}", &wallet_name)) {
            Ok(file) => file,
            Err(_) => panic!("Wallet is not found"),
        };
        let mut mnemonic = String::new();
        file.read_to_string(&mut mnemonic).unwrap();
        let xkey: ExtendedKey = Mnemonic::from_str(&mnemonic)
            .unwrap()
            .into_extended_key()
            .unwrap();
        let xprv: ExtendedPrivKey = xkey.into_xprv(NETWORK).unwrap();
        let bdk_wallet = bdk::Wallet::new(
            Bip84(xprv, bdk::KeychainKind::External),
            Some(Bip84(xprv, bdk::KeychainKind::Internal)),
            NETWORK,
            database,
        )
        .expect("Failed to set up on-chain wallet");
        let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
        fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
        fees.insert(Target::Normal, AtomicU32::new(2000));
        fees.insert(Target::HighPriority, AtomicU32::new(5000));

        Self {
            blockchain: BitcoinRPC::new(&wallet_name),
            wallet_name: wallet_name.to_string(),
            wallet: Mutex::new(bdk_wallet),
            fees: Arc::new(fees),
        }
    }
    // Initialize the on-chain wallet and chain access

    fn generate_descx(mnemonic: Option<String>) -> (String, String) {
        let secp = Secp256k1::new();
        // let passphrase = None;
        let xkey: ExtendedKey = match mnemonic {
            Some(mnemonic) => Mnemonic::from_str(&mnemonic)
                .unwrap()
                .into_extended_key()
                .unwrap(),
            None => Mnemonic::generate((WordCount::Words12, Language::English))
                .unwrap()
                .into_extended_key()
                .unwrap(),
        };

        // println!("Wallet  mnemonic: {:#}", &mnemonic.to_string());
        // let xkey: ExtendedKey = (mnemonic, passphrase).into_extended_key().unwrap();
        let xprv: ExtendedPrivKey = xkey.into_xprv(NETWORK).unwrap();
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

//test
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tx_list() {
        tomato();
        assert!(false);
        // let wallet = BitcoinWallet::load_by_wallet_name("vu8wg3uragz04yvm".to_string());
        // let res = wallet.list_txs().unwrap();
        // println!("{:#?}", res);
        // assert!(false);
    }

    // #[test]
    // fn new_wallet() {
    //     let mmc1 =
    //         "morning vault innocent rose also alien neutral piano decorate around pioneer system";
    //     let mmc2: &str =
    //         "winner maid tower wrong rebuild list net amused okay turtle shrimp swallow";
    //     // let w1 = BitcoinWallet::load_with_mmc(mmc1.to_string());
    //     let w1 = BitcoinWallet::load_by_wallet_name("7a096s3m0f2y89pr".to_string());
    //     // w2.sync_wallet().unwrap();
    //     // let w2_address = w2.generate_address().unwrap();
    //     // w1.send_tx(w2_address.address, 1000);
    //     // w1.generate_to_address(1000).unwrap();
    //     let w1_info = w1.wallet_info().unwrap();
    //     // let w2_info = w2.wallet_info().unwrap();
    //     // dbg!(&w1_info);
    //     dbg!(&w1_info);
    //     // dbg!(&w1);
    //     // dbg!(&w2);
    //     assert!(w1.wallet_name.len() > 0);
    //     // assert!(w2.wallet_name.len() > 0);
    // }
}

// pub fn get_wallet(&self) -> Wallet<sled::Tree> {
//     Wallet::new(
//         &self.receive_desc,
//         Some(&self.change_desc),
//         NETWORK,
//         Self::create_db_tree(&self.wallet_name),
//     )
//     .unwrap()
// }

// let seed_bytes = match &*self.entropy_source_config.read().unwrap() {
//     Some(EntropySourceConfig::SeedBytes(bytes)) => bytes.clone(),
//     Some(EntropySourceConfig::SeedFile(seed_path)) => {
//         io::utils::read_or_generate_seed_file(seed_path)
//     }
//     Some(EntropySourceConfig::Bip39Mnemonic {
//         mnemonic,
//         passphrase,
//     }) => match passphrase {
//         Some(passphrase) => mnemonic.to_seed(passphrase),
//         None => mnemonic.to_seed(""),
//     },
//     None => {
//         // Default to read or generate from the default location generate a seed file.
//         let seed_path = format!("{}/keys_seed", config.storage_dir_path);
//         io::utils::read_or_generate_seed_file(&seed_path)
//     }
// };

// let xprv = bitcoin::util::bip32::ExtendedPrivKey::new_master(config.network, &seed_bytes)
//     .expect("Failed to read wallet master key");
//
// fn create_db_tree(wallet_name: &str) -> sled::Tree {
//     // Create the datadir to store wallet data
//     datadir.push(".bdk-example");
//     let database = sled::open(datadir).unwrap();
//     let db_tree = database.open_tree(wallet_name.clone()).unwrap();
//     db_tree
// }
// let binding: TxBuilder<SqliteDatabase, BranchAndBoundCoinSelection, CreateTx> =
//     self.inner.lock().unwrap().build_tx();
// let (mut psbt, _): (PartiallySignedTransaction, TransactionDetails) = binding
//     .enable_rbf()
//     .add_recipient(recipient.script_pubkey(), amount.to_sat())
//     .do_not_spend_change();

// Ok(psbt.clone())
// let xkey: ExtendedKey = match mnemonic {
//     Some(mnemonic) => Mnemonic::from_str(&mnemonic)
//         .unwrap()
//         .into_extended_key()
//         .unwrap(),
//     None => Self::generate_mnemonic(),
// };
// let mut datadir = dirs_next::home_dir().unwrap();
// let mut file = File::create(format!("/home/ecode/.bdk-example/{}", &wallet_name)).unwrap();
// let database_path = format!("{}.sqlite", wallet_name);
// datadir.push(".bdk-example");
// datadir.push(database_path.clone());
// let database = SqliteDatabase::new(datadir);
