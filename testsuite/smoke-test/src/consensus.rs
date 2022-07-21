// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use core::time;
use std::{collections::HashSet, sync::Arc, thread, time::Instant};

use crate::{
    smoke_test_environment::SwarmBuilder,
    test_utils::{create_and_fund_account, transfer_coins, transfer_coins_non_blocking},
};
use aptos_rest_client::{Client as RestClient, Transaction};
use forge::{NodeExt, Swarm};
use futures::future::join_all;
use rand::{self, Rng};
use rand::{rngs::SmallRng, SeedableRng};

#[derive(Clone, Debug)]
struct NodeState {
    pub version: u64,
    pub epoch: u64,
    pub round: u64,
}

// TODO: check if we can fetch consensus round, not just committed round.
async fn get_node_state(validator_client: &RestClient) -> NodeState {
    let (transactions, state) = validator_client
        .get_transactions(None, Some(3))
        .await
        .unwrap()
        .into_parts();

    let mut round = 0;
    for t in transactions {
        if let Transaction::BlockMetadataTransaction(metadata) = t {
            round = metadata.round.into();
        }
    }
    NodeState {
        version: state.version,
        epoch: state.epoch,
        round,
    }
}

/// Run a test, where we spin up a set of validators, and then
/// check a reliability scenario.
/// After scenario finishes, we always return reliability to 100%,
/// and we confirm that chain recouperates, makes progress, and
/// all nodes agree on ledger.
///
/// Scenario is performed via two nested loops:
/// outer cycles, and within each cycle over parts.
/// Scenario can specify failpoint changes on every part,
/// and can check the performance of the network after every cycle.
///
/// Transaction can be inserted on every part, to control the throughput.
/// I.e. if part is shorter than how long it takes for empty block to be
/// generated, we can make sure one block gets created on every part.
async fn run_fail_point_test(
    num_nodes: usize,
    cycles: usize,
    cycle_duration_s: f32,
    parts_in_cycle: usize,
    insert_transaction_every_part: bool,
    // (cycle, part) -> (Vec(validator_index, name, action), reset_old_enpoints)
    mut get_fail_points_to_set: Box<
        dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool),
    >,
    // (cycle, executed_rounds, executed_transactions, current_state, previous_state)
    mut check_cycle: Box<dyn FnMut(usize, u64, u64, Vec<NodeState>, Vec<NodeState>)>,
) {
    let mut swarm = SwarmBuilder::new_local(num_nodes)
        .with_init_config(Arc::new(|_, config, _| {
            config.api.failpoints_enabled = true;
            config.consensus.max_block_size = 1;
            config.consensus.round_initial_timeout_ms = 1000;
            // no increase in timeout, to have stable round/second rate.
            config.consensus.round_timeout_backoff_exponent_base = 1.0;
            // empty block generated automatically every 600ms
            config.consensus.quorum_store_poll_count = 20;
        }))
        .build()
        .await;

    let mut validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    validator_peer_ids.sort();
    println!("Swarm started for dir {}", swarm.dir().to_string_lossy());
    println!("Validators {:?}", validator_peer_ids);

    let validator_clients: Vec<RestClient> = validator_peer_ids
        .iter()
        .map(|validator| swarm.validator(*validator).unwrap().rest_client())
        .collect();
    let validator_client_0 = &validator_clients.get(0).unwrap();

    async fn get_all_states(validator_clients: &[RestClient]) -> Vec<NodeState> {
        join_all(
            validator_clients
                .iter()
                .cloned()
                .map(move |v| async move { get_node_state(&v).await }),
        )
        .await
    }

    let transaction_factory = swarm.chain_info().transaction_factory();
    let mut account_0 = create_and_fund_account(&mut swarm, 100000).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    let mut modified_failpoints: HashSet<(usize, String)> = HashSet::new();

    let mut small_rng = SmallRng::from_entropy();

    let mut previous = get_all_states(&validator_clients).await;
    for cycle in 0..cycles {
        let now = Instant::now();
        for part in 0..parts_in_cycle {
            let (fail_points_to_set, reset_old_failpoints) = get_fail_points_to_set(cycle, part);
            if reset_old_failpoints {
                let actions = "off".to_string();
                for (validator_idx, name) in modified_failpoints {
                    // println!("Setting client {} failpoint {}={}", validator_idx, name, actions);
                    validator_clients[validator_idx]
                        .set_failpoint(name, actions.clone())
                        .await
                        .unwrap();
                }
                modified_failpoints = HashSet::new();
            }
            for (validator_idx, name, actions) in fail_points_to_set {
                validator_clients[validator_idx]
                    .set_failpoint(name.clone(), actions.clone())
                    .await
                    .unwrap();
                // println!("Setting client {} failpoint {}={}", validator_idx, name, actions);
                modified_failpoints.insert((validator_idx, name));
            }

            if insert_transaction_every_part {
                transfer_coins_non_blocking(
                    &validator_clients[small_rng.gen_range(0usize, validator_clients.len())],
                    &transaction_factory,
                    &mut account_0,
                    &account_1,
                    10,
                )
                .await;
            }

            let elapsed = now.elapsed().as_secs_f32();
            let wanted = (1 + part) as f32 * cycle_duration_s / (parts_in_cycle as f32);
            if elapsed < wanted {
                thread::sleep(time::Duration::from_secs_f32(wanted - elapsed));
            }
        }

        let cur = get_all_states(&validator_clients).await;

        let transactions = cur.iter().map(|s| s.version).max().unwrap()
            - previous.iter().map(|s| s.version).max().unwrap();
        let rounds = cur.iter().map(|s| s.round).max().unwrap()
            - previous.iter().map(|s| s.round).max().unwrap();
        let epochs = cur.iter().map(|s| s.epoch).max().unwrap()
            - previous.iter().map(|s| s.epoch).max().unwrap();

        println!(
            "cycle {} lasted {:.3} with {} epochs, {} rounds and {} transactions",
            cycle,
            now.elapsed().as_secs_f32(),
            epochs,
            rounds,
            transactions,
        );
        println!(
            "All at versions: {:?}",
            cur.iter().map(|s| s.version).collect::<Vec<_>>()
        );
        println!(
            "All at rounds: {:?}",
            cur.iter().map(|s| s.round).collect::<Vec<_>>()
        );

        check_cycle(cycle, rounds, transactions, cur.clone(), previous);
        previous = cur;
    }

    for (validator_idx, name) in modified_failpoints {
        validator_clients[validator_idx]
            .set_failpoint(name, "off".to_string())
            .await
            .unwrap();
    }

    thread::sleep(time::Duration::from_secs(4));

    let cur = get_all_states(&validator_clients).await;
    println!(
        "All at versions: {:?}",
        cur.iter().map(|s| s.version).collect::<Vec<_>>()
    );

    transfer_coins(
        validator_client_0,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;

    let cur = get_all_states(&validator_clients).await;
    println!(
        "All at versions: {:?}",
        cur.iter().map(|s| s.version).collect::<Vec<_>>()
    );
    let largest_v = cur.iter().map(|s| s.version).max().unwrap();
    println!("Largest version {}", largest_v);

    thread::sleep(time::Duration::from_secs(4));

    let cur = get_all_states(&validator_clients).await;
    println!(
        "All at versions: {:?}",
        cur.iter().map(|s| s.version).collect::<Vec<_>>()
    );

    let transactions: Vec<_> =
        join_all(validator_clients.iter().cloned().map(move |v| async move {
            let mut txns = v
                .get_transactions(None, Some(1000))
                .await
                .unwrap()
                .into_inner();
            txns.retain(|t| t.version().unwrap() <= largest_v);
            txns
        }))
        .await;

    for i in 1..transactions.len() {
        let txns_a = transactions.get(0).unwrap();
        let txns_b = transactions.get(i).unwrap();
        assert_eq!(txns_a.len(), txns_b.len());
        for i in 0..txns_a.len() {
            assert_eq!(txns_a[i], txns_b[i]);
        }
    }
}

#[tokio::test]
async fn test_no_failures() {
    // Check that we can get meaningful throughput from the local network
    // Pace here is much slower than max we can do, to remove noise.
    let num_validators = 5;
    run_fail_point_test(
        num_validators,
        4,
        5.0,
        5,
        true,
        Box::new(move |_, _| (vec![], false)),
        Box::new(|_, executed_rounds, executed_transactions, _, _| {
            assert!(
                executed_transactions >= 10,
                "no progress with active consensus, only {} transactions",
                executed_transactions
            );
            assert!(
                executed_rounds >= 4,
                "no progress with active consensus, only {} rounds",
                executed_rounds
            );
        }),
    )
    .await;
}

#[tokio::test]
async fn test_fault_tolerance_of_network_send() {
    // Randomly increase network failure rate, until network halts, and check that it comes back afterwards.
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 5;
    let num_cycles = 6;
    run_fail_point_test(
        num_validators,
        num_cycles,
        5.0,
        5,
        true,
        Box::new(move |cycle, _part| {
            let max = 10 * (10 - num_cycles + cycle + 1);
            let rand: usize = small_rng.gen_range(0, 1000);
            let rand_reliability = ((rand as f32 / 1000.0).powf(0.20) * max as f32) as i32;
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
        Box::new(|_, _, _, _, _| {}),
    )
    .await;
}

#[tokio::test]
async fn test_fault_tolerance_of_network_receive() {
    // Randomly increase network failure rate, until network halts, and check that it comes back afterwards.
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 5;
    let num_cycles = 6;
    run_fail_point_test(
        num_validators,
        num_cycles,
        5.0,
        5,
        true,
        Box::new(move |cycle, _part| {
            let max = 10 * (10 - num_cycles + cycle + 1);
            let rand: usize = small_rng.gen_range(0, 1000);
            let rand_reliability = ((rand as f32 / 1000.0).powf(0.33) * max as f32) as i32;
            println!("{}", rand_reliability);
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
        Box::new(|_, _, _, _, _| {}),
    )
    .await;
}

#[tokio::test]
async fn test_changing_working_consensus() {
    // with 7 nodes, consensus needs 5 to operate.
    // we rotate in each cycle, which 2 nodes are down.
    // we should consisnently be seeing progress.
    let num_validators = 7;
    run_fail_point_test(
        num_validators,
        6,
        5.0,
        5,
        true,
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
        Box::new(|_, executed_rounds, executed_transactions, _, _| {
            assert!(
                executed_transactions >= 5,
                "no progress with active consensus, only {} transactions",
                executed_transactions
            );
            assert!(
                executed_rounds >= 2,
                "no progress with active consensus, only {} rounds",
                executed_rounds
            );
        }),
    )
    .await;
}

#[tokio::test]
async fn test_changing_working_consensus_fast() {
    // with 7 nodes, consensus needs 5 to operate.
    // we rotate in each part, which 2 nodes are down.
    // we should consisnently be seeing progress.
    let mut rng = SmallRng::from_seed([5u8; 16]);
    let num_validators = 7;
    run_fail_point_test(
        num_validators,
        4,
        5.0,
        5,
        true,
        Box::new(move |_, _| {
            let client_1 = rng.gen_range(0, num_validators);
            let client_2 = rng.gen_range(0, num_validators);
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
        }),
        Box::new(|_, executed_rounds, executed_transactions, _, _| {
            assert!(
                executed_transactions >= 4,
                "no progress with active consensus, only {} transactions",
                executed_transactions
            );
            assert!(
                executed_rounds >= 2,
                "no progress with active consensus, only {} rounds",
                executed_rounds
            );
        }),
    )
    .await;
}

#[tokio::test]
async fn test_alternating_having_consensus() {
    // with 5 nodes, consensus needs 4 to operate.
    // we alternate between 1 and 2 nodes being down,
    // and checking progress or no progress
    let num_validators = 5;
    run_fail_point_test(
        num_validators,
        6,
        5.0,
        5,
        true,
        Box::new(move |cycle, part| {
            if part == 0 {
                let client_1 = (cycle * 2) % num_validators;
                let mut res = vec![
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
                ];
                if cycle % 2 == 1 {
                    let client_2 = (cycle * 2 + 1) % num_validators;
                    res.push((
                        client_2,
                        "consensus::send::any".to_string(),
                        "return".to_string(),
                    ));
                    res.push((
                        client_2,
                        "consensus::process::any".to_string(),
                        "return".to_string(),
                    ));
                }
                (res, true)
            } else {
                (vec![], false)
            }
        }),
        Box::new(|cycle, executed_rounds, executed_transactions, _, _| {
            if cycle % 2 == 1 {
                // allow 1 round / 3 transactions, in case anything was leftover in the pipeline
                assert!(
                    executed_transactions <= 3,
                    "progress with active consensus, {} transactions",
                    executed_transactions
                );
                assert!(
                    executed_rounds <= 1,
                    "progress with active consensus, {} rounds",
                    executed_rounds
                );
            } else {
                assert!(
                    executed_transactions >= 5,
                    "no progress with active consensus, only {} transactions",
                    executed_transactions
                );
                assert!(
                    executed_rounds >= 2,
                    "no progress with active consensus, only {} rounds",
                    executed_rounds
                );
            }
        }),
    )
    .await;
}
