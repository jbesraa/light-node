use lightning::chain::keysinterface::KeysManager;
use rand::{thread_rng, Rng};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::SystemTime;

pub fn new(ldk_data_dir: &str) -> KeysManager {
    let keys_seed_path = format!("{}/keys_seed", ldk_data_dir.clone());

    // If we're restarting and already have a key seed, read it from disk. Else,
    // create a new one.
    let keys_seed = if let Ok(seed) = fs::read(keys_seed_path.clone()) {
        assert_eq!(seed.len(), 32);
        let mut key = [0; 32];
        key.copy_from_slice(&seed);
        key
    } else {
        let mut key = [0; 32];
        thread_rng().fill_bytes(&mut key);
        match File::create(keys_seed_path.clone()) {
            Ok(mut f) => {
                f.write_all(&key)
                    .expect("Failed to write node keys seed to disk");
                f.sync_all().expect("Failed to sync node keys seed to disk");
            }
            Err(e) => {
                println!(
                    "ERROR: Unable to create keys seed file {}: {}",
                    keys_seed_path, e
                );
                panic!(); // FIXME
            }
        }
        key
    };
    let cur = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    dbg!(&keys_seed);
    let manager = KeysManager::new(&keys_seed, cur.as_secs(), cur.subsec_nanos());
    return manager;
}
