use bitcoin::consensus::encode;
use lightning::{events::{ Event, PaymentFailureReason, PaymentPurpose }, chain::keysinterface::{SpendableOutputDescriptor, EntropySource}, util::persist::KVStorePersister};
use lightning_persister::FilesystemPersister;
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::Arc,
    time::Duration,
};

use bitcoin::{secp256k1::Secp256k1, Network, Transaction};
use bitcoin_bech32::WitnessProgram;
use lightning::{
    chain::{
        chaininterface::{BroadcasterInterface, ConfirmationTarget, FeeEstimator},
        keysinterface::KeysManager,
    },
    routing::gossip::NodeId,
};
use rand::{thread_rng, Rng};
use std::collections::hash_map::Entry;

use crate::{
    types::{
        ChannelManager, HTLCStatus, MillisatAmount, NetworkGraph, PaymentInfo, PaymentInfoStorage,
    },
    utils::hex::{hex_str, to_vec}, wallet::BitcoinWallet,
};

use super::core::CoreLDK;


pub(crate) const PENDING_SPENDABLE_OUTPUT_DIR: &'static str = "pending_spendable_outputs";

pub async fn handle_ldk_events(
    channel_manager: &Arc<ChannelManager>,
    bitcoind_client: &CoreLDK,
    network_graph: &NetworkGraph,
    keys_manager: &KeysManager,
    inbound_payments: &PaymentInfoStorage,
    outbound_payments: &PaymentInfoStorage,
    persister: &Arc<FilesystemPersister>,
    network: Network,
    event: Event,
) {
    match event {
        Event::FundingGenerationReady {
            temporary_channel_id,
            counterparty_node_id,
            channel_value_satoshis,
            output_script,
            ..
        } => {
            // Construct the raw transaction with one output, that is paid the amount of the
            // channel.
            let addr = WitnessProgram::from_scriptpubkey(
                &output_script[..],
                match network {
                    Network::Bitcoin => bitcoin_bech32::constants::Network::Bitcoin,
                    Network::Testnet => bitcoin_bech32::constants::Network::Testnet,
                    Network::Regtest => bitcoin_bech32::constants::Network::Regtest,
                    Network::Signet => bitcoin_bech32::constants::Network::Signet,
                },
            )
            .expect("Lightning funding tx should always be to a SegWit output")
            .to_address();
            let mut outputs = vec![HashMap::with_capacity(1)];
            outputs[0].insert(addr, channel_value_satoshis as f64 / 100_000_000.0);
            let raw_tx = bitcoind_client.create_raw_transaction(outputs).await;

            // Have your wallet put the inputs into the transaction such that the output is
            // satisfied.
            let funded_tx = bitcoind_client.fund_raw_transaction(raw_tx).await;

            // Sign the final funding transaction and broadcast it.
            let signed_tx = bitcoind_client
                .sign_raw_transaction_with_wallet(funded_tx.hex)
                .await;
            assert_eq!(signed_tx.complete, true);
            let final_tx: Transaction =
                encode::deserialize(&to_vec(&signed_tx.hex).unwrap()).unwrap();
            // Give the funding transaction back to LDK for opening the channel.
            if channel_manager
                .funding_transaction_generated(
                    &temporary_channel_id,
                    &counterparty_node_id,
                    final_tx,
                )
                .is_err()
            {
                println!(
					"\nERROR: Channel went away before we could fund it. The peer disconnected or refused the channel.");
                print!("> ");
                io::stdout().flush().unwrap();
            }
        }
        Event::PaymentClaimable {
            payment_hash,
            purpose,
            amount_msat,
            receiver_node_id: _,
            via_channel_id: _,
            via_user_channel_id: _,
            claim_deadline: _,
            onion_fields: _,
        } => {
            println!(
                "\nEVENT: received payment from payment hash {} of {} millisatoshis",
                hex_str(&payment_hash.0),
                amount_msat,
            );
            print!("> ");
            io::stdout().flush().unwrap();
            let payment_preimage = match purpose {
                PaymentPurpose::InvoicePayment {
                    payment_preimage, ..
                } => payment_preimage,
                PaymentPurpose::SpontaneousPayment(preimage) => Some(preimage),
            };
            channel_manager.claim_funds(payment_preimage.unwrap());
        }
        Event::PaymentClaimed {
            payment_hash,
            purpose,
            amount_msat,
            receiver_node_id: _,
        } => {
            println!(
                "\nEVENT: claimed payment from payment hash {} of {} millisatoshis",
                hex_str(&payment_hash.0),
                amount_msat,
            );
            print!("> ");
            io::stdout().flush().unwrap();
            let (payment_preimage, payment_secret) = match purpose {
                PaymentPurpose::InvoicePayment {
                    payment_preimage,
                    payment_secret,
                    ..
                } => (payment_preimage, Some(payment_secret)),
                PaymentPurpose::SpontaneousPayment(preimage) => (Some(preimage), None),
            };
            let mut payments = inbound_payments.lock().unwrap();
            match payments.entry(payment_hash) {
                Entry::Occupied(mut e) => {
                    let payment = e.get_mut();
                    payment.status = HTLCStatus::Succeeded;
                    payment.preimage = payment_preimage;
                    payment.secret = payment_secret;
                }
                Entry::Vacant(e) => {
                    e.insert(PaymentInfo {
                        preimage: payment_preimage,
                        secret: payment_secret,
                        status: HTLCStatus::Succeeded,
                        amt_msat: MillisatAmount(Some(amount_msat)),
                    });
                }
            }
        }
        Event::PaymentSent {
            payment_preimage,
            payment_hash,
            fee_paid_msat,
            ..
        } => {
            let mut payments = outbound_payments.lock().unwrap();
            for (hash, payment) in payments.iter_mut() {
                if *hash == payment_hash {
                    payment.preimage = Some(payment_preimage);
                    payment.status = HTLCStatus::Succeeded;
                    println!(
                        "\nEVENT: successfully sent payment of {} millisatoshis{} from \
								 payment hash {:?} with preimage {:?}",
                        payment.amt_msat,
                        if let Some(fee) = fee_paid_msat {
                            format!(" (fee {} msat)", fee)
                        } else {
                            "".to_string()
                        },
                        hex_str(&payment_hash.0),
                        hex_str(&payment_preimage.0)
                    );
                    print!("> ");
                    io::stdout().flush().unwrap();
                }
            }
        }
        Event::OpenChannelRequest { .. } => {
            // Unreachable, we don't set manually_accept_inbound_channels
        }
        Event::PaymentPathSuccessful { .. } => {}
        Event::PaymentPathFailed { .. } => {}
        Event::ProbeSuccessful { .. } => {}
        Event::ProbeFailed { .. } => {}
        Event::PaymentFailed {
            payment_hash,
            reason,
            ..
        } => {
            print!(
                "\nEVENT: Failed to send payment to payment hash {:?}: {:?}",
                hex_str(&payment_hash.0),
                if let Some(r) = reason {
                    r
                } else {
                    PaymentFailureReason::RetriesExhausted
                }
            );
            print!("> ");
            io::stdout().flush().unwrap();

            let mut payments = outbound_payments.lock().unwrap();
            if payments.contains_key(&payment_hash) {
                let payment = payments.get_mut(&payment_hash).unwrap();
                payment.status = HTLCStatus::Failed;
            }
        }
        Event::PaymentForwarded {
            prev_channel_id,
            next_channel_id,
            fee_earned_msat,
            claim_from_onchain_tx,
            outbound_amount_forwarded_msat,
        } => {
            let read_only_network_graph = network_graph.read_only();
            let nodes = read_only_network_graph.nodes();
            let channels = channel_manager.list_channels();

            let node_str = |channel_id: &Option<[u8; 32]>| match channel_id {
                None => String::new(),
                Some(channel_id) => match channels.iter().find(|c| c.channel_id == *channel_id) {
                    None => String::new(),
                    Some(channel) => {
                        match nodes.get(&NodeId::from_pubkey(&channel.counterparty.node_id)) {
                            None => "private node".to_string(),
                            Some(node) => match &node.announcement_info {
                                None => "unnamed node".to_string(),
                                Some(announcement) => {
                                    format!("node {}", announcement.alias)
                                }
                            },
                        }
                    }
                },
            };
            let channel_str = |channel_id: &Option<[u8; 32]>| {
                channel_id
                    .map(|channel_id| format!(" with channel {}", hex_str(&channel_id)))
                    .unwrap_or_default()
            };
            let from_prev_str = format!(
                " from {}{}",
                node_str(&prev_channel_id),
                channel_str(&prev_channel_id)
            );
            let to_next_str = format!(
                " to {}{}",
                node_str(&next_channel_id),
                channel_str(&next_channel_id)
            );

            let from_onchain_str = if claim_from_onchain_tx {
                "from onchain downstream claim"
            } else {
                "from HTLC fulfill message"
            };
            let amt_args = if let Some(v) = outbound_amount_forwarded_msat {
                format!("{}", v)
            } else {
                "?".to_string()
            };
            if let Some(fee_earned) = fee_earned_msat {
                println!(
                    "\nEVENT: Forwarded payment for {} msat{}{}, earning {} msat {}",
                    amt_args, from_prev_str, to_next_str, fee_earned, from_onchain_str
                );
            } else {
                println!(
                    "\nEVENT: Forwarded payment for {} msat{}{}, claiming onchain {}",
                    amt_args, from_prev_str, to_next_str, from_onchain_str
                );
            }
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::HTLCHandlingFailed { .. } => {}
        Event::PendingHTLCsForwardable { time_forwardable } => {
            let forwarding_channel_manager = channel_manager.clone();
            let min = time_forwardable.as_millis() as u64;
            tokio::spawn(async move {
                let millis_to_sleep = thread_rng().gen_range(min, min * 5) as u64;
                tokio::time::sleep(Duration::from_millis(millis_to_sleep)).await;
                forwarding_channel_manager.process_pending_htlc_forwards();
            });
        }
        Event::SpendableOutputs { outputs } => {
            // SpendableOutputDescriptors, of which outputs is a vec of, are critical to keep track
            // of! While a `StaticOutput` descriptor is just an output to a static, well-known key,
            // other descriptors are not currently ever regenerated for you by LDK. Once we return
            // from this method, the descriptor will be gone, and you may lose track of some funds.
            //
            // Here we simply persist them to disk, with a background task running which will try
            // to spend them regularly (possibly duplicatively/RBF'ing them). These can just be
            // treated as normal funds where possible - they are only spendable by us and there is
            // no rush to claim them.
            for output in outputs {
                let key = hex_str(&keys_manager.get_secure_random_bytes());
                // Note that if the type here changes our read code needs to change as well.
                let output: SpendableOutputDescriptor = output;
                persister
                    .persist(
                        &format!("{}/{}", PENDING_SPENDABLE_OUTPUT_DIR, key),
                        &output,
                    )
                    .unwrap();
            }
        }
        Event::ChannelPending {
            channel_id,
            counterparty_node_id,
            ..
        } => {
            println!(
                "\nEVENT: Channel {} with peer {} is pending awaiting funding lock-in!",
                hex_str(&channel_id),
                hex_str(&counterparty_node_id.serialize()),
            );
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::ChannelReady {
            ref channel_id,
            user_channel_id: _,
            ref counterparty_node_id,
            channel_type: _,
        } => {
            println!(
                "\nEVENT: Channel {} with peer {} is ready to be used!",
                hex_str(channel_id),
                hex_str(&counterparty_node_id.serialize()),
            );
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::ChannelClosed {
            channel_id,
            reason,
            user_channel_id: _,
        } => {
            println!(
                "\nEVENT: Channel {} closed due to: {:?}",
                hex_str(&channel_id),
                reason
            );
            print!("> ");
            io::stdout().flush().unwrap();
        }
        Event::DiscardFunding { .. } => {
            // A "real" node should probably "lock" the UTXOs spent in funding transactions until
            // the funding transaction either confirms, or this event is generated.
        }
        Event::HTLCIntercepted { .. } => {}
    }
}
