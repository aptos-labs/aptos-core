// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus::consensus_fault_tolerance::{start_traffic, ActiveTrafficGuard},
    smoke_test_environment::SwarmBuilder,
};
use aptos_config::config::DagFetcherConfig;
use aptos_forge::{
    test_utils::consensus_utils::{
        no_failure_injection, test_consensus_fault_tolerance, FailPointFailureInjection, NodeState,
    },
    LocalSwarm, Swarm, SwarmExt,
};
use aptos_types::on_chain_config::{
    ConsensusAlgorithmConfig, DagConsensusConfigV1, OnChainConsensusConfig, ValidatorTxnConfig,
    DEFAULT_WINDOW_SIZE,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};
use std::sync::{atomic::AtomicBool, Arc};

pub async fn create_dag_swarm(num_nodes: usize) -> LocalSwarm {
    let swarm = SwarmBuilder::new_local(num_nodes)
        .with_init_config(Arc::new(move |_, config, _| {
            config.api.failpoints_enabled = true;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
            config.dag_consensus.fetcher_config = DagFetcherConfig {
                retry_interval_ms: 30,
                rpc_timeout_ms: 500,
                min_concurrent_responders: 2,
                max_concurrent_responders: 7,
                max_concurrent_fetches: 4,
            }
        }))
        .with_init_genesis_config(Arc::new(move |genesis_config| {
            let onchain_consensus_config = OnChainConsensusConfig::V4 {
                alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size: DEFAULT_WINDOW_SIZE,
            };

            genesis_config.consensus_config = onchain_consensus_config;
        }))
        .build()
        .await;

    println!(
        "Validators {:?}",
        swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>()
    );
    swarm
}

#[tokio::test]
#[ignore]
async fn test_no_failures() {
    let num_validators = 3;

    let swarm = create_dag_swarm(num_validators).await;
    let (validator_clients, public_info) = {
        (
            swarm.get_validator_clients_with_names(),
            swarm.aptos_public_info(),
        )
    };
    test_consensus_fault_tolerance(
        validator_clients,
        public_info,
        3,
        5.0,
        1,
        no_failure_injection(),
        Box::new(move |_, _, _, executed_transactions, _, _| {
            assert!(
                executed_transactions >= 4,
                "no progress with active consensus, only {} transactions",
                executed_transactions
            );
            Ok(())
        }),
        true,
        false,
    )
    .await
    .unwrap();
}

async fn run_dag_fail_point_test(
    num_validators: usize,
    cycles: usize,
    cycle_duration_s: f32,
    parts_in_cycle: usize,
    traffic_tps: f32,
    _max_block_size: u64,
    // (cycle, part) -> (Vec(validator_index, name, action), reset_old_enpoints)
    get_fail_points_to_set: Box<
        dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool) + Send,
    >,
    // (cycle, executed_epochs, executed_rounds, executed_transactions, current_state, previous_state)
    check_cycle: Box<
        dyn FnMut(usize, u64, u64, u64, Vec<NodeState>, Vec<NodeState>) -> anyhow::Result<()>
            + Send,
    >,
) {
    let mut swarm = create_dag_swarm(num_validators).await;
    let _active_traffic = if traffic_tps > 0.0 {
        start_traffic(5, traffic_tps, &mut swarm).await
    } else {
        ActiveTrafficGuard {
            finish_traffic: Arc::new(AtomicBool::new(false)),
        }
    };
    let (validator_clients, public_info) = {
        (
            swarm.get_validator_clients_with_names(),
            swarm.aptos_public_info(),
        )
    };
    test_consensus_fault_tolerance(
        validator_clients,
        public_info,
        cycles,
        cycle_duration_s,
        parts_in_cycle,
        Box::new(FailPointFailureInjection::new(get_fail_points_to_set)),
        check_cycle,
        false,
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
#[ignore]
async fn test_fault_tolerance_of_network_send() {
    // Randomly increase network failure rate, until network halts, and check that it comes back afterwards.
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 3;
    let num_cycles = 4;
    run_dag_fail_point_test(
        num_validators,
        num_cycles,
        2.5,
        5,
        1.0,
        1,
        Box::new(move |cycle, _part| {
            let max = 10 * (10 - num_cycles + cycle + 1);
            let rand: usize = small_rng.gen_range(0, 1000);
            let rand_reliability = ((rand as f32 / 1000.0).powf(0.5) * max as f32) as i32;
            let wanted_client = small_rng.gen_range(0usize, num_validators);

            (
                vec![(
                    wanted_client,
                    "consensus::send::any".to_string(),
                    format!("{}%return", rand_reliability),
                )],
                false,
            )
        }),
        Box::new(|_, _, _, _, _, _| Ok(())),
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn test_fault_tolerance_of_network_receive() {
    // Randomly increase network failure rate, until network halts, and check that it comes back afterwards.
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 3;
    let num_cycles = 4;
    run_dag_fail_point_test(
        num_validators,
        num_cycles,
        2.5,
        5,
        1.0,
        1,
        Box::new(move |cycle, _part| {
            let max = 10 * (10 - num_cycles + cycle + 1);
            let rand: usize = small_rng.gen_range(0, 1000);
            let rand_reliability = ((rand as f32 / 1000.0).powf(0.5) * max as f32) as i32;
            let wanted_client = small_rng.gen_range(0usize, num_validators);

            (
                vec![(
                    wanted_client,
                    "consensus::process::any".to_string(),
                    format!("{}%return", rand_reliability),
                )],
                false,
            )
        }),
        Box::new(|_, _, _, _, _, _| Ok(())),
    )
    .await;
}

#[tokio::test]
#[ignore]
async fn test_changing_working_consensus() {
    // with 7 nodes, consensus needs 5 to operate.
    // we rotate in each cycle, which 2 nodes are down.
    // we should consisnently be seeing progress.
    let num_validators = 7;
    run_dag_fail_point_test(
        num_validators,
        6,
        10.0,
        2,
        1.0,
        num_validators as u64,
        Box::new(move |cycle, part| {
            if part == 0 {
                let client_1 = (cycle * 2) % num_validators;
                let client_2 = (cycle * 2 + 1) % num_validators;
                (
                    vec![
                        (
                            client_1,
                            "consensus::send::any".to_string(),
                            "return".to_string(),
                        ),
                        (
                            client_1,
                            "consensus::process::any".to_string(),
                            "return".to_string(),
                        ),
                        (
                            client_2,
                            "consensus::send::any".to_string(),
                            "return".to_string(),
                        ),
                        (
                            client_2,
                            "consensus::process::any".to_string(),
                            "return".to_string(),
                        ),
                    ],
                    true,
                )
            } else {
                (vec![], false)
            }
        }),
        Box::new(|_, _, executed_rounds, executed_transactions, _, _| {
            assert!(
                executed_transactions >= 1,
                "no progress with active consensus, only {} transactions",
                executed_transactions
            );
            assert!(
                executed_rounds >= 1,
                "no progress with active consensus, only {} rounds",
                executed_rounds
            );
            Ok(())
        }),
    )
    .await;
}
