use std::sync::Mutex;
use actix_web::{
    get,
    web::{self, Data},
    Responder,
};

use crate::http_server::state::HttpServerState;

#[get("/lightning/info")]
pub async fn lightning_node_info(data: Data<Mutex<HttpServerState>>) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    Ok(web::Json(data.node_info()))
}
