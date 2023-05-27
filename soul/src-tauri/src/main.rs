// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

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
            get_data,
            greet,
            get_blockchain_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
