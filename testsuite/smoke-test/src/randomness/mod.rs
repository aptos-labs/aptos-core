// Copyright © Aptos Foundation

use aptos_rest_client::Client;
use aptos_types::on_chain_config::{DKGSessionState, DKGState, OnChainConfig, ValidatorSet};
use tokio::time::Instant;
use std::time::Duration;
use aptos_dkg::pvss::{das, Player, WeightedConfig};
use aptos_types::validator_verifier::ValidatorVerifier;
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::collections::HashMap;
use aptos_dkg::pvss::das::PublicParameters;
use aptos_dkg::pvss::encryption_dlog::g1::DecryptPrivKey;
use aptos_dkg::pvss::input_secret::InputSecret;
use aptos_dkg::pvss::traits::{Convert, Reconstructable, Transcript};
use aptos_dkg::pvss::dealt_secret_key::g1::DealtSecretKey;
use aptos_forge::LocalSwarm;
use aptos_types::dkg::{build_dkg_pvss_config, DKGTranscriptWrapper, WTrx};
use move_core_types::account_address::AccountAddress;
use rand::prelude::StdRng;
use num_traits::Zero;
use rand::SeedableRng;
use aptos_crypto::Uniform;

mod e2e_basic_consumption;
mod dkg_basic;
mod dkg_feature_flag_flips;
mod dkg_with_validator_down;
mod dkg_with_validator_join_leave;
mod validator_restart_during_dkg;

type WT = das::weighted_transcript::Transcript;

async fn get_on_chain_resource<T: OnChainConfig>(rest_client: &Client) -> T {
    let maybe_response = rest_client
        .get_account_resource_bcs::<T>(CORE_CODE_ADDRESS, T::struct_tag().to_string().as_str())
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}

/// Poll the on-chain state until we see a DKG session finishes.
/// Return a `DKGSessionState` of the DKG session seen.
async fn wait_for_dkg_finish(
    client: &Client,
    target_epoch: Option<u64>,
    time_limit_secs: u64,
) -> DKGSessionState {
    let mut dkg_state = get_on_chain_resource::<DKGState>(client).await;
    let timer = Instant::now();
    while timer.elapsed().as_secs() < time_limit_secs
        && !(dkg_state.in_progress.is_none()
            && dkg_state.last_complete.is_some()
            && (target_epoch.is_none()
                || dkg_state
                    .last_complete
                    .as_ref()
                    .map(|session| session.target_epoch)
                    == target_epoch))
    {
        std::thread::sleep(Duration::from_secs(1));
        dkg_state = get_on_chain_resource::<DKGState>(client).await;
    }
    assert!(timer.elapsed().as_secs() < time_limit_secs);
    dkg_state.last_complete().clone()
}

/// Verify that DKG transcript of epoch i (stored in `new_dkg_state`) is correctly generated
/// by the validator set in epoch i-1 (stored in `new_dkg_state`).
fn verify_dkg_transcript(
    dkg_session: &DKGSessionState,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
) -> bool {
    println!(
        "Verifying the transcript generated for epoch {} by epoch {}.",
        dkg_session.target_epoch, dkg_session.dealer_epoch,
    );
    let verifier = ValidatorVerifier::from(&dkg_session.dealer_validator_set);
    let pvss_config =
        build_dkg_pvss_config(dkg_session.dealer_epoch, &dkg_session.target_validator_set);
    let trxs: DKGTranscriptWrapper = bcs::from_bytes(dkg_session.result.as_slice()).unwrap();
    if trxs.verify(&pvss_config, &verifier).is_err() {
        return false;
    }

    println!("Double-verifying by reconstructing the dealt secret.");
    let dealt_secret_from_shares = dealt_secret_from_shares(
        &dkg_session.target_validator_set,
        decrypt_key_map,
        &pvss_config.wconfig,
        &trxs.trx,
    );
    let dealt_secret_from_inputs = dealt_secret_from_input(
        &pvss_config.pp,
        &trxs.trx,
        &dkg_session.dealer_validator_set,
        decrypt_key_map,
    );

    if dealt_secret_from_shares != dealt_secret_from_inputs {
        return false;
    }
    true
}

fn dealt_secret_from_shares(
    target_validator_set: &ValidatorSet,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
    pvss_config: &WeightedConfig,
    trx: &WT,
) -> DealtSecretKey {
    let x = ValidatorVerifier::from(target_validator_set);
    let player_share_pairs = x
        .get_ordered_account_addresses()
        .iter()
        .enumerate()
        .map(|(id, validator_addr)| {
            let player = Player { id };
            let dk = decrypt_key_map.get(validator_addr).unwrap();
            let (secret_key_share, _pub_key_share) =
                trx.decrypt_own_share(pvss_config, &player, dk);
            (player, secret_key_share)
        })
        .collect();

    <WTrx as Transcript>::DealtSecretKey::reconstruct(pvss_config, &player_share_pairs)
}

fn dealt_secret_from_input(
    pp: &PublicParameters,
    trx: &WT,
    dealer_validator_set: &ValidatorSet,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
) -> DealtSecretKey {
    let mut agg_secret = InputSecret::zero();
    let x = ValidatorVerifier::from(dealer_validator_set);
    let validator_addrs = x.get_ordered_account_addresses();
    for dealer in trx.get_dealers() {
        let private_key = decrypt_key_map.get(&validator_addrs[dealer.id]).unwrap();
        let seed = private_key.to_bytes_be(); // Hardcoded behavior in `aptos_consensus::dkg::dkg_manager::DKGManager::start_dkg()`.
        let mut rng = StdRng::from_seed(seed);
        let s = <WTrx as Transcript>::InputSecret::generate(&mut rng);
        agg_secret += &s;
    }

    // <InputSecret as Convert<aptos_dkg::pvss::dealt_secret_key::g::DealtSecretKey, PublicParameters>>::to(&agg_secret, pp)
    let dealt_secret_from_inputs: DealtSecretKey = agg_secret.to(pp);
    dealt_secret_from_inputs
}

fn num_validators(dkg_state: &DKGSessionState) -> usize {
    ValidatorVerifier::from(&dkg_state.target_validator_set).len()
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
