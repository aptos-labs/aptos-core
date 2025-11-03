// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{smoke_test_environment::SwarmBuilder, utils::get_on_chain_resource};
use anyhow::bail;
use aptos::common::types::GasOptions;
use aptos_config::config::{OverrideNodeConfig, PersistableConfig};
use aptos_crypto::{bls12381, Uniform};
use aptos_forge::{NodeExt, Swarm, SwarmExt};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::{
    on_chain_config::{ConfigurationResource, OnChainRandomnessConfig, ValidatorSet},
    validator_verifier::ValidatorVerifier,
};
use rand::{thread_rng, Rng};
use std::{
    fs::File,
    io::Write,
    ops::Add,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

#[tokio::test]
async fn consensus_key_rotation() {
    let epoch_duration_secs = 60;
    let n = 2;
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(n)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // Ensure randomness is enabled.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_client = swarm.validators().next().unwrap().rest_client();

    info!("Wait for epoch 3.");
    wait_until_epoch(
        &rest_client,
        3,
        Duration::from_secs(epoch_duration_secs * 2),
    )
    .await
    .unwrap();
    info!("Epoch 3 arrived.");

    let (operator_addr, new_pk, pop, operator_idx) =
        if let Some(validator) = swarm.validators_mut().nth(n - 1) {
            let operator_sk = validator
                .account_private_key()
                .as_ref()
                .unwrap()
                .private_key();
            let operator_idx = cli.add_account_to_cli(operator_sk);
            info!("Stopping the last node.");

            validator.stop();
            tokio::time::sleep(Duration::from_secs(5)).await;

            let new_identity_path = PathBuf::from(
                format!(
                    "/tmp/{}-new-validator-identity.yaml",
                    thread_rng().r#gen::<u64>()
                )
                .as_str(),
            );
            info!(
                "Generating and writing new validator identity to {:?}.",
                new_identity_path
            );
            let new_sk = bls12381::PrivateKey::generate(&mut thread_rng());
            let pop = bls12381::ProofOfPossession::create(&new_sk);
            let new_pk = bls12381::PublicKey::from(&new_sk);
            let mut validator_identity_blob = validator
                .config()
                .consensus
                .safety_rules
                .initial_safety_rules_config
                .identity_blob()
                .unwrap();
            validator_identity_blob.consensus_private_key = Some(new_sk);
            let operator_addr = validator_identity_blob.account_address.unwrap();

            Write::write_all(
                &mut File::create(&new_identity_path).unwrap(),
                serde_yaml::to_string(&validator_identity_blob)
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();

            info!("Updating the node config accordingly.");
            let config_path = validator.config_path();
            let mut validator_override_config =
                OverrideNodeConfig::load_config(config_path.clone()).unwrap();
            validator_override_config
                .override_config_mut()
                .consensus
                .safety_rules
                .initial_safety_rules_config
                .overriding_identity_blob_paths_mut()
                .push(new_identity_path);
            validator_override_config.save_config(config_path).unwrap();

            info!("Restarting the node.");
            validator.start().unwrap();
            info!("Let it bake for 5 secs.");
            tokio::time::sleep(Duration::from_secs(5)).await;
            (operator_addr, new_pk, pop, operator_idx)
        } else {
            unreachable!()
        };

    info!("Update on-chain. Retry is needed in case randomness is enabled.");
    swarm
        .chain_info()
        .into_aptos_public_info()
        .mint(operator_addr, 99999999999)
        .await
        .unwrap();
    let mut attempts = 10;
    while attempts > 0 {
        attempts -= 1;
        let gas_options = GasOptions {
            gas_unit_price: Some(100),
            max_gas: Some(200000),
            expiration_secs: 60,
        };
        let update_result = cli
            .update_consensus_key(
                operator_idx,
                None,
                new_pk.clone(),
                pop.clone(),
                Some(gas_options),
            )
            .await;
        info!("update_result={:?}", update_result);
        if let Ok(txn_smry) = update_result {
            if txn_smry.success == Some(true) {
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    assert!(attempts >= 1);

    info!("Wait for epoch 5.");
    wait_until_epoch(
        &rest_client,
        5,
        Duration::from_secs(epoch_duration_secs * 2),
    )
    .await
    .unwrap();
    info!("Epoch 5 arrived.");

    info!("All nodes should be alive.");
    let liveness_check_result = swarm
        .liveness_check(Instant::now().add(Duration::from_secs(30)))
        .await;
    assert!(liveness_check_result.is_ok());

    info!("On-chain pk should be updated.");
    let validator_set = get_on_chain_resource::<ValidatorSet>(&rest_client).await;
    let verifier = ValidatorVerifier::from(&validator_set);
    assert_eq!(new_pk, verifier.get_public_key(&operator_addr).unwrap());
}

async fn wait_until_epoch(
    rest_cli: &Client,
    target_epoch: u64,
    time_limit: Duration,
) -> anyhow::Result<()> {
    let timer = Instant::now();
    while timer.elapsed() < time_limit {
        let c = get_on_chain_resource::<ConfigurationResource>(rest_cli).await;
        if c.epoch() >= target_epoch {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    bail!("");
}
