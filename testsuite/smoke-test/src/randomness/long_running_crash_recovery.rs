// Copyright © Aptos Foundation

use std::sync::Arc;
use std::time::{Duration, Instant};
use rand::{Rng, thread_rng};
use aptos::test::CliTestFramework;
use aptos_forge::{LocalSwarm, Node, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_types::on_chain_config::{OnChainRandomnessConfig, RandomnessConfigMoveStruct};
use crate::randomness::{get_current_version, get_on_chain_resource, get_on_chain_resource_at_version, script_to_enable_main_logic};
use crate::smoke_test_environment::SwarmBuilder;
use crate::utils::get_current_consensus_config;

#[tokio::test]
async fn long_running_crash_recovery() {
    let epoch_duration_secs = 20;

    let (mut swarm, mut aptos_cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;
            conf.allow_new_validators = true;

            // start with vtxn disabled and randomness off.
            conf.consensus_config.disable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_disabled());
        }))
        .build_with_cli(0)
        .await;

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = aptos_cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    // We will not touch node 0 and make it the R/W gateway.
    let client_endpoint = swarm.validators().nth(0).unwrap().rest_api_endpoint();
    let rest_cli = Client::new(client_endpoint.clone());

    let mut rng = thread_rng();
    let mut validator_power_status_vec = vec![true; 4];
    let mut num_iterations = 0;
    loop {
        let root_balance = rest_cli.get_account_balance(root_addr).await.unwrap().into_inner();
        println!("root_balance={:?}", root_balance);
        print_current_state(num_iterations, &rest_cli, &validator_power_status_vec).await;
        make_change(&mut validator_power_status_vec, &rest_cli, &aptos_cli, root_idx, &mut swarm).await;
        let sleep_sec = rng.gen_range(5, 30);
        println!("Acted. Sleeping for {} secs.", sleep_sec);
        println!();
        tokio::time::sleep(Duration::from_secs(sleep_sec)).await;
        assert_state(&validator_power_status_vec, &mut swarm).await;
        num_iterations += 1;
    }
}

async fn print_current_state(num_iterations: usize, client: &Client, validator_power_status_vec: &Vec<bool>) {
    let (consensus_config, randomness_config, index) = tokio::join!(
        get_current_consensus_config(client),
        get_on_chain_resource::<RandomnessConfigMoveStruct>(client),
        client.get_index(),
    );
    let index = index.unwrap().into_inner();
    println!("num_iterations={}", num_iterations);
    println!("current_time={:?}", Instant::now());
    println!("epoch={}, block_height={}, version={}", index.epoch, index.block_height, index.ledger_version);
    println!("vtxn_enabled={}", consensus_config.is_vtxn_enabled());
    println!("randomness_config={}", randomness_config.variant.type_name);
    println!("validator_power_status_vec={:?}", validator_power_status_vec);
    println!();
}

async fn start_random_node(validator_power_status_vec: &mut Vec<bool>, swarm: &mut LocalSwarm) {
    let stopped_nodes: Vec<usize> = validator_power_status_vec.iter().enumerate().filter_map(|(idx, &power_on)|if !power_on {Some(idx)} else {None}).collect();
    let i = stopped_nodes[thread_rng().gen_range(0, stopped_nodes.len())];
    println!("Action: start node {}", i);
    println!();
    swarm.validators_mut().nth(i).unwrap().start().unwrap();
    validator_power_status_vec[i] = true;
}

async fn stop_random_node(validator_power_status_vec: &mut Vec<bool>, swarm: &mut LocalSwarm) {
    let started_nodes: Vec<usize> = validator_power_status_vec.iter().enumerate().filter_map(|(idx, &power_on)|if power_on {Some(idx)} else {None}).collect();
    let i = started_nodes[thread_rng().gen_range(1, started_nodes.len())]; // Ensure node 0 is never stopped.
    println!("Action: stop node {}", i);
    println!();
    swarm.validators_mut().nth(i).unwrap().stop();
    validator_power_status_vec[i] = false;
}

