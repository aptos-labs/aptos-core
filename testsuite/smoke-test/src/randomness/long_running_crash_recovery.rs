// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Copyright © Aptos Foundation

use std::collections::HashSet;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use diesel::internal::derives::multiconnection::chrono::NaiveDateTime;
use rand::{Rng, thread_rng};
use aptos::common::types::GasOptions;
use aptos::move_tool::MemberId;
use aptos::node::ValidatorConfig;
use aptos::test::CliTestFramework;
use aptos_forge::{LocalSwarm, Node, NodeExt, Swarm, SwarmExt};
use aptos_logger::{debug, info};
use aptos_rest_client::Client;
use aptos_types::on_chain_config::{OnChainRandomnessConfig, RandomnessConfigMoveStruct, ValidatorTxnConfig};
use crate::randomness::{get_current_version, get_on_chain_resource, get_on_chain_resource_at_version, script_to_enable_main_logic};
use crate::randomness::e2e_basic_consumption::publish_on_chain_dice_module;
use crate::smoke_test_environment::SwarmBuilder;
use crate::utils::get_current_consensus_config;

#[derive(Clone, Debug)]
enum Fault {
    MaulOutgoingMsgs,
    DropIncomingMsgs,
}

#[derive(Clone, Debug)]
enum NodeState {
    PowerOn(Vec<Fault>),
    PowerOff,
}

impl NodeState {
    fn is_normal(&self) -> bool {
        match self {
            NodeState::PowerOn(faults) => {
                faults.is_empty()
            }
            NodeState::PowerOff => false,
        }
    }
}

impl Default for NodeState {
    fn default() -> Self {
        Self::PowerOn(vec![])
    }
}

#[tokio::test]
async fn long_running_crash_recovery() {
    let epoch_duration_secs = 20;
    let dkg_secs = 20;
    let (mut swarm, mut aptos_cli, _faucet) = SwarmBuilder::new_local(4)
        .with_num_fullnodes(1)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |conf| {
            conf.epoch_duration_secs = epoch_duration_secs;

            // start with vtxn disabled and randomness off.
            conf.consensus_config.enable_validator_txns();
            conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled());
        }))
        .build_with_cli(0)
        .await;

    let rest_clis: Vec<Client> =
        swarm.validators().map(|node| node.rest_client()).collect();

    swarm
        .wait_for_all_nodes_to_catchup_to_epoch(2, Duration::from_secs(epoch_duration_secs + dkg_secs + 5))
        .await
        .expect("Waited too long for epoch 3.");

    let root_addr = swarm.chain_info().root_account().address();
    let root_idx = aptos_cli.add_account_with_address_to_cli(swarm.root_key(), root_addr);

    info!("Publishing dice module.");
    publish_on_chain_dice_module(&mut aptos_cli, 0).await;

    let mut rng = thread_rng();
    let mut node_states = vec![NodeState::default(); 4];
    let mut num_iterations = 0;
    info!("Entering loop.");
    loop {
        let rand_txn_can_work = print_current_state(num_iterations, &rest_clis[0], &node_states).await;
        if rand_txn_can_work {
            info!("Randomness works, sleep and roll.");
            tokio::time::sleep(Duration::from_secs(epoch_duration_secs + dkg_secs + 5)).await; // In case we need a DKG.
            roll_dice(&aptos_cli).await;
        }
        make_change(&mut node_states, rest_clis.as_slice(), &aptos_cli, root_idx, &mut swarm).await;
        let sleep_sec = rng.gen_range(5, 30);
        info!("Acted. Sleeping for {} secs.", sleep_sec);
        println!();
        tokio::time::sleep(Duration::from_secs(sleep_sec)).await;
        assert_state(&node_states, &mut swarm).await;
        num_iterations += 1;
    }
}

/// Also return whether randomness txn can work.
async fn print_current_state(num_iterations: usize, client: &Client, node_states: &Vec<NodeState>) -> bool {
    let (consensus_config, randomness_config, index) = tokio::join!(
        get_current_consensus_config(client),
        get_on_chain_resource::<RandomnessConfigMoveStruct>(client),
        client.get_index(),
    );
    let index = index.unwrap().into_inner();
    let current_time = SystemTime::now();
    let seconds_since_epoch = current_time.duration_since(UNIX_EPOCH).unwrap().as_secs();
    let datetime = NaiveDateTime::from_timestamp_opt(seconds_since_epoch as i64, 0).unwrap();
    println!();
    println!("{}", datetime.format("%Y-%m-%d %H:%M:%S"));
    println!("num_iterations={}", num_iterations);
    println!("current_time={:?}", Instant::now());
    println!("epoch={}, block_height={}, version={}", index.epoch, index.block_height, index.ledger_version);
    println!("vtxn_enabled={}", consensus_config.is_vtxn_enabled());
    println!("randomness_config={}", randomness_config.variant.type_name);
    println!("node_states={:?}", node_states);
    println!();
    let normal_nodes: Vec<NodeState> = node_states.clone().into_iter().filter(NodeState::is_normal).collect();
    let randomness_config = OnChainRandomnessConfig::try_from(randomness_config).unwrap();
    consensus_config.is_vtxn_enabled() && randomness_config.randomness_enabled() && normal_nodes.len() >= 3
}

async fn cleanse_random_node(node_states: &mut Vec<NodeState>, swarm: &mut LocalSwarm) {
    let abnormal_nodes: Vec<usize> = node_states.iter().enumerate().filter_map(|(idx, node_state)|if !node_state.is_normal() {Some(idx)} else {None}).collect();
    let i = abnormal_nodes[thread_rng().gen_range(0, abnormal_nodes.len())];
    info!("Action: cleanse node {}", i);
    println!();
    swarm.validators_mut().nth(i).unwrap().restart().await.unwrap();
    node_states[i] = NodeState::default();
}

