// Copyright © Aptos Foundation

use crate::{dkg, smoke_test_environment::SwarmBuilder};
use aptos::test::CliTestFramework;
use aptos_consensus::dkg::build_dkg_pvss_config;
use aptos_forge::{Node, NodeExt, Swarm};
use aptos_rest_client::Client;
use aptos_types::{dkg::DKGTranscriptWrapper, validator_verifier::ValidatorVerifier};
use std::{collections::HashSet, sync::Arc};

#[tokio::test]
async fn dkg_with_validator_set_change() {
    let epoch_duration_secs = 30;
    let estimated_dkg_latency_secs = 40;
    let time_limit_secs = epoch_duration_secs + estimated_dkg_latency_secs;

    let mut swarm = SwarmBuilder::new_local(7)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
        }))
        .build()
        .await;

    println!("Wait for a moment when DKG is not running.");
    let client: Client = swarm.validators().next().unwrap().rest_client();
    let dkg_state_1 = dkg::wait_for_epoch_fully_entered(&client, None, time_limit_secs).await;
    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_state_1.target_epoch,
        dkg_state_1
            .validator_set
            .as_ref()
            .unwrap()
            .active_validators
            .len()
    );
    println!(
        "Wait until we fully entered epoch {}.",
        dkg_state_1.target_epoch + 1
    );
    let dkg_state_2 = dkg::wait_for_epoch_fully_entered(
        &client,
        Some(dkg_state_1.target_epoch + 1),
        time_limit_secs,
    )
    .await;
    let num_validators_in_epoch_2 = dkg_state_2
        .validator_set
        .as_ref()
        .unwrap()
        .active_validators
        .len();
    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_state_2.target_epoch, num_validators_in_epoch_2
    );
    let dkg_addr_set = dkg_state_2
        .validator_set
        .as_ref()
        .unwrap()
        .active_validators
        .iter()
        .map(|v| v.account_address)
        .collect::<HashSet<_>>();
    let swarm_addr_set = swarm
        .validators()
        .map(|v| v.peer_id())
        .collect::<HashSet<_>>();
    assert_eq!(dkg_addr_set, swarm_addr_set);

    println!("Letting one of the validators leave.");
    let (victim_validator_sk, victim_validator_addr, victim_validator_endpoint) = {
        let victim_validator = swarm.validators().next().unwrap();
        let sk = victim_validator
            .account_private_key()
            .clone()
            .unwrap()
            .private_key();
        let addr = victim_validator.peer_id();
        let endpoint = victim_validator.rest_api_endpoint();
        (sk, addr, endpoint)
    };

    println!("Give the victim some money so it can at least request to leave.");
    let mut public_info = swarm.chain_info().into_aptos_public_info();
    public_info
        .mint(victim_validator_addr, 1000000000000)
        .await
        .unwrap();

    println!("Send the txn to request leave.");
    let faucet_endpoint: reqwest::Url = "http://localhost:8081".parse().unwrap();
    let mut cli = CliTestFramework::new(
        victim_validator_endpoint,
        faucet_endpoint,
        /*num_cli_accounts=*/ 0,
    )
    .await;
    let idx = cli.add_account_to_cli(victim_validator_sk);
    let txn_result = cli.leave_validator_set(idx, None).await.unwrap();
    println!("Txn result: {:?}", txn_result);

    println!(
        "Wait until we fully entered epoch {}.",
        dkg_state_2.target_epoch + 1
    );
    let dkg_state_3 = dkg::wait_for_epoch_fully_entered(
        &client,
        Some(dkg_state_2.target_epoch + 1),
        time_limit_secs,
    )
    .await;
    let num_validators_in_epoch_3 = dkg_state_3
        .validator_set
        .as_ref()
        .unwrap()
        .active_validators
        .len();
    println!(
        "Current epoch is {}. Number of validators: {}.",
        dkg_state_3.target_epoch, num_validators_in_epoch_3
    );
    assert_eq!(num_validators_in_epoch_3, num_validators_in_epoch_2 - 1);
    println!(
        "Verifying the transcript generated for epoch {} by epoch {}.",
        dkg_state_3.target_epoch, dkg_state_2.target_epoch
    );
    let verifier = ValidatorVerifier::from(dkg_state_2.validator_set.as_ref().unwrap());
    let (_, pvss_config) = build_dkg_pvss_config(
        dkg_state_2.target_epoch,
        dkg_state_3.validator_set.as_ref().unwrap(),
    );
    let trxs: DKGTranscriptWrapper =
        bcs::from_bytes(dkg_state_3.serialized_transcript.as_slice()).unwrap();
    assert!(trxs.verify(&pvss_config, &verifier).is_ok());
}
