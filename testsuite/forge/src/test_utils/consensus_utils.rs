// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{wait_for_all_nodes_to_catchup_to_version, VelorPublicInfo};
use anyhow::{bail, Context, Result};
use velor_config::config::DEFAULT_MAX_PAGE_SIZE;
use velor_rest_client::Client as RestClient;
use async_trait::async_trait;
use chrono::Utc;
use core::time;
use futures::future::join_all;
use itertools::Itertools;
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub struct NodeState {
    pub version: u64,
    pub epoch: u64,
    pub round: u64,
}

// TODO: check if we can fetch consensus round, not just committed round.
async fn get_node_state(validator_client: &RestClient) -> NodeState {
    let (events, state) = validator_client
        .get_new_block_events_bcs(None, Some(1))
        .await
        .unwrap()
        .into_parts();
    let event = events.first().unwrap();
    assert!(event.version <= state.version);
    NodeState {
        version: state.version,
        epoch: event.event.epoch(),
        round: event.event.round(),
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
pub async fn test_consensus_fault_tolerance(
    // swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
    validator_clients: Vec<(String, RestClient)>,
    public_info: VelorPublicInfo,
    cycles: usize,
    cycle_duration_s: f32,
    parts_in_cycle: usize,
    mut failure_injection: Box<dyn FailureInjection + Send>,
    // (cycle, executed_epochs, executed_rounds, executed_transactions, current_state, previous_state)
    mut check_cycle: Box<
        dyn FnMut(usize, u64, u64, u64, Vec<NodeState>, Vec<NodeState>) -> Result<()> + Send,
    >,
    new_epoch_on_cycle: bool,
    // Instead of failing on first check, we check the full run,
    // and then fail if any checks failed during the run.
    // Can allow us to better see if state would've gotten resolved by itself, etc.
    raise_check_error_at_the_end: bool,
) -> Result<()> {
    async fn get_all_states(validator_clients: &[(String, RestClient)]) -> Vec<NodeState> {
        join_all(
            validator_clients
                .iter()
                .cloned()
                .map(move |(_, v)| async move { get_node_state(&v).await }),
        )
        .await
    }

    let mut errors = Vec::new();

    for cycle in 0..cycles {
        let previous = get_all_states(&validator_clients).await;

        let now = Instant::now();
        for part in 0..parts_in_cycle {
            failure_injection
                .inject(&validator_clients, cycle, part)
                .await;
            let elapsed = now.elapsed().as_secs_f32();
            let wanted = (1 + part) as f32 * cycle_duration_s / (parts_in_cycle as f32);
            if elapsed < wanted {
                tokio::time::sleep(time::Duration::from_secs_f32(wanted - elapsed)).await;
            }
        }

        let cur = get_all_states(&validator_clients).await;

        let (cur_epoch, cur_round) = cur.iter().map(|s| (s.epoch, s.round)).max().unwrap();
        let (prev_epoch, prev_round) = previous.iter().map(|s| (s.epoch, s.round)).max().unwrap();
        let epochs = cur_epoch.saturating_sub(prev_epoch);
        let rounds = cur_round.saturating_sub(prev_round);

        let transactions = cur.iter().map(|s| s.version).max().unwrap()
            - previous.iter().map(|s| s.version).max().unwrap();

        println!(
            "cycle {} lasted {:.3} with {} epochs, {} rounds and {} transactions",
            cycle,
            now.elapsed().as_secs_f32(),
            epochs,
            rounds,
            transactions,
        );
        println!(
            "All at epochs: {:?}, from {:?}",
            cur.iter().map(|s| s.epoch).collect::<Vec<_>>(),
            previous.iter().map(|s| s.epoch).collect::<Vec<_>>(),
        );
        println!(
            "All at rounds: {:?}, from {:?}",
            cur.iter().map(|s| s.round).collect::<Vec<_>>(),
            previous.iter().map(|s| s.round).collect::<Vec<_>>(),
        );
        println!(
            "All at versions: {:?}, from {:?}",
            cur.iter().map(|s| s.version).collect::<Vec<_>>(),
            previous.iter().map(|s| s.version).collect::<Vec<_>>(),
        );

        let check_result = check_cycle(cycle, epochs, rounds, transactions, cur.clone(), previous);
        if raise_check_error_at_the_end {
            if let Err(error) = check_result {
                println!("Failed check {}", error);
                errors.push((error, cycle, Utc::now()));
            }
        } else {
            check_result?;
        }

        if new_epoch_on_cycle {
            public_info.reconfig().await;
        }
    }

    failure_injection.clear(&validator_clients).await;

    let cur = get_all_states(&validator_clients).await;
    println!(
        "All at versions: {:?}",
        cur.iter().map(|s| s.version).collect::<Vec<_>>()
    );
    let largest_v = cur.iter().map(|s| s.version).max().unwrap();
    println!("Largest version {}", largest_v);
    let target_v = largest_v + 10;

    wait_for_all_nodes_to_catchup_to_version(&validator_clients, target_v, Duration::from_secs(30))
        .await
        .context("catchup failed")?;

    let transactions: Vec<_> =
        join_all(validator_clients.iter().cloned().map(move |v| async move {
            let mut txns =
                v.1.get_transactions_bcs(
                    Some(target_v.saturating_sub(DEFAULT_MAX_PAGE_SIZE as u64)),
                    Some(DEFAULT_MAX_PAGE_SIZE),
                )
                .await
                .unwrap()
                .into_inner();
            txns.retain(|t| t.version <= target_v);
            <Result<Vec<_>>>::Ok(txns)
        }))
        .await;

    let txns_a = transactions.first().unwrap().as_ref().unwrap();

    for i in 1..transactions.len() {
        let txns_b = transactions.get(i).unwrap().as_ref().unwrap();
        assert_eq!(
            txns_a.len(),
            txns_b.len(),
            "Fetched length of transactions for target_v {} doesn't match: from {} to {} vs from {} to {}",
            target_v,
            txns_a.first().map(|t| t.version).unwrap_or(0),
            txns_a.last().map(|t| t.version).unwrap_or(0),
            txns_b.first().map(|t| t.version).unwrap_or(0),
            txns_b.last().map(|t| t.version).unwrap_or(0),
        );
        for i in 0..txns_a.len() {
            assert_eq!(
                txns_a[i], txns_b[i],
                "Transaction at index {} after target version {}, doesn't match",
                i, target_v
            );
        }
    }

    if !errors.is_empty() {
        bail!(
            "There were {} check failures during the run: {}",
            errors.len(),
            errors
                .iter()
                .map(|(err, cycle, ts)| format!(
                    "cycle {} at {}: {:?} ",
                    cycle,
                    ts.to_rfc3339(),
                    err
                ))
                .join("\n")
        );
    }
    Ok(())
}

#[async_trait]
pub trait FailureInjection {
    async fn inject(
        &mut self,
        validator_clients: &[(String, RestClient)],
        cycle: usize,
        part: usize,
    );
    async fn clear(&mut self, validator_clients: &[(String, RestClient)]);
}

pub struct NoFailureInjection {}

#[async_trait]
impl FailureInjection for NoFailureInjection {
    async fn inject(&mut self, _: &[(String, RestClient)], _: usize, _: usize) {}

    async fn clear(&mut self, _: &[(String, RestClient)]) {}
}

pub fn no_failure_injection() -> Box<dyn FailureInjection + Send> {
    Box::new(NoFailureInjection {})
}

pub struct FailPointFailureInjection {
    modified_failpoints: HashSet<(usize, String)>,
    // (cycle, part) -> (Vec(validator_index, name, action), reset_old_enpoints)
    get_fail_points_to_set:
        Box<dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool) + Send>,
}

impl FailPointFailureInjection {
    pub fn new(
        get_fail_points_to_set: Box<
            dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool) + Send,
        >,
    ) -> Self {
        Self {
            modified_failpoints: HashSet::new(),
            get_fail_points_to_set,
        }
    }
}

