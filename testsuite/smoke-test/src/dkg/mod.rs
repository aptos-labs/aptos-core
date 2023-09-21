// Copyright © Aptos Foundation

use aptos_consensus::dkg::build_dkg_pvss_config;
use aptos_crypto::Uniform;
use aptos_dkg::pvss::{
    das::{DealtSecretKey, InputSecret, PublicParameters},
    encryption_dlog::g1::DecryptPrivKey,
    traits::{Convert, Reconstructable, Transcript},
    weighted::weighting::WeightedKey,
    Player, WeightedConfig, WeightedTranscript,
};
use aptos_forge::LocalSwarm;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::DKGTranscriptWrapper, on_chain_config::DKGState, validator_verifier::ValidatorVerifier,
};
use move_core_types::{account_address::AccountAddress, language_storage::CORE_CODE_ADDRESS};
use num_traits::Zero;
use rand::{prelude::StdRng, SeedableRng};
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

type WT = WeightedTranscript<aptos_dkg::pvss::das::Transcript>;

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
fn verify_dkg_transcript(
    old_dkg_state: &DKGState,
    new_dkg_state: &DKGState,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
) -> bool {
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
    if !trxs.verify(&pvss_config, &verifier).is_ok() {
        return false;
    }

    println!("Double-verifying by reconstructing the dealt secret.");
    let dealt_secret_2_from_shares = dealt_secret_from_shares(
        new_dkg_state,
        decrypt_key_map,
        &pvss_config.wc_2,
        &trxs.trx_two_third,
    );
    let dealt_secret_1_from_shares = dealt_secret_from_shares(
        new_dkg_state,
        decrypt_key_map,
        &pvss_config.wc_1,
        &trxs.trx_one_third,
    );
    let dealt_secret_1_from_inputs = dealt_secret_from_input(
        &pvss_config.pp,
        &trxs.trx_one_third,
        old_dkg_state,
        decrypt_key_map,
    );
    let dealt_secret_2_from_inputs = dealt_secret_from_input(
        &pvss_config.pp,
        &trxs.trx_two_third,
        old_dkg_state,
        decrypt_key_map,
    );

    // println!("dealt_secret_1_from_shares={}", hex::encode(dealt_secret_1_from_shares.sub_key().to_bytes()));
    // println!("dealt_secret_2_from_shares={}", hex::encode(dealt_secret_2_from_shares.sub_key().to_bytes()));
    // println!("dealt_secret_1_from_inputs={}", hex::encode(dealt_secret_1_from_inputs.to_bytes()));
    // println!("dealt_secret_2_from_inputs={}", hex::encode(dealt_secret_2_from_inputs.to_bytes()));
    if dealt_secret_1_from_shares.sub_key().to_bytes() != dealt_secret_1_from_inputs.to_bytes() {
        return false;
    }
    if dealt_secret_2_from_shares.sub_key().to_bytes() != dealt_secret_2_from_inputs.to_bytes() {
        return false;
    }
    if dealt_secret_1_from_shares.sub_key().to_bytes()
        != dealt_secret_2_from_shares.sub_key().to_bytes()
    {
        return false;
    }
    true
}

fn dealt_secret_from_shares(
    new_dkg_state: &DKGState,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
    pvss_config: &WeightedConfig,
    trx: &WT,
) -> WeightedKey<DealtSecretKey> {
    let player_share_pairs = new_dkg_state
        .validator_set
        .as_ref()
        .unwrap()
        .active_validators
        .iter()
        .enumerate()
        .map(|(id, validator_info)| {
            let player = Player { id };
            let dk = decrypt_key_map
                .get(&validator_info.account_address)
                .unwrap();
            let (secret_key_share, _pub_key_share) =
                trx.decrypt_own_share(&pvss_config, &player, dk);
            (player, secret_key_share)
        })
        .collect();

    <WT as Transcript>::DealtSecretKey::reconstruct(&pvss_config, &player_share_pairs)
}

fn dealt_secret_from_input(
    pp: &PublicParameters,
    trx: &WT,
    old_dkg_state: &DKGState,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
) -> DealtSecretKey {
    let mut agg_secret = InputSecret::zero();
    for dealer in trx.get_dealers() {
        let addr = old_dkg_state
            .validator_set
            .as_ref()
            .unwrap()
            .active_validators[dealer.id]
            .account_address;
        let private_key = decrypt_key_map.get(&addr).unwrap();
        let seed = private_key.to_bytes_be(); // Hardcoded behavior in `aptos_consensus::dkg::dkg_manager::DKGManager::start_dkg()`.
        let mut rng = StdRng::from_seed(seed);
        let s = <WT as Transcript>::InputSecret::generate(&mut rng);
        agg_secret += &s;
    }

    let dealt_secret_from_inputs: DealtSecretKey = agg_secret.to(pp);
    dealt_secret_from_inputs
}

fn num_validators(dkg_state: &DKGState) -> usize {
    dkg_state
        .validator_set
        .as_ref()
        .unwrap()
        .active_validators
        .len()
}

fn decrypt_key_map(swarm: &LocalSwarm) -> HashMap<AccountAddress, DecryptPrivKey> {
    swarm
        .validators()
        .map(|validator| {
            let private_key = validator
                .config()
                .consensus
                .safety_rules
                .initial_safety_rules_config
                .identity_blob()
                .consensus_private_key
                .unwrap();
            let dk = DecryptPrivKey::from_bytes_be(&private_key.to_bytes());
            (validator.peer_id(), dk)
        })
        .collect::<HashMap<_, _>>()
}

mod dkg_basic;
mod dkg_with_validator_down;
mod dkg_with_validator_join_leave;
