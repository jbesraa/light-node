// #[derive(Serialize, Deserialize, Debug)]
// pub struct NodeInfo {
//     pub pubkey: String,
//     pub network: String,
//     pub port: u16,
//     pub node_name: String,
//     pub announced_listen_addr: String,
//     pub num_usable_channels: usize,
//     pub num_channels: usize,
//     pub local_balance_msat: u64,
//     pub num_peers: usize,
// }

// #[tauri::command]
// async fn get_data() -> Result<NodeInfo, ()> {
//     let resp = reqwest::get("http://127.0.0.1:8181/lightning/info")
//         .await
//         .unwrap()
//         .json::<NodeInfo>()
//         .await
//         .unwrap();
//     println!("{:#?}", resp);
//     Ok(resp)
// }
