// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use core::time;
use std::{thread, sync::Arc, time::Instant, collections::HashSet};

use consensus::analyze_leader_selection::AnalyzeLeaderSelection;
use forge::{NodeExt, Swarm};

use crate::{
    smoke_test_environment::{new_local_swarm_with_aptos, new_local_swarm_with_aptos_and_config},
    test_utils::{create_and_fund_account, transfer_coins_non_blocking, transfer_coins}, leader_election::FetchMetadata,
};
use aptos_api_types::Transaction;
use aptos_rest_client::{aptos_api_types, Client as RestClient, state::State};
use std::convert::TryFrom;
use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::NewBlockEvent;
use std::collections::HashMap;
use rand::{rngs::SmallRng, SeedableRng};
use futures::future::join_all;
use rand::{self, Rng}; 


async fn run_fail_point_test(
    num_nodes: usize, 
    cycles: usize, 
    cycle_duration_s: f32, 
    parts_in_cycle: usize, 
    // (cycle, part) -> (Vec(validator_index, name, action), reset_old_enpoints)
    mut get_fail_points_to_set: Box<dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool)>,
    // (cycle, current_state, previous_state)
    mut check_cycle: Box<dyn FnMut(usize, Vec<State>, Vec<State>)>) {
    let mut swarm = new_local_swarm_with_aptos_and_config(num_nodes, Arc::new(|i, config| {
        config.consensus.max_block_size = 1;
    })).await;

    let mut validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    validator_peer_ids.sort();
    println!("Swarm started for dir {}", swarm.dir().to_string_lossy());
    println!("Validators {:?}", validator_peer_ids);

    let validator_clients: Vec<RestClient> = validator_peer_ids.iter()
        .map(|validator| {
            swarm
                .validator(*validator)
                .unwrap()
                .rest_client()
        })
        .collect();
    let validator_client_0 = &validator_clients.get(0).unwrap();

    let transaction_factory = swarm.chain_info().transaction_factory();
    let mut account_0 = create_and_fund_account(&mut swarm, 100000).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    let mut modified_failpoints: HashSet<(usize, String)> = HashSet::new();

    let mut small_rng = SmallRng::from_entropy();

    let mut previous = join_all(validator_clients.iter().cloned().map(move |v| async move {v.get_ledger_information().await.unwrap().into_inner()})).await; 
    for i in 0..cycles {
        let now = Instant::now();
        for p in 0..parts_in_cycle {
            let (fail_points_to_set, reset_old_failpoints) = get_fail_points_to_set(i, p);
            if reset_old_failpoints {
                let actions = "0%return".to_string();
                for (validator_idx, name) in modified_failpoints {
                    println!("Setting client {} failpoint {}={}", validator_idx, name, actions);
                    validator_clients[validator_idx].set_failpoint(
                            name,
                            actions.clone(),
                        )
                        .await
                        .unwrap();
                }
                modified_failpoints = HashSet::new();
            }
            for (validator_idx, name, actions) in fail_points_to_set {
                validator_clients[validator_idx]
                .set_failpoint(
                    name.clone(),
                    actions.clone(),
                )
                .await
                .unwrap();
                println!("Setting client {} failpoint {}={}", validator_idx, name, actions);
                modified_failpoints.insert((validator_idx, name));
            }

            transfer_coins_non_blocking(
                &validator_clients[small_rng.gen_range(0usize, validator_clients.len())],
                &transaction_factory,
                &mut account_0,
                &account_1,
                10,
            )
            .await;

            let elapsed = now.elapsed().as_secs_f32();
            let wanted = (1 + p) as f32 * cycle_duration_s / (parts_in_cycle as f32);
            if elapsed < wanted {
                thread::sleep(time::Duration::from_secs_f32(wanted - elapsed));
            }
        }

        let cur = join_all(validator_clients.iter().cloned().map(move |v| async move {v.get_ledger_information().await.unwrap().into_inner()})).await;

        println!(
            "cycle {} lasted {:.3} with {} transactions and {} rounds", 
            i, 
            now.elapsed().as_secs_f32(), 
            cur.iter().map(|s| s.version).max().unwrap() - previous.iter().map(|s| s.version).max().unwrap(), 
            cur.iter().map(|s| s.round).max().unwrap() - previous.iter().map(|s| s.round).max().unwrap());
        println!("All at versions: {:?}", cur.iter().map(|s| s.version).collect::<Vec<_>>());
        println!("All at rounds: {:?}", cur.iter().map(|s| s.round).collect::<Vec<_>>());
        check_cycle(i, cur.clone(), previous);
        previous = cur;
    }

    for (validator_idx, name) in modified_failpoints {
        validator_clients[validator_idx].set_failpoint(
            name,
            "0%return".to_string(),
        )
        .await
        .unwrap();
    }

    thread::sleep(time::Duration::from_secs(2));

    let largest_v = *join_all(validator_clients.iter().cloned().map(move |v| async move {v.get_ledger_information().await.unwrap().into_inner().version})).await.iter().max().unwrap();
    println!("Largest version {}", largest_v);

    transfer_coins(
        validator_client_0,
        &transaction_factory,
        &mut account_0,
        &account_1,
        10,
    )
    .await;

    let largest_v = *join_all(validator_clients.iter().cloned().map(move |v| async move {v.get_ledger_information().await.unwrap().into_inner().version})).await.iter().max().unwrap();
    println!("Largest version {}", largest_v);

    thread::sleep(time::Duration::from_secs(2));

    let transactions : Vec<_> = join_all(validator_clients.iter().cloned().map(move |v| async move {
        let mut txns = v.get_transactions(None, Some(1000))
            .await
            .unwrap()
            .into_inner();
        txns.retain(|t| t.version().unwrap() <= largest_v);
        txns
    })).await;

    for i in 1..transactions.len() {
        assert_eq!(transactions.get(0).unwrap(), transactions.get(i).unwrap());
    }

    let epoch = 2;
    let blocks = FetchMetadata::fetch_new_block_events(epoch, &validator_client_0).await;
    if !validator_peer_ids.is_empty() {
        let events: Vec<NewBlockEvent> =
            blocks.into_iter().filter(|e| e.epoch() == epoch).collect();
        println!("Analyzing epoch : {}", epoch);
        let stats = AnalyzeLeaderSelection::analyze(events, &validator_peer_ids);
        AnalyzeLeaderSelection::print_table(&stats, None, false);
    }

    assert!(false);
}


