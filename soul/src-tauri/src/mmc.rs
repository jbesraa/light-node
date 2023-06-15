//use bdk::descriptor::Segwitv0;
//use bdk::keys::bip39::{Language, Mnemonic, WordCount};
//use bdk::keys::GeneratedKey;

//pub fn generate_mnemonic() -> String {
//    // let secp = Secp256k1::new();
//    let mmc: GeneratedKey<Mnemonic, Segwitv0> =
//        Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();
//    mmc.word_iter().collect::<Vec<&str>>().join(" ")
//    // mmc.into_extended_key().unwrap()
//}

////test
//#[cfg(test)]
//mod tests {
//    use super::*;

//    #[test]
//    fn new_wallet() {
//        let mmc1 =
//            "morning vault innocent rose also alien neutral piano decorate around pioneer system";
//        let mmc2: &str =
//            "winner maid tower wrong rebuild list net amused okay turtle shrimp swallow";
//        // let w1 = BitcoinWallet::load_with_mmc(mmc1.to_string());
//        let w1 = BitcoinWallet::load_by_wallet_name("7a096s3m0f2y89pr".to_string());
//        // w2.sync_wallet().unwrap();
//        // let w2_address = w2.generate_address().unwrap();
//        // w1.send_tx(w2_address.address, 1000);
//        // w1.generate_to_address(1000).unwrap();
//        let w1_info = w1.wallet_info().unwrap();
//        // let w2_info = w2.wallet_info().unwrap();
//        // dbg!(&w1_info);
//        dbg!(&w1_info);
//        // dbg!(&w1);
//        // dbg!(&w2);
//        assert!(w1.wallet_name.len() > 0);
//        // assert!(w2.wallet_name.len() > 0);
//    }
//}
