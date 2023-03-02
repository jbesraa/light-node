pub mod bdk_integration;
pub mod bitcoind;
pub mod ldk;

fn main() {
    // let core_address = bitcoind::create_wallet();
    // ldk::start_node();

    // let w = bdk_integration::create_wallet();
    // println!("Wallet Network: {:?}", w.network());
    // println!("Wallet Balance: {:?}", w.get_balance().unwrap());
    // println!("Hello, world!");
    let w = bitcoind::get_wallets_info();
    let n = bitcoind::get_network_info();
    let i = bitcoind::get_blockchain_info();
    // let m = bitcoind::get_mining_info();
    // let b = bitcoind::get_balances();
    dbg!(w);
    // dbg!(b);
    dbg!(i);
    // dbg!(n);
    // dbg!(m);
}