#[tokio::test]
async fn test_fault_tolerance_of_network() {
    let mut small_rng = SmallRng::from_entropy();
    let num_validators = 7;
    run_fail_point_test(
        7, 10, 5.0, 5, 
        Box::new(move |cycle, part| {
            let rand_reliability = small_rng.gen_range(0usize, cycle + 2);
            let wanted_reliability = (cycle + 1) * 10; // - rand_reliability * rand_reliability;
            let wanted_client = small_rng.gen_range(0usize, num_validators);

            (vec![(wanted_client, "consensus::send_any".to_string(), format!("{}%return", wanted_reliability))], false)
        }),
        Box::new(|_, _, _| {}),
    ).await;
}


#[tokio::test]
async fn test_changing_working_consensus() {
    // with 7 nodes, consensus needs 5 to operate. 
    // we rotate in each cycle, which 2 nodes are down.
    // we should consisnently be seeing progress.
    let num_validators = 7;
    run_fail_point_test(
        7, 8, 3.0, 3, 
        Box::new(move |cycle, part| {
            if part == 0 {
                let client_1 = (cycle * 2) % num_validators;
                let client_2 = (cycle * 2) % num_validators;
                (
                    vec![
                        (client_1, "consensus::send_any".to_string(), "100%return".to_string()),
                        (client_1, "consensus::process_any".to_string(), "100%return".to_string()),
                        (client_2, "consensus::send_any".to_string(), "100%return".to_string()),
                        (client_2, "consensus::process_any".to_string(), "100%return".to_string()),
                    ],
                    true
                )
            } else {
                (vec![], false)
            }
        }), 
        Box::new(|_, cur, prev| {
            assert!(cur.iter().map(|s| s.version).max().unwrap() - prev.iter().map(|s| s.version).max().unwrap() > 2, "no progress with active consensus");
            assert!(cur.iter().map(|s| s.round).max().unwrap() - prev.iter().map(|s| s.round).max().unwrap() > 2, "no progress with active consensus");
        }),
    ).await;
}

