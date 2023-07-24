use crate::{blockchain::BlockchainHandler, wallet::BitcoinWallet};
use actix_web::{
    get, post,
    web::{self, Data},
    Responder,
};
use bdk::{
    keys::{
        bip39::{Language, Mnemonic, WordCount},
        GeneratableKey, GeneratedKey,
    },
    miniscript::Segwitv0,
};
use bitcoin::Address;
use std::{
    str::FromStr,
    sync::{Arc, Mutex},
};

#[get("/wallet/list")]
pub async fn wallet_list(
    data: Data<Mutex<Arc<BlockchainHandler>>>,
) -> actix_web::Result<impl Responder> {
    let data = data.lock().unwrap();
    let info = data.list_wallets().await.unwrap();
    Ok(web::Json(info))
}

#[get("/wallet/{wallet_name}/info")]
pub async fn my_wallet_info(wallet_name: web::Path<String>) -> actix_web::Result<impl Responder> {
    let wallet = BitcoinWallet::load_by_wallet_name(wallet_name.into_inner());
    let info = wallet.wallet_info().unwrap();
    Ok(web::Json(info))
}

#[get("/wallet/{wallet_name}/address")]
pub async fn generate_address(wallet_name: web::Path<String>) -> actix_web::Result<impl Responder> {
    let wallet = BitcoinWallet::load_by_wallet_name(wallet_name.into_inner());
    let info = wallet.generate_address().unwrap();
    Ok(web::Json(info.address))
}

#[post("/wallet/{wallet_name}/send")]
pub async fn send_coins(
    wallet_name: web::Path<String>,
    rec_address: web::Json<String>,
    amount: web::Json<u64>,
) -> actix_web::Result<impl Responder> {
    let wallet = BitcoinWallet::load_by_wallet_name(wallet_name.into_inner());
    let _info = wallet.send_tx(
        Address::from_str(&rec_address.into_inner()).unwrap(),
        amount.into_inner(),
    );
    Ok(web::Json(""))
}
#[get("/mmc")]
pub async fn new_mmc() -> actix_web::Result<impl Responder> {
    let mmc: GeneratedKey<Mnemonic, Segwitv0> =
        Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
    Ok(web::Json(mmc.word_iter().collect::<Vec<&str>>().join(" ")))
}
