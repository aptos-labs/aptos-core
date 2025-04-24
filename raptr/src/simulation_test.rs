// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(not(feature = "sim-types"), feature = "force-aptos-types"))]
use crate::raikou::dissemination::native::{Batch, NativeDisseminationLayer};
use crate::{
    delays::{heterogeneous_symmetric_delay, DelayFunction},
    framework::{
        crypto::{SignatureVerifier, Signer},
        module_network::ModuleNetwork,
        network::{InjectedLocalNetwork, Network, NetworkInjection, NetworkService},
        timer::{clock_skew_injection, InjectedTimerService},
        NodeId, Protocol,
    },
    metrics,
    metrics::display_metric,
    raikou,
    raikou::{
        dissemination,
        dissemination::native::{Batch, NativeDisseminationLayer},
        types::N_SUB_BLOCKS,
        RaikouNode,
    },
};
use aptos_crypto::bls12381::{PrivateKey, PublicKey};
use aptos_types::{
    account_address::AccountAddress, validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use rand::{thread_rng, Rng};
use std::{
    collections::BTreeMap,
    iter,
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{time, time::Instant};

fn network_injection<M: Send>(
    delay_function: impl DelayFunction,
    // crashes: Vec<(NodeId, Instant)>,
) -> impl NetworkInjection<M> {
    move |from, to, message| {
        let delay_function = delay_function.clone();

        async move {
            let delay = f64::max(delay_function(from, to), 0.);
            tokio::time::sleep(Duration::from_secs_f64(delay)).await;
            Some(message)
        }
    }
}

// fn multichain_network_injection(
//     delay_function: impl DelayFunction,
//     crashes: Vec<(NodeId, Slot)>,
// ) -> impl NetworkInjection<multichain::Message> {
//     let crashes = Arc::new(BTreeMap::from_iter(crashes));
//
//     move |from, to, message| {
//         let delay_function = delay_function.clone();
//         let crashes = crashes.clone();
//
//         async move {
//             if to == from {
//                 if let multichain::Message::Entering(slot) = message {
//                     if let Some(crash_slot) = crashes.get(&to) {
//                         if slot >= *crash_slot {
//                             // Replace the message with a notification that the
//                             // node should halt when the node sends Entering(slot)
//                             // to itself with`slot >= crash_slot`.
//                             return Some(multichain::Message::Crash);
//                         }
//                     }
//                 }
//             }
//
//             let delay = f64::max(delay_function(from, to), 0.);
//             tokio::time::sleep(Duration::from_secs_f64(delay)).await;
//             Some(message)
//         }
//     }
// }
//
// async fn test_multichain(
//     delay_function: impl DelayFunction,
//     n_nodes: usize,
//     slots_per_delta: Slot,
//     delta: f64,
//     n_slots: Slot,
//     optimistic_dissemination: bool,
//     crashes: Vec<(NodeId, Slot)>,
//     // choose one arbitrary correct node to monitor it more closely.
//     monitored_node: NodeId,
// ) {
//     if 3 * crashes.len() + 1 > n_nodes {
//         println!("WARNING: too many crashes, the protocol may stall.");
//     }
//
//     let spawn_delay_distr = rand_distr::Uniform::new(1. * delta, 20. * delta);
//     let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();
//
//     let mut network = InjectedLocalNetwork::new(
//         n_nodes,
//         multichain_network_injection(delay_function, crashes),
//     );
//
//     let config = Config {
//         n_nodes,
//         slots_per_delta,
//         leader_timeout: PBFT_TIMEOUT,
//         leader_schedule: round_robin(n_nodes),
//         delta: Duration::from_secs_f64(delta),
//         halt_on_slot: n_slots + 1,
//         progress_threshold: 0.75,
//         slot_duration_sample_size: 20,
//         responsive: true,
//         adaptive_timer: true,
//         optimistic_dissemination,
//     };
//
//     let mut join_handles = Vec::new();
//
//     // Semaphore is used to track the number of nodes that have started.
//     let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
//     let mut propose_time = metrics::UnorderedBuilder::new();
//     let mut enter_time = metrics::UnorderedBuilder::new();
//     let mut batch_commit_time = metrics::UnorderedBuilder::new();
//     let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();
//
//     let start_time = Instant::now();
//     for node_id in 0..n_nodes {
//         let config = config.clone();
//         let network_service = network.service(node_id);
//
//         let clock_speed = { thread_rng().sample(clock_speed_distr) };
//         let timer = InjectedTimerService::local(move |duration, event: multichain::TimerEvent| {
//             (
//                 Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
//                 event,
//             )
//         });
//
//         let propose_time_sender = Some(propose_time.new_sender());
//         let enter_time_sender = if node_id == monitored_node {
//             Some(enter_time.new_sender())
//         } else {
//             None
//         };
//         let batch_commit_time_sender = Some(batch_commit_time.new_sender());
//         let indirectly_committed_slots_sender = if node_id == monitored_node {
//             Some(indirectly_committed_slots.new_sender())
//         } else {
//             None
//         };
//
//         let semaphore = semaphore.clone();
//         join_handles.push(spawn(async move {
//             // Sleep for a random duration before spawning the node.
//             let spawn_delay = {
//                 let mut rng = thread_rng();
//                 rng.sample(spawn_delay_distr)
//             };
//             time::sleep(Duration::from_secs_f64(spawn_delay)).await;
//
//             // // Before starting the node, "drop" all messages sent to it during the spawn delay.
//             // network_service.clear_inbox().await;
//
//             // println!("Spawning node {node_id}");
//             let node = Arc::new(tokio::sync::Mutex::new(MultiChainBft::new_node(
//                 node_id,
//                 config,
//                 start_time,
//                 node_id == monitored_node,
//                 multichain::Metrics {
//                     propose_time: propose_time_sender,
//                     enter_time: enter_time_sender,
//                     batch_commit_time: batch_commit_time_sender,
//                     indirectly_committed_slots: indirectly_committed_slots_sender,
//                 },
//             )));
//
//             semaphore.add_permits(1);
//             Protocol::run(node, node_id, network_service, timer).await
//         }));
//     }
//
//     let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
//     println!("All nodes are running!");
//
//     for join_handle in join_handles {
//         join_handle.await.unwrap();
//     }
//
//     let propose_time = propose_time
//         .build()
//         .await
//         .sort()
//         .drop_first(29)
//         .drop_last(10)
//         .derivative();
//     println!("Propose Time:");
//     propose_time.print_stats();
//     propose_time.show_histogram(n_slots as usize / 5, 10);
//     println!();
//
//     // let enter_time = enter_time
//     //     .build()
//     //     .await
//     //     .sort()
//     //     .drop_first(19)
//     //     .drop_last(10)
//     //     .derivative();
//     // println!("Enter Time:");
//     // enter_time.print_stats();
//     // enter_time.show_histogram(n_slots as usize / 5, 10);
//     // println!();
//
//     let batch_commit_time = batch_commit_time
//         .build()
//         .await
//         .filter(|&(depth, _)| depth >= 20)
//         .map(|(_, time)| time)
//         .sort();
//     println!("Batch commit time:");
//     batch_commit_time.print_stats();
//     batch_commit_time.show_histogram(n_slots as usize / 10, 10);
//     println!();
//
//     println!("Indirectly Committed Slots:");
//     let indirectly_committed_slots = indirectly_committed_slots
//         .build()
//         .await
//         .filter(|&slot| slot >= 20);
//     println!("\tCount: {}", indirectly_committed_slots.len());
//     println!(
//         "\tList: {:?}",
//         indirectly_committed_slots.clone().into_vec()
//     );
//     indirectly_committed_slots
//         .map(|slot| slot as f64) // histogram is supported only for f64.
//         .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
//     println!();
// }
//
// async fn test_multichain_with_random_crashes(
//     delay_function: impl DelayFunction,
//     n_nodes: usize,
//     slots_per_delta: Slot,
//     delta: f64,
//     n_slots: Slot,
//     optimistic_dissemination: bool,
//     n_crashes: usize,
//     crash_slot: Slot,
//     // choose one arbitrary correct node to monitor it more closely.
//     monitored_node: NodeId,
// ) {
//     let mut nodes = (0..n_nodes).collect::<Vec<_>>();
//     nodes.shuffle(&mut thread_rng());
//     let crashes = nodes
//         .into_iter()
//         .take(n_crashes)
//         .map(|node_id| (node_id, crash_slot))
//         .collect();
//
//     test_multichain(
//         delay_function,
//         n_nodes,
//         slots_per_delta,
//         delta,
//         n_slots,
//         optimistic_dissemination,
//         crashes,
//         monitored_node,
//     )
//     .await;
// }
//
// async fn test_multichain_with_consecutive_faulty_leaders_in_a_chain(
//     delay_function: impl DelayFunction,
//     n_nodes: usize,
//     slots_per_delta: Slot,
//     delta: f64,
//     n_slots: Slot,
//     optimistic_dissemination: bool,
//     n_crashes: usize,
//     crash_slot: Slot,
//     // choose one arbitrary correct node to monitor it more closely.
//     monitored_node: NodeId,
// ) {
//     let n_chains = PBFT_TIMEOUT * slots_per_delta as u32;
//     let crashes = (0..n_crashes)
//         .map(|i| ((n_chains as NodeId * i as NodeId) % n_nodes, crash_slot))
//         .collect();
//
//     test_multichain(
//         delay_function,
//         n_nodes,
//         slots_per_delta,
//         delta,
//         n_slots,
//         optimistic_dissemination,
//         crashes,
//         monitored_node,
//     )
//     .await;
// }
//
// async fn test_jolteon(
//     delay_function: impl DelayFunction,
//     n_nodes: usize,
//     delta: f64,
//     warmup_duration_in_delta: u32,
//     total_duration_in_delta: u32,
//     // crashes: Vec<(NodeId, Slot)>,
//     // choose one arbitrary correct node to monitor it more closely.
//     monitored_node: NodeId,
// ) {
//     // if 3 * crashes.len() + 1 > n_nodes {
//     //     println!("WARNING: too many crashes, the protocol may stall.");
//     // }
//
//     let spawn_delay_distr =
//         rand_distr::Uniform::new(1. * delta, warmup_duration_in_delta as f64 * delta);
//     let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();
//
//     let mut network = InjectedLocalNetwork::new(n_nodes, network_injection(delay_function));
//
//     let config = jolteon::Config {
//         n_nodes,
//         f: (n_nodes - 1) / 3,
//         leader_timeout: JOLTEON_TIMEOUT,
//         leader_schedule: round_robin(n_nodes),
//         delta: Duration::from_secs_f64(delta),
//         end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
//     };
//
//     let mut join_handles = Vec::new();
//
//     // Semaphore is used to track the number of nodes that have started.
//     let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
//     // let mut propose_time = metrics::UnorderedBuilder::new();
//     // let mut enter_time = metrics::UnorderedBuilder::new();
//     // let mut batch_commit_time = metrics::UnorderedBuilder::new();
//     // let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();
//
//     let start_time = Instant::now();
//     for node_id in 0..n_nodes {
//         let config = config.clone();
//         let network_service = network.service(node_id);
//
//         let clock_speed = { thread_rng().sample(clock_speed_distr) };
//         let timer = InjectedTimerService::local(move |duration, event| {
//             (
//                 Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
//                 event,
//             )
//         });
//
//         // let propose_time_sender = Some(propose_time.new_sender());
//         // let enter_time_sender = if node_id == monitored_node {
//         //     Some(enter_time.new_sender())
//         // } else {
//         //     None
//         // };
//         // let batch_commit_time_sender = Some(batch_commit_time.new_sender());
//         // let indirectly_committed_slots_sender = if node_id == monitored_node {
//         //     Some(indirectly_committed_slots.new_sender())
//         // } else {
//         //     None
//         // };
//
//         let semaphore = semaphore.clone();
//         join_handles.push(spawn(async move {
//             // Sleep for a random duration before spawning the node.
//             let spawn_delay = {
//                 let mut rng = thread_rng();
//                 rng.sample(spawn_delay_distr)
//             };
//             time::sleep(Duration::from_secs_f64(spawn_delay)).await;
//
//             // // Before starting the node, "drop" all messages sent to it during the spawn delay.
//             // network_service.clear_inbox().await;
//
//             // TODO: add actual transactions
//             let (_, txns_receiver) = mpsc::channel::<()>(100);
//
//             // println!("Spawning node {node_id}");
//             let node = Arc::new(tokio::sync::Mutex::new(jolteon::JolteonNode::new(
//                 node_id,
//                 config,
//                 txns_receiver,
//                 start_time,
//                 node_id == monitored_node,
//             )));
//
//             semaphore.add_permits(1);
//             Protocol::run(node, node_id, network_service, timer).await
//         }));
//     }
//
//     let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
//     println!("All nodes are running!");
//
//     for join_handle in join_handles {
//         join_handle.await.unwrap();
//     }
//
//     // let propose_time = propose_time
//     //     .build()
//     //     .await
//     //     .sort()
//     //     .drop_first(29)
//     //     .drop_last(10)
//     //     .derivative();
//     // println!("Propose Time:");
//     // propose_time.print_stats();
//     // propose_time.show_histogram(n_slots as usize / 5, 10);
//     // println!();
//
//     // let enter_time = enter_time
//     //     .build()
//     //     .await
//     //     .sort()
//     //     .drop_first(19)
//     //     .drop_last(10)
//     //     .derivative();
//     // println!("Enter Time:");
//     // enter_time.print_stats();
//     // enter_time.show_histogram(n_slots as usize / 5, 10);
//     // println!();
//
//     // let batch_commit_time = batch_commit_time
//     //     .build()
//     //     .await
//     //     .filter(|&(depth, _)| depth >= 20)
//     //     .map(|(_, time)| time)
//     //     .sort();
//     // println!("Batch commit time:");
//     // batch_commit_time.print_stats();
//     // batch_commit_time.show_histogram(n_slots as usize / 10, 10);
//     // println!();
//
//     // println!("Indirectly Committed Slots:");
//     // let indirectly_committed_slots = indirectly_committed_slots
//     //     .build()
//     //     .await
//     //     .filter(|&slot| slot >= 20);
//     // println!("\tCount: {}", indirectly_committed_slots.len());
//     // println!(
//     //     "\tList: {:?}",
//     //     indirectly_committed_slots.clone().into_vec()
//     // );
//     // indirectly_committed_slots
//     //     .map(|slot| slot as f64) // histogram is supported only for f64.
//     //     .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
//     // println!();
// }
//
// async fn test_jolteon_with_fast_qs(
//     delay_function: impl DelayFunction,
//     n_nodes: usize,
//     delta: f64,
//     warmup_duration_in_delta: u32,
//     total_duration_in_delta: u32,
//     // crashes: Vec<(NodeId, Slot)>,
//     // choose one arbitrary correct node to monitor it more closely.
//     monitored_node: NodeId,
// ) {
//     // if 3 * crashes.len() + 1 > n_nodes {
//     //     println!("WARNING: too many crashes, the protocol may stall.");
//     // }
//
//     let spawn_delay_distr =
//         rand_distr::Uniform::new(1. * delta, warmup_duration_in_delta as f64 * delta);
//     let clock_speed_distr = rand_distr::Normal::new(1., 0.01).unwrap();
//
//     let mut network = InjectedLocalNetwork::new(n_nodes, network_injection(delay_function));
//
//     let f = (n_nodes - 1) / 3;
//
//     let config = jolteon_fast_qs::Config {
//         n_nodes,
//         f,
//         storage_requirement: f + (f / 2 + 1),
//         leader_timeout: JOLTEON_TIMEOUT,
//         leader_schedule: round_robin(n_nodes),
//         delta: Duration::from_secs_f64(delta),
//         batch_interval: Duration::from_secs_f64(delta * 0.1),
//         end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
//     };
//
//     let mut join_handles = Vec::new();
//
//     // Semaphore is used to track the number of nodes that have started.
//     let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
//     // let mut propose_time = metrics::UnorderedBuilder::new();
//     // let mut enter_time = metrics::UnorderedBuilder::new();
//     let mut batch_commit_time = metrics::UnorderedBuilder::new();
//     // let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();
//
//     let start_time = Instant::now();
//     for node_id in 0..n_nodes {
//         let config = config.clone();
//         let network_service = network.service(node_id);
//
//         let clock_speed = { thread_rng().sample(clock_speed_distr) };
//         let timer = InjectedTimerService::local(move |duration, event| {
//             (
//                 Duration::from_secs_f64(duration.as_secs_f64() / clock_speed),
//                 event,
//             )
//         });
//
//         // let propose_time_sender = Some(propose_time.new_sender());
//         // let enter_time_sender = if node_id == monitored_node {
//         //     Some(enter_time.new_sender())
//         // } else {
//         //     None
//         // };
//         let batch_commit_time_sender = Some(batch_commit_time.new_sender());
//         // let indirectly_committed_slots_sender = if node_id == monitored_node {
//         //     Some(indirectly_committed_slots.new_sender())
//         // } else {
//         //     None
//         // };
//
//         let semaphore = semaphore.clone();
//         join_handles.push(spawn(async move {
//             // Sleep for a random duration before spawning the node.
//             let spawn_delay = {
//                 let mut rng = thread_rng();
//                 rng.sample(spawn_delay_distr)
//             };
//             time::sleep(Duration::from_secs_f64(spawn_delay)).await;
//
//             // // Before starting the node, "drop" all messages sent to it during the spawn delay.
//             // network_service.clear_inbox().await;
//
//             // TODO: add actual transactions
//             let next_txn = || ();
//
//             // println!("Spawning node {node_id}");
//             let node = Arc::new(tokio::sync::Mutex::new(jolteon_fast_qs::JolteonNode::new(
//                 node_id,
//                 config,
//                 next_txn,
//                 start_time,
//                 node_id == monitored_node,
//                 jolteon_fast_qs::Metrics {
//                     // propose_time: propose_time_sender,
//                     // enter_time: enter_time_sender,
//                     batch_commit_time: batch_commit_time_sender,
//                     // indirectly_committed_slots: indirectly_committed_slots_sender,
//                 },
//             )));
//
//             semaphore.add_permits(1);
//             Protocol::run(node, node_id, network_service, timer).await
//         }));
//     }
//
//     let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
//     println!("All nodes are running!");
//
//     for join_handle in join_handles {
//         join_handle.await.unwrap();
//     }
//
//     // let propose_time = propose_time
//     //     .build()
//     //     .await
//     //     .sort()
//     //     .drop_first(29)
//     //     .drop_last(10)
//     //     .derivative();
//     // println!("Propose Time:");
//     // propose_time.print_stats();
//     // propose_time.show_histogram(n_slots as usize / 5, 10);
//     // println!();
//
//     // let enter_time = enter_time
//     //     .build()
//     //     .await
//     //     .sort()
//     //     .drop_first(19)
//     //     .drop_last(10)
//     //     .derivative();
//     // println!("Enter Time:");
//     // enter_time.print_stats();
//     // enter_time.show_histogram(n_slots as usize / 5, 10);
//     // println!();
//
//     let batch_commit_time = batch_commit_time
//         .build()
//         .await
//         .filter(|&(time, _)| {
//             time >= start_time + Duration::from_secs_f64(delta) * (2 * warmup_duration_in_delta)
//         })
//         .map(|(_, time)| time)
//         .sort();
//     println!("Batch commit time:");
//     batch_commit_time.print_stats();
//     batch_commit_time.show_histogram(30, 10);
//     println!();
//
//     // println!("Indirectly Committed Slots:");
//     // let indirectly_committed_slots = indirectly_committed_slots
//     //     .build()
//     //     .await
//     //     .filter(|&slot| slot >= 20);
//     // println!("\tCount: {}", indirectly_committed_slots.len());
//     // println!(
//     //     "\tList: {:?}",
//     //     indirectly_committed_slots.clone().into_vec()
//     // );
//     // indirectly_committed_slots
//     //     .map(|slot| slot as f64) // histogram is supported only for f64.
//     //     .show_histogram_range(n_slots as usize / 5, 5, 20., n_slots as f64);
//     // println!();
// }

async fn test_raikou(
    delay_function: impl DelayFunction + Clone,
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

    let mut diss_network =
        InjectedLocalNetwork::new(n_nodes, network_injection(delay_function.clone()));
    let mut network = InjectedLocalNetwork::new(n_nodes, network_injection(delay_function));

    let f = (n_nodes - 1) / 3;
    let poa_quorum = 2 * f + 1;

    let config = raikou::Config {
        n_nodes,
        f,
        storage_requirement: f + 1, // f + (f / 2 + 1),
        leader_timeout: Duration::from_secs_f64(delta * 4.5),
        delta: Duration::from_secs_f64(delta),
        end_of_run: Instant::now() + Duration::from_secs_f64(delta) * total_duration_in_delta,
        extra_wait_before_qc_vote: Duration::from_secs_f64(delta * 0.1),
        enable_partial_qc_votes: true,
        enable_commit_votes: true,
        status_interval: Duration::from_secs_f64(delta) * 10,
        round_sync_interval: Duration::from_secs_f64(delta * 15.),
        block_fetch_multiplicity: std::cmp::min(2, n_nodes),
        block_fetch_interval: Duration::from_secs_f64(delta) * 2,
        poa_quorum,
    };

    let mut join_handles = Vec::new();

    // Semaphore is used to track the number of nodes that have started.
    let semaphore = Arc::new(tokio::sync::Semaphore::new(0));
    // let mut propose_time = metrics::UnorderedBuilder::new();
    // let mut enter_time = metrics::UnorderedBuilder::new();
    let mut batch_commit_time = metrics::UnorderedBuilder::new();
    let mut queueing_time = metrics::UnorderedBuilder::new();
    let mut penalty_wait_time = metrics::UnorderedBuilder::new();
    let mut block_consensus_latency = metrics::UnorderedBuilder::new();
    let mut batch_consensus_latency = metrics::UnorderedBuilder::new();
    let mut batch_execute_time = metrics::UnorderedBuilder::new();
    let mut fetch_wait_time_after_commit = metrics::UnorderedBuilder::new();
    // let mut indirectly_committed_slots = metrics::UnorderedBuilder::new();
    let executed_txns_counter = Arc::new(AtomicUsize::new(0));

    let private_keys: Vec<_> = (0..n_nodes)
        .map(|node_id| {
            use aptos_crypto::traits::Uniform;
            Arc::new(PrivateKey::generate(&mut thread_rng()))
        })
        .collect();

    let public_keys: Vec<_> = (0..n_nodes)
        .map(|node_id| PublicKey::from(private_keys[node_id].deref()))
        .collect();

    let start_time = Instant::now();
    for node_id in 0..n_nodes {
        let config = config.clone();

        let sig_verifier = SignatureVerifier::new(
            public_keys.clone(),
            // Not going to be actually used with --features sim-types.
            Arc::new(ValidatorVerifier::new(vec![])),
            N_SUB_BLOCKS + 1,
        );

        let signer = Signer::new(
            Arc::new(ValidatorSigner::new(
                AccountAddress::new([node_id as u8; 32]), // this is not actually used.
                private_keys[node_id].clone(),
            )),
            node_id,
            N_SUB_BLOCKS + 1,
        );

        let mut diss_network_service = diss_network.service(
            node_id,
            Arc::new(dissemination::native::Certifier::new(signer.clone())),
        );
        let mut network_service =
            network.service(node_id, Arc::new(raikou::protocol::Certifier::new()));

        let clock_speed = { thread_rng().sample(clock_speed_distr) };

        // introduce artificial clock skew.
        let diss_timer = InjectedTimerService::local(clock_skew_injection(clock_speed));
        let timer = InjectedTimerService::local(clock_skew_injection(clock_speed));

        // let propose_time_sender = Some(propose_time.new_sender());
        // let enter_time_sender = if node_id == monitored_node {
        //     Some(enter_time.new_sender())
        // } else {
        //     None
        // };
        let batch_commit_time_sender = Some(batch_commit_time.new_sender());
        let queueing_time_sender = Some(queueing_time.new_sender());
        let penalty_wait_time_sender = Some(penalty_wait_time.new_sender());
        let block_consensus_latency_sender = Some(block_consensus_latency.new_sender());
        let batch_consensus_latency_sender = Some(batch_consensus_latency.new_sender());
        let batch_execute_time_sender = Some(batch_execute_time.new_sender());
        let fetch_wait_time_after_commit_sender = Some(fetch_wait_time_after_commit.new_sender());
        let executed_txns_counter = executed_txns_counter.clone();
        // let indirectly_committed_slots_sender = if node_id == monitored_node {
        //     Some(indirectly_committed_slots.new_sender())
        // } else {
        //     None
        // };

        let semaphore = semaphore.clone();
        join_handles.push(tokio::spawn(async move {
            // Sleep for a random duration before spawning the node.
            let spawn_delay = {
                let mut rng = thread_rng();
                rng.sample(spawn_delay_distr)
            };
            time::sleep(Duration::from_secs_f64(spawn_delay)).await;

            // Before starting the node, "drop" all messages sent to it during the spawn delay.
            network_service.clear_inbox().await;
            diss_network_service.clear_inbox().await;

            let txns_iter = iter::repeat_with(|| vec![]);

            let mut module_network = ModuleNetwork::new();
            let diss_module_network = module_network.register().await;
            let cons_module_network = module_network.register().await;

            let (execute_tx, mut execute_rx) = tokio::sync::mpsc::channel::<Batch>(1024);

            let executed_txns_counter = executed_txns_counter.clone();
            tokio::spawn(async move {
                while let Some(batch) = execute_rx.recv().await {
                    if node_id == monitored_node {
                        executed_txns_counter.fetch_add(batch.txns().len(), Ordering::SeqCst);
                    }
                }
            });

            let batch_interval_secs = delta * 0.1;
            let expected_load =
                f64::ceil(n_nodes as f64 * (3. * delta) / batch_interval_secs) as usize;

            let dissemination = NativeDisseminationLayer::new(
                node_id,
                dissemination::native::Config {
                    module_id: diss_module_network.module_id(),
                    n_nodes,
                    f,
                    poa_quorum,
                    delta: Duration::from_secs_f64(delta),
                    batch_interval: Duration::from_secs_f64(batch_interval_secs),
                    enable_optimistic_dissemination,
                    enable_penalty_tracker: true,
                    penalty_tracker_report_delay: Duration::from_secs_f64(delta * 5.),
                    batch_fetch_multiplicity: std::cmp::min(2, n_nodes),
                    batch_fetch_interval: Duration::from_secs_f64(delta) * 2,
                    status_interval: Duration::from_secs_f64(delta) * 10,
                    block_size_limit:
                        dissemination::native::BlockSizeLimit::from_max_number_of_poas(
                            f64::ceil(expected_load as f64 * 1.5) as usize,
                            n_nodes,
                        ),
                },
                txns_iter,
                cons_module_network.module_id(),
                node_id == monitored_node,
                dissemination::Metrics {
                    batch_commit_time: batch_commit_time_sender,
                    queueing_time: queueing_time_sender,
                    penalty_wait_time: penalty_wait_time_sender,
                    batch_execute_time: batch_execute_time_sender,
                    fetch_wait_time_after_commit: fetch_wait_time_after_commit_sender,
                },
                signer.clone(),
                sig_verifier.clone(),
                execute_tx,
            );

            // println!("Spawning node {node_id}");
            let node = Arc::new(tokio::sync::Mutex::new(RaikouNode::new(
                node_id,
                config,
                dissemination.clone(),
                node_id == monitored_node,
                raikou::Metrics {
                    block_consensus_latency: block_consensus_latency_sender,
                    batch_consensus_latency: batch_consensus_latency_sender,
                    // propose_time: propose_time_sender,
                    // enter_time: enter_time_sender,
                    // indirectly_committed_slots: indirectly_committed_slots_sender,
                },
                signer,
                sig_verifier,
                None, // failure_tracker
            )));

            semaphore.add_permits(1);

            tokio::spawn(Protocol::run(
                dissemination.protocol(),
                node_id,
                diss_network_service,
                diss_module_network,
                diss_timer,
            ));

            Protocol::run(node, node_id, network_service, cons_module_network, timer).await;
            println!("Node {} finished", node_id);
        }));
    }

    let _ = semaphore.acquire_many(n_nodes as u32).await.unwrap();
    println!("All nodes are running!");

    for join_handle in join_handles {
        join_handle.await.unwrap();
    }
    println!("All nodes finished");

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

    display_metric(
        "Fetch wait time after commit",
        "The duration from committing a block until being able to execute it, i.e.,\
        until we have the whole prefix of the chain fetched.",
        fetch_wait_time_after_commit,
        start_time,
        delta,
        warmup_period_in_delta,
    )
    .await;

    display_metric(
        "Penalty system delay",
        "The penalties for optimistically committed batches. \
        Measured on the leader.",
        penalty_wait_time,
        start_time,
        delta,
        warmup_period_in_delta,
    )
    .await;

    display_metric(
        "Optimistic batch queueing time",
        "The duration from when the batch is received by leader until the block \
        containing this batch is proposed. \
        Only measured if the block is committed. \
        Only measured for optimistically committed batches. \
        Measured on the leader.",
        queueing_time,
        start_time,
        delta,
        warmup_period_in_delta,
    )
    .await;

    display_metric(
        "Batch consensus latency",
        "The duration from when the batch is included in a block until \
        the block is committed. \
        Measured on the leader.",
        batch_consensus_latency,
        start_time,
        delta,
        warmup_period_in_delta,
    )
    .await;

    display_metric(
        "Batch commit time",
        "The duration from creating the batch until committing it. \
        After committing, we may have to wait for the data to be fetched. \
        Measured on the batch creator.",
        batch_commit_time,
        start_time,
        delta,
        warmup_period_in_delta,
    )
    .await;

    display_metric(
        "Batch execute time (the end-to-end latency)",
        "The duration from creating the batch until executing it. \
        Measured on the batch creator.",
        batch_execute_time,
        start_time,
        delta,
        warmup_period_in_delta,
    )
    .await;

    println!(
        "Executed transactions: {}",
        executed_txns_counter.load(Ordering::SeqCst)
    );
}

pub async fn main() {
    // set up logging
    // env_logger::builder()
    //     .filter_level(log::LevelFilter::Info)
    //     .format_target(false)
    //     .format_timestamp(None)
    //     .init();
    aptos_logger::Logger::builder()
        .level(aptos_logger::Level::Info)
        .build();

    let n_nodes = 31;
    let delta = 1.;
    let spawn_period_in_delta = 10;
    let warmup_period_in_delta = 70;
    let total_duration_in_delta = 150;
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
            // the mean delay between a pair of nodes is uniformly sampled between 0 and 0.9 delta.
            rand_distr::Uniform::new(0., 0.9 * delta),
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
