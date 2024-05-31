use std::collections::BTreeMap;
use std::iter;
use std::sync::Arc;
use std::time::Duration;

use rand::seq::SliceRandom;
use rand::{thread_rng, Rng};
use tokio::sync::mpsc;
use tokio::time::Instant;
use tokio::{spawn, time};

use crate::delays::{heterogeneous_symmetric_delay, spacial_delay_2d, DelayFunction};
use crate::framework::network::{InjectedLocalNetwork, Network, NetworkInjection};
use crate::framework::timer::InjectedTimerService;
use crate::framework::{NodeId, Protocol};
use crate::leader_schedule::round_robin;
use crate::multichain::{Config, MultiChainBft};

pub mod delays;
pub mod framework;
pub mod jolteon;
pub mod jolteon_fast_qs;
pub mod leader_schedule;
pub mod metrics;
pub mod raikou;
pub mod multichain;
pub mod utils;

type Slot = i64;

const PBFT_TIMEOUT: u32 = 5; // in Deltas
const JOLTEON_TIMEOUT: u32 = 3; // in Deltas

// TODO: generalize this to any protocol
fn multichain_network_injection(
    delay_function: impl DelayFunction<multichain::Message>,
    crashes: Vec<(NodeId, Slot)>,
) -> impl NetworkInjection<multichain::Message> {
    let crashes = Arc::new(BTreeMap::from_iter(crashes));

    move |from, to, message| {
        let delay_function = delay_function.clone();
        let crashes = crashes.clone();

        async move {
            if to == from {
                if let multichain::Message::Entering(slot) = message {
                    if let Some(crash_slot) = crashes.get(&to) {
                        if slot >= *crash_slot {
                            // Replace the message with a notification that the
                            // node should halt when the node sends Entering(slot)
                            // to itself with`slot >= crash_slot`.
                            return Some(multichain::Message::Crash);
                        }
                    }
                }
            }

            let delay = f64::max(delay_function(from, to, &message), 0.);
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
            Some(message)
        }
    }
}
fn network_injection<M: Send>(
    delay_function: impl DelayFunction<M>,
    // crashes: Vec<(NodeId, Instant)>,
) -> impl NetworkInjection<M> {
    move |from, to, message| {
        let delay_function = delay_function.clone();

        async move {
            let delay = f64::max(delay_function(from, to, &message), 0.);
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
            Some(message)
        }
    }
}

