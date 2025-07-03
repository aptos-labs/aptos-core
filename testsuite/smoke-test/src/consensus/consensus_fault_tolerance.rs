// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{create_and_fund_account, transfer_coins_non_blocking},
};
use aptos_forge::{
    test_utils::consensus_utils::{
        no_failure_injection, test_consensus_fault_tolerance, FailPointFailureInjection, NodeState,
    },
    LocalSwarm, NodeExt, Swarm, SwarmExt,
};
use aptos_logger::info;
use rand::{self, rngs::SmallRng, Rng, SeedableRng};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

pub async fn create_swarm(num_nodes: usize, max_block_txns: u64) -> LocalSwarm {
    let swarm = SwarmBuilder::new_local(num_nodes)
        .with_init_config(Arc::new(move |_, config, _| {
            config.api.failpoints_enabled = true;
            config.consensus.max_sending_block_txns = max_block_txns;
            config.consensus.quorum_store.sender_max_batch_txns = config
                .consensus
                .quorum_store
                .sender_max_batch_txns
                .min(max_block_txns as usize);
            config.consensus.quorum_store.receiver_max_batch_txns = config
                .consensus
                .quorum_store
                .receiver_max_batch_txns
                .min(max_block_txns as usize);
            config.consensus.round_initial_timeout_ms = 1000;
            // no increase in timeout, to have stable round/second rate.
            config.consensus.round_timeout_backoff_exponent_base = 1.0;
            // empty block generated automatically every ~half a second
            config.consensus.quorum_store_poll_time_ms = 500;
            config
                .state_sync
                .state_sync_driver
                .enable_auto_bootstrapping = true;
            config
                .state_sync
                .state_sync_driver
                .max_connection_deadline_secs = 3;
            config.indexer_db_config.enable_event = true;
        }))
        .build()
        .await;

    println!(
        "Validators {:?}",
        swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>()
    );
    swarm
}

pub struct ActiveTrafficGuard {
    pub finish_traffic: Arc<AtomicBool>,
}

impl Drop for ActiveTrafficGuard {
    fn drop(&mut self) {
        self.finish_traffic.store(true, Ordering::Relaxed);
    }
}

pub async fn start_traffic(
    num_accounts: usize,
    tps: f32,
    swarm: &mut dyn Swarm,
) -> ActiveTrafficGuard {
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
        dyn FnMut(usize, u64, u64, u64, Vec<NodeState>, Vec<NodeState>) -> anyhow::Result<()>
            + Send,
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

fn successful_criteria(executed_epochs: u64, executed_rounds: u64, executed_transactions: u64) {
    assert!(
        executed_transactions >= 4,
        "no progress with active consensus, only {} transactions",
        executed_transactions
    );
    assert!(
        executed_epochs >= 1 || executed_rounds >= 2,
        "no progress with active consensus, only {} epochs, {} rounds",
        executed_epochs,
        executed_rounds
    );
}

#[tokio::test]
async fn test_no_failures() {
    let num_validators = 3;

    let swarm = create_swarm(num_validators, 1).await;

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
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
        true,
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_faulty_votes() {
    let num_validators = 7;

    let swarm = create_swarm(num_validators, 1).await;

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
        Box::new(FailPointFailureInjection::new(Box::new(move |cycle, _| {
            (
                vec![
                    (
                        cycle % num_validators,
                        "consensus::create_invalid_vote".to_string(),
                        format!("{}%return", 50),
                    ),
                    (
                        (cycle + 1) % num_validators,
                        "consensus::create_invalid_order_vote".to_string(),
                        format!("{}%return", 50),
                    ),
                    (
                        (cycle + 2) % num_validators,
                        "consensus::create_invalid_commit_vote".to_string(),
                        format!("{}%return", 50),
                    ),
                ],
                true,
            )
        }))),
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
        true,
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_ordered_only_cert() {
    let num_validators = 3;

    let swarm = create_swarm(num_validators, 1).await;

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
        Box::new(FailPointFailureInjection::new(Box::new(move |cycle, _| {
            (
                vec![(
                    cycle % num_validators,
                    "consensus::ordered_only_cert".to_string(),
                    format!("{}%return", 50),
                )],
                true,
            )
        }))),
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
        true,
        false,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_execution_retry() {
    let num_validators = 4;

    let swarm = create_swarm(num_validators, 1).await;
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
        Box::new(FailPointFailureInjection::new(Box::new(move |cycle, _| {
            (
                vec![(
                    cycle % num_validators,
                    "consensus::prepare_block".to_string(),
                    format!("{}%return", 50),
                )],
                true,
            )
        }))),
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
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
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
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
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
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
        Box::new(
            |cycle, executed_epochs, executed_rounds, executed_transactions, _, _| {
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
                        executed_epochs >= 1 || executed_rounds >= 2,
                        "no progress with active consensus, only {} epochs, {} rounds",
                        executed_epochs,
                        executed_rounds
                    );
                }
                Ok(())
            },
        ),
    )
    .await;
}

#[tokio::test]
async fn test_round_timeout_msg_rollout() {
    let num_validators = 3;

    let mut swarm = create_swarm(num_validators, 1).await;

    let (validator_clients, public_info) = {
        (
            swarm.get_validator_clients_with_names(),
            swarm.aptos_public_info(),
        )
    };
    test_consensus_fault_tolerance(
        validator_clients.clone(),
        public_info.clone(),
        3,
        5.0,
        1,
        no_failure_injection(),
        Box::new(
            move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                Ok(())
            },
        ),
        true,
        false,
    )
    .await
    .unwrap();

    for val in swarm.validators_mut() {
        val.stop();
        val.config_mut().consensus.enable_round_timeout_msg = true;
        val.start().unwrap();

        val.wait_until_healthy(Instant::now().checked_add(Duration::from_secs(60)).unwrap())
            .await
            .unwrap();

        test_consensus_fault_tolerance(
            validator_clients.clone(),
            public_info.clone(),
            1,
            30.0,
            1,
            no_failure_injection(),
            Box::new(
                move |_, executed_epochs, executed_rounds, executed_transactions, _, _| {
                    successful_criteria(executed_epochs, executed_rounds, executed_transactions);
                    Ok(())
                },
            ),
            true,
            false,
        )
        .await
        .unwrap();
    }
}