pub fn fail_point_injection(
    get_fail_points_to_set: Box<
        dyn FnMut(usize, usize) -> (Vec<(usize, String, String)>, bool) + Send,
    >,
) -> Box<dyn FailureInjection> {
    Box::new(FailPointFailureInjection::new(get_fail_points_to_set))
}

#[async_trait]
impl FailureInjection for FailPointFailureInjection {
    async fn inject(
        &mut self,
        validator_clients: &[(String, RestClient)],
        cycle: usize,
        part: usize,
    ) {
        let (fail_points_to_set, reset_old_failpoints) = (self.get_fail_points_to_set)(cycle, part);
        if reset_old_failpoints {
            let new_set = fail_points_to_set
                .iter()
                .map(|(validator_idx, name, _actions)| (validator_idx, name))
                .collect::<HashSet<_>>();
            for (validator_idx, name) in self.modified_failpoints.iter() {
                // we don't want to clear failpoints we are setting later,
                // as it can cause race conditions.
                if !new_set.contains(&(validator_idx, name)) {
                    validator_clients[*validator_idx]
                        .1
                        .set_failpoint(name.clone(), "off".to_string())
                        .await
                        .context(validator_clients[*validator_idx].0.clone())
                        .unwrap();
                }
            }
            self.modified_failpoints = HashSet::new();
        }
        for (validator_idx, name, actions) in fail_points_to_set {
            validator_clients[validator_idx]
                .1
                .set_failpoint(name.clone(), actions.clone())
                .await
                .context(validator_clients[validator_idx].0.clone())
                .unwrap();
            self.modified_failpoints.insert((validator_idx, name));
        }
    }

    async fn clear(&mut self, validator_clients: &[(String, RestClient)]) {
        for (validator_idx, name) in self.modified_failpoints.iter() {
            validator_clients[*validator_idx]
                .1
                .set_failpoint(name.clone(), "off".to_string())
                .await
                .context(validator_clients[*validator_idx].0.clone())
                .unwrap();
        }
    }
}
