#![allow(unused_imports)]
use bdk::bitcoin::secp256k1::Secp256k1;
use bdk::bitcoin::util::bip32::{DerivationPath, KeySource};
use bdk::bitcoin::Amount;
use bdk::bitcoin::Network;
use bdk::bitcoincore_rpc::{Auth as rpc_auth, Client, RpcApi};
use bdk::blockchain::rpc::{Auth, RpcBlockchain, RpcConfig, RpcSyncParams};
use bdk::blockchain::{ConfigurableBlockchain, NoopProgress};
use bdk::keys::bip39::{Language, Mnemonic, WordCount};
use bdk::keys::DescriptorKey::Secret;
use bdk::keys::{DerivableKey, DescriptorKey, ExtendedKey, GeneratableKey, GeneratedKey};
use bdk::miniscript::miniscript::Segwitv0;
use bdk::sled;
use bdk::wallet::wallet_name_from_descriptor;
use bdk::wallet::{signer::SignOptions, AddressIndex, SyncOptions};
use bdk::Wallet;
use std::str::FromStr;

fn connect(wallet_name: &str) -> RpcBlockchain {
    RpcBlockchain::from_config(&RpcConfig {
        url: "http://127.0.0.1:18443".to_string(),
        auth: Auth::UserPass {
            username: "admin".to_string(),
            password: "password".to_string(),
        },
        network: Network::Regtest,
        wallet_name: wallet_name.to_string(),
        sync_params: None,
    })
    .unwrap()
}

pub fn create_wallet() -> Wallet<bdk::sled::Tree> {
    let (receive_desc, change_desc) = get_descriptors();
    let wallet_name = wallet_name_from_descriptor(
        &receive_desc,
        Some(&change_desc),
        Network::Regtest,
        &Secp256k1::new(),
    )
    .unwrap();
    // Create the datadir to store wallet data
    let mut datadir = dirs_next::home_dir().unwrap();
    datadir.push(".bdk-example");
    let database = sled::open(datadir).unwrap();
    let db_tree = database.open_tree(wallet_name.clone()).unwrap();
    let blockchain = connect(&wallet_name);
    // Combine everything and finally create the BDK wallet structure
    let wallet = Wallet::new(&receive_desc, Some(&change_desc), Network::Regtest, db_tree).unwrap();
    // Sync the wallet
    wallet
        .sync(&blockchain, SyncOptions { progress: None })
        .unwrap();
    let address = wallet.get_address(AddressIndex::New).unwrap().address;
    blockchain.generate_to_address(400, &address).unwrap();
    wallet
}

fn get_descriptors() -> (String, String) {
    let secp = Secp256k1::new();
    let password = Some("random password".to_string());
    let mnemonic = Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
    let xkey: ExtendedKey = (mnemonic, password).into_extended_key().unwrap();
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