async fn test_multichain(
    delay_function: impl DelayFunction<multichain::Message>,
    n_nodes: usize,
    slots_per_delta: Slot,
    delta: f64,
    n_slots: Slot,
    optimistic_dissemination: bool,
    crashes: Vec<(NodeId, Slot)>,
    // choose one arbitrary correct node to monitor it more closely.
    monitored_node: NodeId,
) {
    if 3 * crashes.len() + 1 > n_nodes {
        println!("WARNING: too many crashes, the protocol may stall.");
    }

    let spawn_delay_distr = rand_distr::Uniform::new(1. * delta, 20. * delta);
    let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();

    let mut network = InjectedLocalNetwork::new(
        n_nodes,
        multichain_network_injection(delay_function, crashes),
    );

    let config = Config {
        n_nodes,
        slots_per_delta,
        leader_timeout: PBFT_TIMEOUT,
        leader_schedule: round_robin(n_nodes),
        delta: Duration::from_secs_f64(delta),
        halt_on_slot: n_slots + 1,
        progress_threshold: 0.75,
        slot_duration_sample_size: 20,
        responsive: true,
        adaptive_timer: true,
        optimistic_dissemination,
    };

    let mut join_handles = Vec::new();

    // Semaphore is used to track the number of nodes that have started.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    let mut propose_time = metrics::UnorderedBuilder::new();
    let mut enter_time = metrics::UnorderedBuilder::new();
    let mut batch_commit_time = metrics::UnorderedBuilder::new();
    let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();

    let start_time = Instant::now();
    for node_id in 0..n_nodes {
        let config = config.clone();
        let network_service = network.service(node_id);

        let clock_speed = { thread_rng().sample(clock_speed_distr) };
        let timer = InjectedTimerService::local(move |duration, event: multichain::TimerEvent| {
            (
                Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
                event,
            )
        });

        let propose_time_sender = Some(propose_time.new_sender());
        let enter_time_sender = if node_id == monitored_node {
            Some(enter_time.new_sender())
        } else {
            None
        };
        let batch_commit_time_sender = Some(batch_commit_time.new_sender());
        let indirectly_committed_slots_sender = if node_id == monitored_node {
            Some(indirectly_committed_slots.new_sender())
        } else {
            None
        };

        let semaphore = semaphore.clone();
        join_handles.push(spawn(async move {
            // Sleep for a random duration before spawning the node.
            let spawn_delay = {
                let mut rng = thread_rng();
                rng.sample(spawn_delay_distr)
            };
            time::sleep(Duration::from_secs_f64(spawn_delay)).await;

            // // Before starting the node, "drop" all messages sent to it during the spawn delay.
            // network_service.clear_inbox().await;

            // println!("Spawning node {node_id}");
            let mut node = MultiChainBft::new_node(
                node_id,
                config,
                start_time,
                node_id == monitored_node,
                multichain::Metrics {
                    propose_time: propose_time_sender,
                    enter_time: enter_time_sender,
                    batch_commit_time: batch_commit_time_sender,
                    indirectly_committed_slots: indirectly_committed_slots_sender,
                },
            );

            semaphore.add_permits(1);
            node.run(node_id, network_service, timer).await
        }));
    }

    let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
    println!("All nodes are running!");

    for join_handle in join_handles {
        join_handle.await.unwrap();
    }

    let propose_time = propose_time
        .build()
        .await
        .sort()
        .drop_first(29)
        .drop_last(10)
        .derivative();
    println!("Propose Time:");
    propose_time.print_stats();
    propose_time.show_histogram(n_slots as usize / 5, 10);
    println!();

    // let enter_time = enter_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(19)
    //     .drop_last(10)
    //     .derivative();
    // println!("Enter Time:");
    // enter_time.print_stats();
    // enter_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    let batch_commit_time = batch_commit_time
        .build()
        .await
        .filter(|&(depth, _)| depth >= 20)
        .map(|(_, time)| time)
        .sort();
    println!("Batch commit time:");
    batch_commit_time.print_stats();
    batch_commit_time.show_histogram(n_slots as usize / 10, 10);
    println!();

    println!("Indirectly Committed Slots:");
    let indirectly_committed_slots = indirectly_committed_slots
        .build()
        .await
        .filter(|&slot| slot >= 20);
    println!("\tCount: {}", indirectly_committed_slots.len());
    println!(
        "\tList: {:?}",
        indirectly_committed_slots.clone().into_vec()
    );
    indirectly_committed_slots
        .map(|slot| slot as f64) // histogram is supported only for f64.
        .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
    println!();
}

async fn test_multichain_with_random_crashes(
    delay_function: impl DelayFunction<multichain::Message>,
    n_nodes: usize,
    slots_per_delta: Slot,
    delta: f64,
    n_slots: Slot,
    optimistic_dissemination: bool,
    n_crashes: usize,
    crash_slot: Slot,
    // choose one arbitrary correct node to monitor it more closely.
    monitored_node: NodeId,
) {
    let mut nodes = (0..n_nodes).collect::<Vec<_>>();
    nodes.shuffle(&mut thread_rng());
    let crashes = nodes
        .into_iter()
        .take(n_crashes)
        .map(|node_id| (node_id, crash_slot))
        .collect();

    test_multichain(
        delay_function,
        n_nodes,
        slots_per_delta,
        delta,
        n_slots,
        optimistic_dissemination,
        crashes,
        monitored_node,
    )
    .await;
}

