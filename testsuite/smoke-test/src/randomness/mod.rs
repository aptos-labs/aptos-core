// Copyright © Aptos Foundation

use anyhow::{anyhow, ensure, Result};
use aptos_crypto::{compat::Sha3_256, Uniform};
use aptos_dkg::{
    pvss::{
        das::PublicParameters,
        dealt_secret_key::g1::DealtSecretKey,
        encryption_dlog::g1::DecryptPrivKey,
        input_secret::InputSecret,
        traits::{Convert, Reconstructable, Transcript},
        Player, WeightedConfig,
    },
    weighted_vuf::traits::WeightedVUF,
};
use aptos_forge::LocalSwarm;
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    dkg::{build_dkg_pvss_config, DKGTranscriptWrapper, WTrx},
    on_chain_config::{BlockRandomness, DKGSessionState, DKGState, OnChainConfig, ValidatorSet},
    randomness::{RandMetadataToSign, WVUF},
    validator_verifier::ValidatorVerifier,
};
use digest::Digest;
use move_core_types::{account_address::AccountAddress, language_storage::CORE_CODE_ADDRESS};
use num_traits::Zero;
use rand::{prelude::StdRng, SeedableRng};
use std::{collections::HashMap, time::Duration};
use tokio::time::Instant;

mod dkg_basic;
mod dkg_feature_flag_flips;
mod dkg_with_validator_down;
mod dkg_with_validator_join_leave;
mod e2e_basic_consumption;
mod e2e_correctness;
mod validator_restart_during_dkg;

async fn get_current_version(rest_client: &Client) -> u64 {
    rest_client
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .version
}

async fn get_on_chain_resource<T: OnChainConfig>(rest_client: &Client) -> T {
    let maybe_response = rest_client
        .get_account_resource_bcs::<T>(CORE_CODE_ADDRESS, T::struct_tag().to_string().as_str())
        .await;
    let response = maybe_response.unwrap();
    response.into_inner()
}

async fn get_on_chain_resource_at_version<T: OnChainConfig>(
    rest_client: &Client,
    version: u64,
) -> T {
    let maybe_response = rest_client
        .get_account_resource_at_version_bcs::<T>(
            CORE_CODE_ADDRESS,
            T::struct_tag().to_string().as_str(),
            version,
        )
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
) -> Result<()> {
    info!(
        "Verifying the transcript generated for epoch {} by epoch {}.",
        dkg_session.target_epoch, dkg_session.dealer_epoch,
    );
    let verifier = ValidatorVerifier::from(&dkg_session.dealer_validator_set);
    let pvss_config =
        build_dkg_pvss_config(dkg_session.dealer_epoch, &dkg_session.target_validator_set);
    let trxs: DKGTranscriptWrapper =
        bcs::from_bytes(dkg_session.result.as_slice()).map_err(|e| {
            anyhow!("DKG transcript verification failed with transcript deserialization error: {e}")
        })?;
    trxs.verify(&pvss_config, &verifier)?;

    info!("Double-verifying by reconstructing the dealt secret.");
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

    ensure!(
        dealt_secret_from_shares == dealt_secret_from_inputs,
        "dkg transcript verification failed with final check failure"
    );
    Ok(())
}

fn dealt_secret_from_shares(
    target_validator_set: &ValidatorSet,
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
    pvss_config: &WeightedConfig,
    trx: &WTrx,
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
    trx: &WTrx,
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

/// Fetch the DKG result and the block randomness (from aggregation) for a specific version.
/// Derive the distributed secret from DKG result.
/// Verify the randomness from aggregation (the actual one store on chain) equals to
/// the randomness from direct evaluation using the distributed secret (the expected one).
async fn verify_randomness(
    decrypt_key_map: &HashMap<AccountAddress, DecryptPrivKey>,
    rest_client: &Client,
    version: u64,
) -> Result<()> {
    // Fetch resources.
    let (dkg_state, on_chain_block_randomness) = tokio::join!(
        get_on_chain_resource_at_version::<DKGState>(&rest_client, version),
        get_on_chain_resource_at_version::<BlockRandomness>(&rest_client, version)
    );

    // Derive the shared secret.
    let dkg_session = dkg_state
        .last_complete
        .ok_or_else(|| anyhow!("randomness verification failed with missing dkg result"))?;
    let pvss_config =
        build_dkg_pvss_config(dkg_session.dealer_epoch, &dkg_session.target_validator_set);
    let trxs =
        bcs::from_bytes::<DKGTranscriptWrapper>(dkg_session.result.as_slice()).map_err(|e| {
            anyhow!(
                "randomness verification failed with on-chain dkg transcript deserialization error"
            )
        })?;
    let dealt_secret = dealt_secret_from_shares(
        &dkg_session.target_validator_set,
        &decrypt_key_map,
        &pvss_config.wconfig,
        &trxs.trx,
    );

    // Compare the outputs from 2 paths.
    let rand_metadata = RandMetadataToSign {
        epoch: on_chain_block_randomness.epoch,
        round: on_chain_block_randomness.round,
    };
    let input = bcs::to_bytes(&rand_metadata).unwrap();
    let output = WVUF::eval(&dealt_secret, input.as_slice());
    let output_serialized = bcs::to_bytes(&output).unwrap();
    let expected_block_randomness = Sha3_256::digest(output_serialized.as_slice()).to_vec();

    ensure!(
        expected_block_randomness == on_chain_block_randomness.block_randomness,
        "randomness verification failed with final check failure"
    );
    Ok(())
}
