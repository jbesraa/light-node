// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod mmc;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

use bdk::bitcoincore_rpc::bitcoincore_rpc_json::GetWalletInfoResult;
use bitcoin::{network, secp256k1};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::Manager;

#[derive(Serialize, Deserialize, Debug)]
pub struct NodeInfo {
    pub pubkey: String,
    pub network: String,
    pub port: u16,
    pub node_name: String,
    pub announced_listen_addr: String,
    pub num_usable_channels: usize,
    pub num_channels: usize,
    pub local_balance_msat: u64,
    pub num_peers: usize,
}

#[tauri::command]
async fn get_data() -> Result<NodeInfo, ()> {
    let resp = reqwest::get("http://127.0.0.1:8181/lightning/info")
        .await
        .unwrap()
        .json::<NodeInfo>()
        .await
        .unwrap();
    println!("{:#?}", resp);
    Ok(resp)
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct BlockchainInfo {
    pub latest_height: usize,
    pub latest_blockhash: String,
    pub chain: String,
}

#[tauri::command]
async fn get_blockchain_info() -> Result<BlockchainInfo, ()> {
    let resp = reqwest::get("http://127.0.0.1:8181/blockchain/info")
        .await
        .unwrap()
        .json::<BlockchainInfo>()
        .await
        .unwrap();
    Ok(resp)
}

#[tauri::command]
async fn list_wallets() -> Result<Vec<String>, ()> {
    let resp = reqwest::get("http://127.0.0.1:8181/wallet/list")
        .await
        .unwrap()
        .json::<Vec<String>>()
        .await
        .unwrap();
    Ok(resp)
}

#[tauri::command]
async fn wallet_info(wallet_name: String) -> Result<GetWalletInfoResult, ()> {
    let url = format!("http://127.0.0.1:8181/wallet/{}/info", wallet_name);
    let resp = reqwest::get(url)
        .await
        .unwrap()
        .json::<GetWalletInfoResult>()
        .await
        .unwrap();
    Ok(resp)
}

#[tauri::command]
async fn send(sender: String, amount: u64, rec: String) -> Result<(), ()> {
    let url = format!("http://127.0.0.1:8181/wallet/{}/send", sender);
    let resp = reqwest::Client::new()
        .post(url)
        .json(&serde_json::json!({
            "amount": amount,
            "rec": rec
        }))
        .send()
        .await
        .unwrap()
        .json::<String>()
        .await
        .unwrap();
    dbg!(&resp);
    Ok(())
}

#[tauri::command]
async fn new_mmc() -> Result<String, ()> {
    let url = format!("http://127.0.0.1:8181/mmc");
    let resp = reqwest::get(url)
        .await
        .unwrap()
        .json::<String>()
        .await
        .unwrap();
    Ok(resp)
}

#[tauri::command]
async fn generate_address(wallet_name: String) -> Result<String, ()> {
    let url = format!("http://127.0.0.1:8181/wallet/{}/address", wallet_name);
    let resp = reqwest::get(url)
        .await
        .unwrap()
        .json::<String>()
        .await
        .unwrap();
    dbg!(&resp);
    Ok(resp)
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
            new_mmc
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
