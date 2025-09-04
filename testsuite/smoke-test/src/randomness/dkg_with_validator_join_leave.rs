// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    randomness::{decrypt_key_map, num_validators, verify_dkg_transcript, wait_for_dkg_finish},
    smoke_test_environment::SwarmBuilder,
};
use velor::test::CliTestFramework;
use velor_forge::{Node, Swarm};
use velor_types::on_chain_config::OnChainRandomnessConfig;
use std::sync::Arc;

#[tokio::test]
async fn dkg_with_validator_join_leave() {
    let epoch_duration_secs = 40;
    let estimated_dkg_latency_secs = 80;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let swarm = SwarmBuilder::new_local(7)
        .with_num_fullnodes(1)
        .with_velor()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // Ensure randomness is enabled.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build()
        .await;

    let decrypt_key_map = decrypt_key_map(&swarm);

    println!("Wait for a moment when DKG is not running.");
    let client_endpoint = swarm.validators().nth(1).unwrap().rest_api_endpoint();
    let client = velor_rest_client::Client::new(client_endpoint.clone());
    let dkg_session_1 = wait_for_dkg_finish(&client, None, time_limit_secs).await;
    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_session_1.target_epoch(),
        num_validators(&dkg_session_1)
    );

    println!(
        "Wait until we fully entered epoch {}.",
        dkg_session_1.target_epoch() + 1
    );
    let dkg_session_2 = wait_for_dkg_finish(
        &client,
        Some(dkg_session_1.target_epoch() + 1),
        time_limit_secs,
    )
    .await;

    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_session_2.target_epoch(),
        num_validators(&dkg_session_2)
    );

    println!("Letting one of the validators leave.");
    let (victim_validator_sk, victim_validator_addr) = {
        let victim_validator = swarm.validators().next().unwrap();
        let sk = victim_validator
            .account_private_key()
            .clone()
            .unwrap()
            .private_key();
        let addr = victim_validator.peer_id();
        (sk, addr)
    };

    println!("Give the victim some money so it can first send transactions.");
    let mut public_info = swarm.chain_info().into_velor_public_info();
    public_info
        .mint(victim_validator_addr, 100000000000000)
        .await
        .unwrap();

    println!("Send the txn to request leave.");
    let faucet_endpoint: reqwest::Url = "http://localhost:8081".parse().unwrap();
    let mut cli = CliTestFramework::new(
        client_endpoint,
        faucet_endpoint,
        /*num_cli_accounts=*/ 0,
    )
    .await;
    let idx = cli.add_account_to_cli(victim_validator_sk);
    let txn_result = cli.leave_validator_set(idx, None).await.unwrap();
    println!("Txn result: {:?}", txn_result);

    println!(
        "Wait until we fully entered epoch {}.",
        dkg_session_2.target_epoch() + 1
    );
    let dkg_session_3 = wait_for_dkg_finish(
        &client,
        Some(dkg_session_2.target_epoch() + 1),
        time_limit_secs,
    )
    .await;

    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_session_3.target_epoch(),
        num_validators(&dkg_session_3)
    );

    assert!(verify_dkg_transcript(&dkg_session_3, &decrypt_key_map).is_ok());
    assert_eq!(
        num_validators(&dkg_session_3),
        num_validators(&dkg_session_2) - 1
    );

    println!("Now re-join.");
    let txn_result = cli.join_validator_set(idx, None).await;
    println!("Txn result: {:?}", txn_result);
    println!(
        "Wait until we fully entered epoch {}.",
        dkg_session_3.target_epoch() + 1
    );
    let dkg_session_4 = wait_for_dkg_finish(
        &client,
        Some(dkg_session_3.target_epoch() + 1),
        time_limit_secs,
    )
    .await;

    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_session_4.target_epoch(),
        num_validators(&dkg_session_4)
    );

    assert!(verify_dkg_transcript(&dkg_session_4, &decrypt_key_map).is_ok());
    assert_eq!(
        num_validators(&dkg_session_4),
        num_validators(&dkg_session_3) + 1
    );
}
