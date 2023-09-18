// Copyright © Aptos Foundation

use aptos_rest_client::Client;
use aptos_types::on_chain_config::DKGState;
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::time::Duration;
use tokio::time::Instant;
use aptos_consensus::dkg::build_dkg_pvss_config;
use aptos_types::dkg::DKGTranscriptWrapper;
use aptos_types::validator_verifier::ValidatorVerifier;

async fn get_latest_dkg_state(rest_client: &Client) -> DKGState {
    let maybe_response = rest_client
        .get_account_resource_bcs::<DKGState>(CORE_CODE_ADDRESS, "0x1::dkg::DKGState")
        .await;
    let response = maybe_response.unwrap();
    let dkg_state = response.into_inner();
    println!(
        "Latest DKGState： target_epoch={}, dkg_active={}.",
        dkg_state.target_epoch, dkg_state.state_id
    );
    dkg_state
}

async fn wait_for_epoch_fully_entered(
    client: &Client,
    target_epoch: Option<u64>,
    time_limit_secs: u64,
) -> DKGState {
    let mut dkg_state = get_latest_dkg_state(client).await;
    let timer = Instant::now();
    while timer.elapsed().as_secs() < time_limit_secs
        && !((target_epoch.is_none() || dkg_state.target_epoch == target_epoch.unwrap())
            && dkg_state.state_id == 0)
    {
        std::thread::sleep(Duration::from_secs(1));
        dkg_state = get_latest_dkg_state(client).await;
    }
    assert!(timer.elapsed().as_secs() < time_limit_secs);
    dkg_state
}

/// Verify that DKG transcript of epoch i (stored in `new_dkg_state`) is correctly generated
/// by the validator set in epoch i-1 (stored in `new_dkg_state`).
fn verify_dkg_transcript(old_dkg_state: &DKGState, new_dkg_state: &DKGState) -> bool {
    println!(
        "Verifying the transcript generated for epoch {} by epoch {}.",
        new_dkg_state.target_epoch, old_dkg_state.target_epoch
    );
    let verifier = ValidatorVerifier::from(old_dkg_state.validator_set.as_ref().unwrap());
    let (_, pvss_config) = build_dkg_pvss_config(
        old_dkg_state.target_epoch,
        new_dkg_state.validator_set.as_ref().unwrap(),
    );
    let trxs: DKGTranscriptWrapper =
        bcs::from_bytes(new_dkg_state.serialized_transcript.as_slice()).unwrap();
    trxs.verify(&pvss_config, &verifier).is_ok()
}

fn num_validators(dkg_state: &DKGState) -> usize {
    dkg_state
        .validator_set
        .as_ref()
        .unwrap()
        .active_validators
        .len()
}

mod dkg_basic;
mod dkg_with_validator_join_leave;