async fn break_random_node(node_states: &mut Vec<NodeState>, swarm: &mut LocalSwarm, rest_clis: &[Client]) {
    let normal_node_indices: Vec<usize> = node_states.iter().enumerate().filter_map(|(idx, node_state)|if node_state.is_normal() {Some(idx)} else {None}).collect();
    let i = normal_node_indices[thread_rng().gen_range(1, normal_node_indices.len())]; // Ensure node 0 is always normal.
    let sample = thread_rng().gen_range(0.0, 1.0);
    if sample < 0.25 {
        println!("Action: inject fault DropIncomingMsgs to node {}", i);
        println!();
        let set_failpoint_result = rest_clis[i].set_failpoint("consensus::drop_incoming_msgs".to_string(), "return".to_string()).await;
        println!("set_failpoint_result={:?}", set_failpoint_result);
        println!();
        node_states[i] = NodeState::PowerOn(vec![Fault::DropIncomingMsgs]);
    } else if sample < 0.5 {
        println!("Action: inject fault MaulOutgoingMsgs to node {}", i);
        println!();
        let set_failpoint_result = rest_clis[i].set_failpoint("network::maul_outgoing_msgs".to_string(), "return".to_string()).await;
        println!("set_failpoint_result={:?}", set_failpoint_result);
        println!();
        node_states[i] = NodeState::PowerOn(vec![Fault::MaulOutgoingMsgs]);
    } else if sample < 0.75 {
        println!("Action: inject faults MaulOutgoingMsgs+DropIncomingMsgs to node {}", i);
        println!();
        let task1 = rest_clis[i].set_failpoint("consensus::drop_incoming_msgs".to_string(), "return".to_string());
        let task2 = rest_clis[i].set_failpoint("network::maul_outgoing_msgs".to_string(), "return".to_string());
        let set_failpoint_results = futures::future::join_all(vec![task1, task2]).await;
        println!("set_failpoint_results={:?}", set_failpoint_results);
        println!();
        node_states[i] = NodeState::PowerOn(vec![Fault::MaulOutgoingMsgs]);
    } else {
        println!("Action: power-off node {}", i);
        println!();
        swarm.validators_mut().nth(i).unwrap().stop();
        node_states[i] = NodeState::PowerOff;
    }

}

async fn set_random_vtxn_and_randomness_config(rest_cli: &Client, aptos_cli: &CliTestFramework, root_idx: usize) {
    let target_vtxn_status = thread_rng().gen_range(1, 2);
    let target_randomness_status = thread_rng().gen_range(0, 3);
    info!("Action: set target_vtxn_status={}, target_randomness_status={}", target_vtxn_status, target_randomness_status);
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
    let gas_options = GasOptions {
        gas_unit_price: Some(1),
        max_gas: Some(200000),
        expiration_secs: 60,
    };
    let txn_result = aptos_cli.run_script_with_gas_options(root_idx, script.as_str(), Some(gas_options)).await;
    info!("txn_result={:?}", txn_result);
    assert!(txn_result.unwrap().success.unwrap());
}

async fn make_change(node_states: &mut Vec<NodeState>, rest_clis: &[Client], aptos_cli: &CliTestFramework, root_idx: usize, swarm: &mut LocalSwarm) {
    let abnormal_nodes: Vec<usize> = node_states.iter().enumerate().filter_map(|(idx, node_state)|if !node_state.is_normal() {Some(idx)} else {None}).collect();
    let sample = thread_rng().gen_range(0.0, 1.0);
    match abnormal_nodes.len() {
        0 => {
            if sample < 0.4 {
                set_random_vtxn_and_randomness_config(&rest_clis[0], aptos_cli, root_idx).await;
            } else {
                break_random_node(node_states, swarm, rest_clis).await;
            }
        },
        1 => {
            if sample < 0.5 {
                set_random_vtxn_and_randomness_config(&rest_clis[0], aptos_cli, root_idx).await;
            } else if sample < 0.75 {
                break_random_node(node_states, swarm, rest_clis).await;
            } else {
                cleanse_random_node(node_states, swarm).await;
            }
        },
        2 => {
            if sample < 0.8 {
                cleanse_random_node(node_states, swarm).await;
            } else {
                break_random_node(node_states, swarm, rest_clis).await;
            }
        }
        3 => cleanse_random_node(node_states, swarm).await,
        _ => unreachable!()
    }
}

async fn assert_state(node_states: &Vec<NodeState>, swarm: &mut LocalSwarm) {
    let mut num_normal_nodes = 0;
    for (idx, node_state) in node_states.iter().enumerate() {
        if node_state.is_normal() {
            num_normal_nodes += 1;
            let node = swarm.validators_mut().nth(idx).unwrap();
            let health_check_result = node.health_check().await;
            if health_check_result.is_err() {
                println!("node {} is supposed to be on!", idx);
                assert!(false);
            }
        }
    }
}

async fn roll_dice(aptos_cli: &CliTestFramework) {
    let gas_options = GasOptions {
        gas_unit_price: Some(1),
        max_gas: Some(2000000),
        expiration_secs: 60,
    };
    let account = aptos_cli.account_id(0).to_hex_literal();
    let roll_func_id = MemberId::from_str(&format!("{}::dice::roll", account)).unwrap();
    let txn_summary = aptos_cli
        .run_function(0, Some(gas_options), roll_func_id.clone(), vec![], vec![])
        .await
        .unwrap();
    info!("Roll txn summary: {:?}", txn_summary);
    println!();
}