async fn test_multichain_with_consecutive_faulty_leaders_in_a_chain(
    delay_function: impl DelayFunction<multichain::Message>,
    n_nodes: usize,
    slots_per_delta: Slot,
    delta: f64,
    n_slots: Slot,
    optimistic_dissemination: bool,
    n_crashes: usize,
    crash_slot: Slot,
    // choose one arbitrary correct node to monitor it more closely.
    monitored_node: NodeId,
) {
    let n_chains = PBFT_TIMEOUT * slots_per_delta as u32;
    let crashes = (0..n_crashes)
        .map(|i| ((n_chains as NodeId * i as NodeId) % n_nodes, crash_slot))
        .collect();

    test_multichain(
        delay_function,
        n_nodes,
        slots_per_delta,
        delta,
        n_slots,
        optimistic_dissemination,
        crashes,
        monitored_node,
    )
    .await;
}

async fn test_jolteon(
    delay_function: impl DelayFunction<jolteon::Message<()>>,
    n_nodes: usize,
    delta: f64,
    warmup_duration_in_delta: u32,
    total_duration_in_delta: u32,
    // crashes: Vec<(NodeId, Slot)>,
    // choose one arbitrary correct node to monitor it more closely.
    monitored_node: NodeId,
) {
    // if 3 * crashes.len() + 1 > n_nodes {
    //     println!("WARNING: too many crashes, the protocol may stall.");
    // }

    let spawn_delay_distr =
        rand_distr::Uniform::new(1. * delta, warmup_duration_in_delta as f64 * delta);
    let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();

    let mut network = InjectedLocalNetwork::new(n_nodes, network_injection(delay_function));

    let config = jolteon::Config {
        n_nodes,
        f: (n_nodes - 1) / 3,
        leader_timeout: JOLTEON_TIMEOUT,
        leader_schedule: round_robin(n_nodes),
        delta: Duration::from_secs_f64(delta),
        end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
    };

    let mut join_handles = Vec::new();

    // Semaphore is used to track the number of nodes that have started.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    // let mut propose_time = metrics::UnorderedBuilder::new();
    // let mut enter_time = metrics::UnorderedBuilder::new();
    // let mut batch_commit_time = metrics::UnorderedBuilder::new();
    // let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();

    let start_time = Instant::now();
    for node_id in 0..n_nodes {
        let config = config.clone();
        let network_service = network.service(node_id);

        let clock_speed = { thread_rng().sample(clock_speed_distr) };
        let timer = InjectedTimerService::local(move |duration, event| {
            (
                Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
                event,
            )
        });

        // let propose_time_sender = Some(propose_time.new_sender());
        // let enter_time_sender = if node_id == monitored_node {
        //     Some(enter_time.new_sender())
        // } else {
        //     None
        // };
        // let batch_commit_time_sender = Some(batch_commit_time.new_sender());
        // let indirectly_committed_slots_sender = if node_id == monitored_node {
        //     Some(indirectly_committed_slots.new_sender())
        // } else {
        //     None
        // };

        let semaphore = semaphore.clone();
        join_handles.push(spawn(async move {
            // Sleep for a random duration before spawning the node.
            let spawn_delay = {
                let mut rng = thread_rng();
                rng.sample(spawn_delay_distr)
            };
            time::sleep(Duration::from_secs_f64(spawn_delay)).await;

            // // Before starting the node, "drop" all messages sent to it during the spawn delay.
            // network_service.clear_inbox().await;

            // TODO: add actual transactions
            let (_, txns_receiver) = mpsc::channel::<()>(100);

            // println!("Spawning node {node_id}");
            let mut node = jolteon::JolteonNode::new(
                node_id,
                config,
                txns_receiver,
                start_time,
                node_id == monitored_node,
            );

            semaphore.add_permits(1);
            node.run(node_id, network_service, timer).await
        }));
    }

    let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
    println!("All nodes are running!");

    for join_handle in join_handles {
        join_handle.await.unwrap();
    }

    // let propose_time = propose_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(29)
    //     .drop_last(10)
    //     .derivative();
    // println!("Propose Time:");
    // propose_time.print_stats();
    // propose_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    // let enter_time = enter_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(19)
    //     .drop_last(10)
    //     .derivative();
    // println!("Enter Time:");
    // enter_time.print_stats();
    // enter_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    // let batch_commit_time = batch_commit_time
    //     .build()
    //     .await
    //     .filter(|&(depth, _)| depth >= 20)
    //     .map(|(_, time)| time)
    //     .sort();
    // println!("Batch commit time:");
    // batch_commit_time.print_stats();
    // batch_commit_time.show_histogram(n_slots as usize / 10, 10);
    // println!();

    // println!("Indirectly Committed Slots:");
    // let indirectly_committed_slots = indirectly_committed_slots
    //     .build()
    //     .await
    //     .filter(|&slot| slot >= 20);
    // println!("\tCount: {}", indirectly_committed_slots.len());
    // println!(
    //     "\tList: {:?}",
    //     indirectly_committed_slots.clone().into_vec()
    // );
    // indirectly_committed_slots
    //     .map(|slot| slot as f64) // histogram is supported only for f64.
    //     .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
    // println!();
}

