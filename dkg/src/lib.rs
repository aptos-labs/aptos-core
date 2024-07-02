// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod agg_trx_producer;
mod counters;
pub mod dkg_manager;
pub mod epoch_manager;
pub mod network;
pub mod network_interface;
pub mod transcript_aggregation;
pub mod types;

use std::sync::Arc;
use rand::{Rng, thread_rng};
use crate::{
    epoch_manager::EpochManager, network::NetworkTask, network_interface::DKGNetworkClient,
};
use aptos_config::config::ReliableBroadcastConfig;
use aptos_event_notifications::{
    DbBackedOnChainConfig, EventNotificationListener, ReconfigNotificationListener,
};
use aptos_network::application::interface::{NetworkClient, NetworkServiceEvents};
use aptos_types::dkg::{DKGTrait, DefaultDKG, DKGTranscript, DKGSessionMetadata};
use aptos_validator_transaction_pool::VTxnPoolState;
use move_core_types::account_address::AccountAddress;
use tokio::runtime::Runtime;
use aptos_crypto::bls12381::{PrivateKey, PublicKey};
use aptos_crypto::Uniform;
use aptos_types::dkg::real_dkg::{RealDKG, RealDKGPublicParams, Transcripts};
use aptos_types::dkg::real_dkg::rounding::MAINNET_STAKES;
use aptos_types::on_chain_config::OnChainRandomnessConfig;
use aptos_types::validator_verifier::{ValidatorConsensusInfo, ValidatorConsensusInfoMoveStruct};
pub use types::DKGMessage;
use crate::dkg_manager::setup_deal_main;
use crate::transcript_aggregation::verify_main;

pub fn start_dkg_runtime(
    my_addr: AccountAddress,
    dkg_dealer_sk: <DefaultDKG as DKGTrait>::DealerPrivateKey,
    network_client: NetworkClient<DKGMessage>,
    network_service_events: NetworkServiceEvents<DKGMessage>,
    reconfig_events: ReconfigNotificationListener<DbBackedOnChainConfig>,
    dkg_start_events: EventNotificationListener,
    vtxn_pool: VTxnPoolState,
    rb_config: ReliableBroadcastConfig,
    randomness_override_seq_num: u64,
) -> Runtime {
    let runtime = aptos_runtimes::spawn_named_runtime("dkg".into(), Some(4));
    let (self_sender, self_receiver) = aptos_channels::new(1_024, &counters::PENDING_SELF_MESSAGES);
    let dkg_network_client = DKGNetworkClient::new(network_client);

    let dkg_epoch_manager = EpochManager::new(
        my_addr,
        dkg_dealer_sk,
        reconfig_events,
        dkg_start_events,
        self_sender,
        dkg_network_client,
        vtxn_pool,
        rb_config,
        randomness_override_seq_num,
    );
    let (network_task, network_receiver) = NetworkTask::new(network_service_events, self_receiver);
    runtime.spawn(network_task.start());
    runtime.spawn(dkg_epoch_manager.start(network_receiver));
    runtime
}

pub fn dummy_dkg_init(rand_config: OnChainRandomnessConfig) -> (AccountAddress, usize, Arc<PrivateKey>, DKGSessionMetadata) {
    let mut rng = thread_rng();
    let n = MAINNET_STAKES.len();
    let my_index = rng.gen_range(0, n);
    let addresses: Vec<AccountAddress> = (0..n).map(|_| AccountAddress::random()).collect();
    let private_keys: Vec<Arc<PrivateKey>> = (0..n).map(|_|Arc::new(PrivateKey::generate(&mut rng))).collect();
    let public_keys : Vec<PublicKey> = (0..n).map(|i|PublicKey::from(private_keys[i].as_ref())).collect();
    let validator_info_vec: Vec<ValidatorConsensusInfoMoveStruct> = (0..n).map(|i| ValidatorConsensusInfo::new(addresses[i], public_keys[i].clone(), MAINNET_STAKES[i].clone()).into()).collect();
    let session_metadata = DKGSessionMetadata {
        dealer_epoch: 999,
        randomness_config: rand_config.into(),
        dealer_validator_set: validator_info_vec.clone(),
        target_validator_set: validator_info_vec,
    };
    (addresses[my_index], my_index, private_keys[my_index].clone(), session_metadata)
}

pub fn dummy_dkg_init_deal(rand_config: OnChainRandomnessConfig) -> (RealDKGPublicParams, DKGTranscript) {
    let  (my_addr, my_index, my_sk, session_metadata) = dummy_dkg_init(rand_config);
    setup_deal_main::<RealDKG>(my_addr, my_index, my_sk, &session_metadata).unwrap()
}

pub fn dummy_dkg_init_deal_verify(rand_config: OnChainRandomnessConfig) -> (RealDKGPublicParams, Transcripts, Transcripts) {
    let  (my_addr, my_index, my_sk, session_metadata) = dummy_dkg_init(rand_config);
    let (pp, trx_0) = setup_deal_main::<RealDKG>(my_addr, my_index, my_sk.clone(), &session_metadata).unwrap();
    let (_, trx_1) = setup_deal_main::<RealDKG>(my_addr, my_index, my_sk.clone(), &session_metadata).unwrap();
    let ts_0 = verify_main::<RealDKG>(&pp, trx_0.transcript_bytes).unwrap();
    let ts_1 = verify_main::<RealDKG>(&pp, trx_1.transcript_bytes).unwrap();
    (pp, ts_0, ts_1)
}

#[test]
fn print_trx_size() {
    let (_, trx_1, _) = dummy_dkg_init_deal_verify(OnChainRandomnessConfig::default_v1());
    let (_, trx_2, _) = dummy_dkg_init_deal_verify(OnChainRandomnessConfig::default_enabled());
    let size_1 = bcs::serialized_size(&trx_1).unwrap();
    let size_2 = bcs::serialized_size(&trx_2).unwrap();
    println!("size_1={}, size_2={}", size_1, size_2);
}
