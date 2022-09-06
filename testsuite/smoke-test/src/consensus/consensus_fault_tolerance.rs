// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use crate::smoke_test_environment::SwarmBuilder;
use crate::test_utils::{create_and_fund_account, transfer_coins_non_blocking};
use aptos_logger::info;
use forge::{
    test_utils::consensus_utils::{
        no_failure_injection, test_consensus_fault_tolerance, FailPointFailureInjection, NodeState,
    },
    LocalSwarm, Swarm, SwarmExt,
};
use rand::{self, Rng};
use rand::{rngs::SmallRng, SeedableRng};

pub async fn create_swarm(num_nodes: usize, max_block_txns: u64) -> LocalSwarm {
    let swarm = SwarmBuilder::new_local(num_nodes)
        .with_init_config(Arc::new(move |_, config, _| {
            config.api.failpoints_enabled = true;
            config.consensus.max_block_txns = max_block_txns;
            config.consensus.round_initial_timeout_ms = 1000;
            // no increase in timeout, to have stable round/second rate.
            config.consensus.round_timeout_backoff_exponent_base = 1.0;
            // empty block generated automatically every ~half a second
            config.consensus.quorum_store_poll_count = 15;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
        }))
        .build()
        .await;

    println!(
        "Validators {:?}",
        swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>()
    );
    swarm
}

struct ActiveTrafficGuard {
    finish_traffic: Arc<AtomicBool>,
}

impl Drop for ActiveTrafficGuard {
    fn drop(&mut self) {
        self.finish_traffic.store(true, Ordering::Relaxed);
    }
}

async fn start_traffic(num_accounts: usize, tps: f32, swarm: &mut dyn Swarm) -> ActiveTrafficGuard {
    let validator_clients = swarm.get_all_nodes_clients_with_names();

    let finish = Arc::new(AtomicBool::new(false));
    let finish_copy = finish.clone();
    let transaction_factory = swarm.chain_info().transaction_factory();

    info!("Preparing accounts");
    let mut accounts = vec![];
    for _ in 0..num_accounts {
        accounts.push(create_and_fund_account(swarm, 10000000).await);
    }

    info!("Starting traffic");
    tokio::spawn(async move {
        let mut small_rng = SmallRng::from_entropy();

        let now = Instant::now();
        let mut index = 0;
        while !finish.load(Ordering::Relaxed) {
            let sender = small_rng.gen_range(0usize, accounts.len() - 1);
            let (a, b) = accounts.split_at_mut(sender + 1);
            transfer_coins_non_blocking(
                &validator_clients[small_rng.gen_range(0usize, validator_clients.len())].1,
                &transaction_factory,
                &mut a[sender],
                &b[small_rng.gen_range(0, b.len())],
                1,
            )
            .await;

            index += 1;

            let elapsed = now.elapsed().as_secs_f32();
            let wanted = (1 + index) as f32 / tps;
            if elapsed < wanted {
                tokio::time::sleep(Duration::from_secs_f32(wanted - elapsed)).await;
            } else if elapsed > wanted + 1.0 {
                info!("Traffic is running {}s behind", elapsed - wanted);
            }
        }
    });
    ActiveTrafficGuard {
        finish_traffic: finish_copy,
    }
}

async fn run_fail_point_test(
    num_validators: usize,
    cycles: usize,
    cycle_duration_s: f32,
    parts_in_cycle: usize,
    traffic_tps: f32,
    max_block_size: u64,
    // (cycle, part) -> (Vec(validator_index, name, action), reset_old_enpoints)
    get_fail_points_to_set: Box<
        dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool) + Send,
    >,
    // (cycle, executed_epochs, executed_rounds, executed_transactions, current_state, previous_state)
    check_cycle: Box<
        dyn FnMut(usize, u64, u64, u64, Vec<NodeState>, Vec<NodeState>) -> anyhow::Result<()>,
    >,
) {
    let mut swarm = create_swarm(num_validators, max_block_size).await;
    let _active_traffic = if traffic_tps > 0.0 {
        start_traffic(5, traffic_tps, &mut swarm).await
    } else {
        ActiveTrafficGuard {
            finish_traffic: Arc::new(AtomicBool::new(false)),
        }
    };
    test_consensus_fault_tolerance(
        &mut swarm,
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
async fn test_no_failures() {
    let num_validators = 3;

    let mut swarm = create_swarm(num_validators, 1).await;

    test_consensus_fault_tolerance(
        &mut swarm,
        3,
        5.0,
        1,
        no_failure_injection(),
        Box::new(move |_, _, executed_rounds, executed_transactions, _, _| {
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
            Ok(())
        }),
        true,
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_fault_tolerance_of_network_send() {
    // Randomly increase network failure rate, until network halts, and check that it comes back afterwards.
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 3;
    let num_cycles = 4;
    run_fail_point_test(
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
async fn test_fault_tolerance_of_network_receive() {
    // Randomly increase network failure rate, until network halts, and check that it comes back afterwards.
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 3;
    let num_cycles = 4;
    run_fail_point_test(
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
        1.0,
        1,
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
                executed_transactions >= 5,
                "no progress with active consensus, only {} transactions",
                executed_transactions
            );
            assert!(
                executed_rounds >= 2,
                "no progress with active consensus, only {} rounds",
                executed_rounds
            );
            Ok(())
        }),
    )
    .await;
}

#[ignore]
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
        1.0,
        1,
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
        Box::new(|_, _, executed_rounds, executed_transactions, _, _| {
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
            Ok(())
        }),
    )
    .await;
}

#[ignore]
#[tokio::test]
async fn test_alternating_having_consensus() {
    // with 5 nodes, consensus needs 4 to operate.
    // we alternate between 1 and 2 nodes being down,
    // and checking progress or no progress
    let num_validators = 5;
    run_fail_point_test(
        num_validators,
        8,
        4.0,
        1,
        1.0,
        1,
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
        Box::new(|cycle, _, executed_rounds, executed_transactions, _, _| {
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
            Ok(())
        }),
    )
    .await;
}