async fn test_jolteon_with_fast_qs(
    delay_function: impl DelayFunction<jolteon_fast_qs::Message<()>>,
    n_nodes: usize,
    delta: f64,
    warmup_duration_in_delta: u32,
    total_duration_in_delta: u32,
    // crashes: Vec<(NodeId, Slot)>,
    // choose one arbitrary correct node to monitor it more closely.
    monitored_node: NodeId,
) {
    // if 3 * crashes.len() + 1 > n_nodes {
    //     println!("WARNING: too many crashes, the protocol may stall.");
    // }

    let spawn_delay_distr =
        rand_distr::Uniform::new(1. * delta, warmup_duration_in_delta as f64 * delta);
    let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();

    let mut network = InjectedLocalNetwork::new(n_nodes, network_injection(delay_function));

    let f = (n_nodes - 1) / 3;

    let config = jolteon_fast_qs::Config {
        n_nodes,
        f,
        storage_requirement: f + (f / 2 + 1),
        leader_timeout: JOLTEON_TIMEOUT,
        leader_schedule: round_robin(n_nodes),
        delta: Duration::from_secs_f64(delta),
        batch_interval: Duration::from_secs_f64(delta * 0.1),
        end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
    };

    let mut join_handles = Vec::new();

    // Semaphore is used to track the number of nodes that have started.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    // let mut propose_time = metrics::UnorderedBuilder::new();
    // let mut enter_time = metrics::UnorderedBuilder::new();
    let mut batch_commit_time = metrics::UnorderedBuilder::new();
    // let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();

    let start_time = Instant::now();
    for node_id in 0..n_nodes {
        let config = config.clone();
        let network_service = network.service(node_id);

        let clock_speed = { thread_rng().sample(clock_speed_distr) };
        let timer = InjectedTimerService::local(move |duration, event| {
            (
                Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
                event,
            )
        });

        // let propose_time_sender = Some(propose_time.new_sender());
        // let enter_time_sender = if node_id == monitored_node {
        //     Some(enter_time.new_sender())
        // } else {
        //     None
        // };
        let batch_commit_time_sender = Some(batch_commit_time.new_sender());
        // let indirectly_committed_slots_sender = if node_id == monitored_node {
        //     Some(indirectly_committed_slots.new_sender())
        // } else {
        //     None
        // };

        let semaphore = semaphore.clone();
        join_handles.push(spawn(async move {
            // Sleep for a random duration before spawning the node.
            let spawn_delay = {
                let mut rng = thread_rng();
                rng.sample(spawn_delay_distr)
            };
            time::sleep(Duration::from_secs_f64(spawn_delay)).await;

            // // Before starting the node, "drop" all messages sent to it during the spawn delay.
            // network_service.clear_inbox().await;

            // TODO: add actual transactions
            let next_txn = || ();

            // println!("Spawning node {node_id}");
            let mut node = jolteon_fast_qs::JolteonNode::new(
                node_id,
                config,
                next_txn,
                start_time,
                node_id == monitored_node,
                jolteon_fast_qs::Metrics {
                    // propose_time: propose_time_sender,
                    // enter_time: enter_time_sender,
                    batch_commit_time: batch_commit_time_sender,
                    // indirectly_committed_slots: indirectly_committed_slots_sender,
                },
            );

            semaphore.add_permits(1);
            node.run(node_id, network_service, timer).await
        }));
    }

    let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
    println!("All nodes are running!");

    for join_handle in join_handles {
        join_handle.await.unwrap();
    }

    // let propose_time = propose_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(29)
    //     .drop_last(10)
    //     .derivative();
    // println!("Propose Time:");
    // propose_time.print_stats();
    // propose_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    // let enter_time = enter_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(19)
    //     .drop_last(10)
    //     .derivative();
    // println!("Enter Time:");
    // enter_time.print_stats();
    // enter_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    let batch_commit_time = batch_commit_time
        .build()
        .await
        .filter(|&(time, _)| {
            time >= start_time + Duration::from_secs_f64(delta) * (2 * warmup_duration_in_delta)
        })
        .map(|(_, time)| time)
        .sort();
    println!("Batch commit time:");
    batch_commit_time.print_stats();
    batch_commit_time.show_histogram(30, 10);
    println!();

    // println!("Indirectly Committed Slots:");
    // let indirectly_committed_slots = indirectly_committed_slots
    //     .build()
    //     .await
    //     .filter(|&slot| slot >= 20);
    // println!("\tCount: {}", indirectly_committed_slots.len());
    // println!(
    //     "\tList: {:?}",
    //     indirectly_committed_slots.clone().into_vec()
    // );
    // indirectly_committed_slots
    //     .map(|slot| slot as f64) // histogram is supported only for f64.
    //     .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
    // println!();
}

