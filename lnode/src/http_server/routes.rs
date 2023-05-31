use crate::{
    http_server::state::{HttpServerState, PeerInfo},
    ldk::core::CoreLDK,
    wallet::BitcoinWallet,
};
use actix_web::{
    get, post,
    web::{self, Data},
    Responder,
};
use std::sync::{Arc, Mutex};

#[get("/lightning/info")]
pub async fn lightning_node_info(
    data: Data<Mutex<HttpServerState>>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    Ok(web::Json(data.node_info()))
}

#[get("/lightning/peers/list")]
pub async fn lightning_peers_list(
    data: Data<Mutex<HttpServerState>>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    Ok(web::Json(data.list_peers()))
}

#[post("/lightning/peers/connect")]
pub async fn lightning_peers_connect(
    data: Data<Mutex<HttpServerState>>,
    info: web::Json<PeerInfo>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    match data.connect_peer(info.into_inner()).await {
        Ok(_) => Ok(web::Json("")),
        Err(_) => Err(actix_web::error::ErrorBadRequest("")),
    }
}

#[get("/blockchain/info")]
pub async fn blockchain_info(data: Data<Mutex<CoreLDK>>) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    let bc_info = data.get_blockchain_info().await;
    dbg!(&bc_info);
    Ok(web::Json(bc_info))
}

#[get("/wallet/info")]
pub async fn wallet_info(
    data: Data<Mutex<Arc<BitcoinWallet>>>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    let info = data.wallet_info().unwrap();
    Ok(web::Json(info))
}

#[get("/wallet/list")]
pub async fn wallet_list(
    data: Data<Mutex<Arc<BitcoinWallet>>>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    let info = data.list_wallets().unwrap();
    Ok(web::Json(info))
}

#[get("/wallet/address/new")]
pub async fn generate_address(
    data: Data<Mutex<Arc<BitcoinWallet>>>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    let info = data.generate_address().unwrap();
    Ok(web::Json(info.address))
}

#[get("/wallet/{count}/generate")]
pub async fn generate_to_address(
    count: web::Path<u64>,
    data: Data<Mutex<Arc<BitcoinWallet>>>,
) -> actix_web::Result<impl Responder> {
    let count = count.into_inner();
    let data = data.lock().unwrap();
    let info = data.generate_to_address(count);
    match info {
        Ok(dat) => {
            dbg!(&dat);
            return Ok(web::Json("OK"));
        }
        Err(_) => Err(actix_web::error::ErrorBadRequest("")),
    }
}
