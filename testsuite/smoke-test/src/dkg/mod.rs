// Copyright © Aptos Foundation

use crate::smoke_test_environment::SwarmBuilder;
use aptos::test::CliTestFramework;
use aptos_config::keys::ConfigKey;
use aptos_consensus::dkg::build_dkg_pvss_config;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_forge::{Node, NodeExt, Swarm};
use aptos_rest_client::Client;
use aptos_types::{
    dkg::DKGTranscriptWrapper, on_chain_config::DKGState, validator_verifier::ValidatorVerifier,
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::time::Instant;

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
    let mut dkg_state = get_latest_dkg_state(&client).await;
    let timer = Instant::now();
    while timer.elapsed().as_secs() < time_limit_secs
        && !((target_epoch.is_none() || dkg_state.target_epoch == target_epoch.unwrap())
            && dkg_state.state_id == 0)
    {
        std::thread::sleep(Duration::from_secs(1));
        dkg_state = get_latest_dkg_state(&client).await;
    }
    assert!(timer.elapsed().as_secs() < time_limit_secs);
    dkg_state
}

mod dkg_basic;
mod dkg_with_validator_set_change;