async fn test_raikou(
    delay_function: impl DelayFunction<raikou::Message>,
    n_nodes: usize,
    delta: f64,
    spawn_period_in_delta: u32,
    warmup_period_in_delta: u32,
    total_duration_in_delta: u32,
    // crashes: Vec<(NodeId, Slot)>,
    // choose one arbitrary correct node to monitor it more closely.
    monitored_node: NodeId,
    enable_optimistic_dissemination: bool,
) {
    // if 3 * crashes.len() + 1 > n_nodes {
    //     println!("WARNING: too many crashes, the protocol may stall.");
    // }

    let spawn_delay_distr =
        rand_distr::Uniform::new(1. * delta, spawn_period_in_delta as f64 * delta);
    let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();

    let mut network = InjectedLocalNetwork::new(n_nodes, network_injection(delay_function));

    let f = (n_nodes - 1) / 3;

    let config = raikou::Config {
        n_nodes,
        f,
        storage_requirement: f + (f / 2 + 1),
        leader_timeout: JOLTEON_TIMEOUT,
        leader_schedule: round_robin(n_nodes),
        delta: Duration::from_secs_f64(delta),
        batch_interval: Duration::from_secs_f64(delta * 0.1),
        end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
        enable_optimistic_dissemination,
        extra_wait_before_qc_vote: Duration::from_secs_f64(delta * 0.1),
        enable_penalty_system: true,
        enable_round_entry_permission: false,
        enable_commit_votes: true,
    };

    let mut join_handles = Vec::new();

    // Semaphore is used to track the number of nodes that have started.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    // let mut propose_time = metrics::UnorderedBuilder::new();
    // let mut enter_time = metrics::UnorderedBuilder::new();
    let mut batch_commit_time = metrics::UnorderedBuilder::new();
    // let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();

    let start_time = Instant::now();
    for node_id in 0..n_nodes {
        let config = config.clone();
        let network_service = network.service(node_id);

        let clock_speed = { thread_rng().sample(clock_speed_distr) };
        let timer = InjectedTimerService::local(move |duration, event| {
            (
                Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
                event,
            )
        });

        // let propose_time_sender = Some(propose_time.new_sender());
        // let enter_time_sender = if node_id == monitored_node {
        //     Some(enter_time.new_sender())
        // } else {
        //     None
        // };
        let batch_commit_time_sender = Some(batch_commit_time.new_sender());
        // let indirectly_committed_slots_sender = if node_id == monitored_node {
        //     Some(indirectly_committed_slots.new_sender())
        // } else {
        //     None
        // };

        let semaphore = semaphore.clone();
        join_handles.push(spawn(async move {
            // Sleep for a random duration before spawning the node.
            let spawn_delay = {
                let mut rng = thread_rng();
                rng.sample(spawn_delay_distr)
            };
            time::sleep(Duration::from_secs_f64(spawn_delay)).await;

            // // Before starting the node, "drop" all messages sent to it during the spawn delay.
            // network_service.clear_inbox().await;

            // println!("Spawning node {node_id}");
            let mut node = raikou::RaikouNode::new(
                node_id,
                config,
                start_time,
                node_id == monitored_node,
                raikou::Metrics {
                    // propose_time: propose_time_sender,
                    // enter_time: enter_time_sender,
                    batch_commit_time: batch_commit_time_sender,
                    // indirectly_committed_slots: indirectly_committed_slots_sender,
                },
            );

            semaphore.add_permits(1);
            node.run(node_id, network_service, timer).await
        }));
    }

    let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
    println!("All nodes are running!");

    for join_handle in join_handles {
        join_handle.await.unwrap();
    }

    // let propose_time = propose_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(29)
    //     .drop_last(10)
    //     .derivative();
    // println!("Propose Time:");
    // propose_time.print_stats();
    // propose_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    // let enter_time = enter_time
    //     .build()
    //     .await
    //     .sort()
    //     .drop_first(19)
    //     .drop_last(10)
    //     .derivative();
    // println!("Enter Time:");
    // enter_time.print_stats();
    // enter_time.show_histogram(n_slots as usize / 5, 10);
    // println!();

    let batch_commit_time = batch_commit_time
        .build()
        .await
        .filter(|&(time, _)| {
            time >= start_time + Duration::from_secs_f64(delta) * warmup_period_in_delta
        })
        .map(|(_, time)| time)
        .sort();
    println!("Batch commit time:");
    batch_commit_time.print_stats();
    batch_commit_time.show_histogram(30, 10);
    println!();

    // println!("Indirectly Committed Slots:");
    // let indirectly_committed_slots = indirectly_committed_slots
    //     .build()
    //     .await
    //     .filter(|&slot| slot >= 20);
    // println!("\tCount: {}", indirectly_committed_slots.len());
    // println!(
    //     "\tList: {:?}",
    //     indirectly_committed_slots.clone().into_vec()
    // );
    // indirectly_committed_slots
    //     .map(|slot| slot as f64) // histogram is supported only for f64.
    //     .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
    // println!();
}

