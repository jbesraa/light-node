// use lightning::chain::chaininterface::{ConfirmationTarget, FeeEstimator};
// use std::collections::HashMap;
// use std::sync::atomic::{AtomicU32, Ordering};
// use std::sync::Arc;

// const MIN_FEERATE: u32 = 253;

// #[derive(Clone, Eq, Hash, PartialEq)]
// pub enum Target {
//     Background,
//     Normal,
//     HighPriority,
// }

// pub struct MyFeeEstimator {
//     fees: Arc<HashMap<Target, AtomicU32>>,
// }

// impl Default for MyFeeEstimator {
//     fn default() -> Self {
//         let mut fees: HashMap<Target, AtomicU32> = HashMap::new();
//         fees.insert(Target::Background, AtomicU32::new(MIN_FEERATE));
//         fees.insert(Target::Normal, AtomicU32::new(2000));
//         fees.insert(Target::HighPriority, AtomicU32::new(5000));

//         Self {
//             fees: Arc::new(fees),
//         }
//     }
// }

// impl FeeEstimator for MyFeeEstimator {
//     fn get_est_sat_per_1000_weight(&self, confirmation_target: ConfirmationTarget) -> u32 {
//         match confirmation_target {
//             ConfirmationTarget::Background => self
//                 .fees
//                 .get(&Target::Background)
//                 .unwrap()
//                 .load(Ordering::Acquire),
//             ConfirmationTarget::Normal => self
//                 .fees
//                 .get(&Target::Normal)
//                 .unwrap()
//                 .load(Ordering::Acquire),
//             ConfirmationTarget::HighPriority => self
//                 .fees
//                 .get(&Target::HighPriority)
//                 .unwrap()
//                 .load(Ordering::Acquire),
//         }
//     }
// }