async fn set_random_vtxn_and_randomness_config(rest_cli: &Client, aptos_cli: &CliTestFramework, root_idx: usize) {
    let target_vtxn_status = thread_rng().gen_range(1, 2);
    let target_randomness_status = thread_rng().gen_range(0, 3);
    println!("Action: set target_vtxn_status={}, target_randomness_status={}", target_vtxn_status, target_randomness_status);
    println!();

    let mut consensus_config = get_current_consensus_config(rest_cli).await;
    if target_vtxn_status == 0 {
        consensus_config.disable_validator_txns();
    } else {
        consensus_config.enable_validator_txns();
    }
    let consensus_config_bytes = bcs::to_bytes(&consensus_config).unwrap();
    let script = format!(r#"
script {{
    use aptos_framework::aptos_governance;
    use aptos_framework::consensus_config;
    use aptos_framework::randomness_config;
    use aptos_std::fixed_point64;

    fun main(core_resources: &signer) {{
        let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0x1);
        let consensus_config_bytes = vector{:?};
        consensus_config::set_for_next_epoch(&framework_signer, consensus_config_bytes);
        let target_randomness_status = {};
        let randomness_config = if (target_randomness_status == 0) {{
            randomness_config::new_off()
        }} else if (target_randomness_status == 1) {{
            randomness_config::new_v1(
                fixed_point64::create_from_rational(1, 2),
                fixed_point64::create_from_rational(2, 3)
            )
        }} else {{
            randomness_config::new_v2(
                fixed_point64::create_from_rational(1, 2),
                fixed_point64::create_from_rational(2, 3),
                fixed_point64::create_from_rational(2, 3),
            )
        }};
        randomness_config::set_for_next_epoch(&framework_signer, randomness_config);
        aptos_governance::reconfigure(&framework_signer);
    }}
}}
"#, consensus_config_bytes, target_randomness_status);
    let txn_result = aptos_cli.run_script(root_idx, script.as_str()).await;
    println!("txn_result={:?}", txn_result);
    assert!(txn_result.unwrap().success.unwrap());
}

async fn make_change(validator_power_status_vec: &mut Vec<bool>, rest_cli: &Client, aptos_cli: &CliTestFramework, root_idx: usize, swarm: &mut LocalSwarm) {
    let stopped_nodes: Vec<usize> = validator_power_status_vec.iter().enumerate().filter_map(|(idx, &power_on)|if !power_on {Some(idx)} else {None}).collect();
    match stopped_nodes.len() {
        0 => {
            match thread_rng().gen_range(0.0, 1.0) {
                x if x < 0.5 => set_random_vtxn_and_randomness_config(rest_cli, aptos_cli, root_idx).await,
                _ => stop_random_node(validator_power_status_vec, swarm).await,
            }
        },
        1 => {
            match thread_rng().gen_range(0.0, 1.0) {
                x if x < 0.5 => set_random_vtxn_and_randomness_config(rest_cli, aptos_cli, root_idx).await,
                x if x >= 0.5 && x < 0.75 => stop_random_node(validator_power_status_vec, swarm).await,
                _ => start_random_node(validator_power_status_vec, swarm).await,
            }
        },
        2 => {
            match thread_rng().gen_range(0.0, 1.0) {
                x if x < 0.8 => start_random_node(validator_power_status_vec, swarm).await,
                _ => stop_random_node(validator_power_status_vec, swarm).await,
            }
        }
        3 => start_random_node(validator_power_status_vec, swarm).await,
        _ => unreachable!()
    }
}

async fn assert_state(validator_power_status_vec: &Vec<bool>, swarm: &mut LocalSwarm) {
    for (idx, &power_status) in validator_power_status_vec.iter().enumerate() {
        if power_status {
            let health_check_result = swarm.validators_mut().nth(idx).unwrap().health_check().await;
            if health_check_result.is_err() {
                println!("node {} is supposed to be on!", idx);
                assert!(false);
            }
        }
    }
}
