use crate::{
    http_server::state::{HttpServerState, PeerInfo},
    ldk::core::CoreLDK,
};
use actix_web::{
    get, post,
    web::{self, Data},
    Responder,
};
use std::sync::Mutex;

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
