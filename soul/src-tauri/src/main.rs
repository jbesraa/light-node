// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod blockchain;
pub mod mmc;
pub mod wallet;

use bdk::{
    bitcoincore_rpc::bitcoincore_rpc_json::GetWalletInfoResult, wallet::AddressInfo,
    TransactionDetails,
};
use bitcoin::Address;
use blockchain::WalletList;
use tauri::Manager;

#[tauri::command]
async fn list_wallets() -> Result<WalletList, ()> {
    let blockchain = blockchain::BlockchainHandler::new().await.unwrap();
    let resp = blockchain.list_wallets().await.unwrap();
    Ok(resp)
}

#[tauri::command]
async fn wallet_info(wallet_name: String) -> Result<GetWalletInfoResult, ()> {
    let wallet = wallet::BitcoinWallet::load_by_wallet_name(wallet_name.clone());
    wallet.sync_wallet().unwrap();
    let wallet_info = wallet.wallet_info().unwrap();
    Ok(wallet_info)
}

#[tauri::command]
async fn generate_address(wallet_name: String) -> Result<String, ()> {
    let wallet = wallet::BitcoinWallet::load_by_wallet_name(wallet_name.clone());
    let address_info: AddressInfo = wallet.generate_address().unwrap();
    Ok(address_info.address.to_string())
}

#[tauri::command]
async fn generate_to_address(wallet_name: String) -> Result<(), ()> {
    let wallet = wallet::BitcoinWallet::load_by_wallet_name(wallet_name.clone());
    let hashes = wallet.generate_to_address(450);
    dbg!(hashes.len());
    Ok(())
}

#[tauri::command]
async fn send_tx(sender: String, amount: u64, rec: String) -> Result<bool, ()> {
    let wallet = wallet::BitcoinWallet::load_by_wallet_name(sender.clone());
    let address: Address = rec.parse().unwrap();
    let res = wallet.send_tx(address, amount).unwrap();
    Ok(res)
}

#[tauri::command]
async fn list_txs(wallet_name: String) -> Result<Vec<TransactionDetails>, ()> {
    let wallet = wallet::BitcoinWallet::load_by_wallet_name(wallet_name.clone());
    let res = wallet.list_txs().unwrap();
    Ok(res)
}

#[tauri::command]
async fn load_wallet_with_mmc(mmc: String) -> Result<(), ()> {
    wallet::BitcoinWallet::load_with_mmc(mmc.clone());
    Ok(())
}

#[tauri::command]
async fn new_mmc() -> String {
    mmc::generate_mnemonic()
}

fn main() {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(debug_assertions)] // only include this code on debug builds
            {
                let window = app.get_window("main").unwrap();
                window.open_devtools();
                window.close_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_wallets,
            wallet_info,
            generate_address,
            generate_to_address,
            send_tx,
            list_txs,
            load_wallet_with_mmc,
            new_mmc
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