#[tokio::main]
async fn main() {
    // set up logging
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_target(false)
        .format_timestamp(None)
        .init();

    let n_nodes = 31;
    let delta = 2.;
    let spawn_period_in_delta = 10;
    let warmup_period_in_delta = 100;
    let total_duration_in_delta = 300;
    let monitored_node = 3;

    // run the test

    // test_jolteon_with_fast_qs(
    //     uniformly_random_delay(rand_distr::Normal::new(0.6, 0.06).unwrap()),
    //     31,
    //     2.,
    //     20,
    //     100,
    //     3,
    // )
    // .await;

    // test_jolteon_with_fast_qs(
    //     spacial_delay_2d(rand_distr::Normal::new(0.3 * delta, 0.03 * delta).unwrap()),
    //     n_nodes,
    //     delta,
    //     20,
    //     100,
    //     1,
    // )
    // .await;

    // test_raikou(
    //     spacial_delay_2d(rand_distr::Normal::new(0.3 * delta, 0.03 * delta).unwrap()),
    //     n_nodes,
    //     delta,
    //     warmup_period_in_delta,
    //     total_duration_in_delta,
    //     monitored_node,
    //     true,
    // )
    // .await;

    test_raikou(
        heterogeneous_symmetric_delay(
            // the mean delay between a pair of nodes is uniformly sampled between 0 and 0.5 delta.
            rand_distr::Uniform::new(0., 0.5 * delta),
            // 2% standard deviation from the mean in all delays.
            rand_distr::Normal::new(1., 0.02).unwrap(),
            // Fixed additive noise of 0.01 delta to make sure there are no 0-delay messages.
            rand_distr::Uniform::new(0.01 * delta, 0.0100001 * delta),
        ),
        n_nodes,
        delta,
        spawn_period_in_delta,
        warmup_period_in_delta,
        total_duration_in_delta,
        monitored_node,
        true,
    )
    .await;

    // test_jolteon(
    //     uniformly_random_delay(rand_distr::Normal::new(0.6, 0.06).unwrap()),
    //     31,
    //     2.,
    //     20,
    //     100,
    //     3,
    // )
    // .await;

    // // Optimistic scenario: fast uniform communication, no crashes
    // test_multichain(
    //     uniformly_random_delay(rand_distr::Normal::new(0.6, 0.06).unwrap()),
    //     31,
    //     2,
    //     2.,
    //     200,
    //     true,
    //     // vec![(10, 0), (1, 35), (4, 67), (5, 67), (6, 67)],
    //     vec![],
    //     3,
    // )
    // .await;

    // // Slow network scenario: slow uniform communication, no crashes
    // run_test(
    //     // Small standard deviation to ensure random order while still having very uniform delays.
    //     uniformly_random_delay(rand_distr::Normal::new(1.9, 0.0001).unwrap()),
    //     31,
    //     2,
    //     2.,
    //     200,
    //     true,
    //     // vec![(10, 0), (1, 35), (4, 67), (5, 67), (6, 67)],
    //     vec![],
    //     3,
    // )
    // .await;

    // test_with_random_crashes(
    //     uniformly_random_delay(rand_distr::Normal::new(0.3, 0.03).unwrap()),
    //     31,
    //     2,
    //     1.,
    //     200,
    //     false,
    //     5,
    //     35,
    //     3,
    // )
    // .await;

    // test_consecutive_faulty_leaders_in_a_chain(
    //     uniformly_random_delay(rand_distr::Normal::new(0.3, 0.03).unwrap()),
    //     31,
    //     2,
    //     1.,
    //     200,
    //     5,
    //     35,
    //     3,
    // )
    // .await;

    // run_test(
    //     spacial_delay_2d(rand_distr::Normal::new(0.9, 0.09).unwrap()),
    //     31,
    //     4,
    //     1.,
    //     200,
    //     // vec![(10, 0), (1, 35), (4, 67), (5, 67), (6, 67)],
    //     vec![],
    //     3,
    // ).await;

    // run_test(
    //     clustered_delay(
    //         rand_distr::Normal::new(0.3, 0.03).unwrap(),
    //         rand_distr::Normal::new(0.9, 0.09).unwrap(),
    //         vec![
    //             vec![0, 3, 4, 5, 8, 9, 15],
    //             vec![1, 2, 11, 14],
    //             vec![6, 7, 10, 12, 13],
    //         ],
    //     ),
    //     31,
    //     2,
    //     1.,
    //     200,
    //     vec![],
    //     3,
    // )
    // .await;

    // run_test(
    //     clustered_delay(
    //         rand_distr::Normal::new(0.3, 0.03).unwrap(),
    //         rand_distr::Normal::new(0.9, 0.09).unwrap(),
    //         vec![
    //             vec![0, 3, 4, 5, 6, 7,  8, 9, 10, 13, 14],
    //             vec![1, 2, 11, 12, 15],
    //         ],
    //     ),
    //     31,
    //     2,
    //     1.,
    //     200,
    //     vec![],
    //     3,
    // )
    // .await;
}
