// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use anyhow::{anyhow, Result};
use aptos_forge::{
    test_utils::consensus_utils::{
        test_consensus_fault_tolerance, FailureInjection,
    },
    GroupNetEm, NetworkContext, NetworkContextSynchronizer, NetworkTest, Swarm, SwarmChaos,
    SwarmExt, SwarmNetEm, Test,
};
use aptos_rest_client::Client as RestClient;
use aptos_types::PeerId;
use async_trait::async_trait;
use log::info;
use std::sync::Arc;

/// Test scenario with 4 nodes:
/// - Round 1: Normal operation (all nodes communicate)
/// - Round 2: Network split into two partitions (nodes 0,1 vs nodes 2,3)
/// - Test ends after 2 rounds
pub struct FourNodePartitionScenario {
    pub round_duration_secs: f32,
}

impl Default for FourNodePartitionScenario {
    fn default() -> Self {
        Self {
            round_duration_secs: 10.0, // 10 seconds per round
        }
    }
}

impl Test for FourNodePartitionScenario {
    fn name(&self) -> &'static str {
        "four node partition scenario"
    }
}

#[async_trait]
impl NetworkTest for FourNodePartitionScenario {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        let swarm = {
            let ctx_guard = ctx.ctx.lock().await;
            ctx_guard.swarm.clone()
        };

        let validator_clients = {
            swarm.read().await.get_validator_clients_with_names()
        };

        let public_info = {
            swarm.read().await.aptos_public_info()
        };

        // Ensure we have exactly 4 validators
        if validator_clients.len() != 4 {
            return Err(anyhow!(
                "This test requires exactly 4 validators, but found {}",
                validator_clients.len()
            ));
        }

        info!("Starting 4-node partition scenario test");
        info!("Round 1: Normal operation (10 seconds)");
        info!("Round 2: Network partition - nodes [0,1] vs [2,3] (10 seconds)");

        // Get validator peer IDs for partition configuration
        let validator_peer_ids: Vec<PeerId> = {
            swarm
                .read()
                .await
                .validators()
                .map(|v| v.peer_id())
                .collect()
        };

        // Create the failure injection for our specific scenario
        let scenario_injection = Box::new(FourNodeScenarioFailureInjection::new(
            swarm.clone(),
            validator_peer_ids,
        ));

        // Track progress through the scenario
        let mut round_states = Vec::new();

        test_consensus_fault_tolerance(
            validator_clients,
            public_info,
            2,                           // 2 cycles (rounds)
            self.round_duration_secs,    // duration per cycle
            1,                           // 1 part per cycle (simplified)
            scenario_injection,
            Box::new(move |cycle, executed_epochs, executed_rounds, executed_transactions, current_state, previous_state| {
                info!(
                    "Round {} completed: epochs={}, rounds={}, transactions={}",
                    cycle + 1, executed_epochs, executed_rounds, executed_transactions
                );
                
                // Log node states
                for (i, state) in current_state.iter().enumerate() {
                    info!(
                        "Node {}: version={}, epoch={}, round={}",
                        i, state.version, state.epoch, state.round
                    );
                }

                // Store states for analysis
                round_states.push((cycle, current_state.clone(), previous_state.clone()));

                // Validate scenario expectations
                match cycle {
                    0 => {
                        // Round 1: All nodes should make normal progress
                        info!("Validating Round 1: Normal operation");
                        let progress: Vec<u64> = current_state.iter()
                            .zip(previous_state.iter())
                            .map(|(curr, prev)| curr.round.saturating_sub(prev.round))
                            .collect();
                        
                        info!("Round progress per node: {:?}", progress);
                        
                        // All nodes should have made some progress
                        if !progress.iter().all(|&p| p > 0) {
                            return Err(anyhow!("Not all nodes made progress in Round 1"));
                        }
                    },
                    1 => {
                        // Round 2: Nodes should show effects of partition
                        info!("Validating Round 2: Partition effects");
                        
                        // Check if nodes are still making progress despite partition
                        let progress: Vec<u64> = current_state.iter()
                            .zip(previous_state.iter())
                            .map(|(curr, prev)| curr.round.saturating_sub(prev.round))
                            .collect();
                        
                        info!("Round progress per node during partition: {:?}", progress);
                        
                        // During partition, progress might be slower but should still occur
                        // as each partition has 2 nodes (not enough for 3f+1 consensus with f=1)
                        // but the test will show the behavior
                        info!("Partition test completed - progress: {:?}", progress);
                    },
                    _ => {
                        info!("Unexpected round: {}", cycle);
                    }
                }

                Ok(())
            }),
            false, // new_epoch_on_cycle
            false, // raise_check_error_at_the_end
        ).await?;

        info!("Four-node partition scenario completed successfully!");
        
        // Report final summary
        ctx.report_text(format!(
            "Four-node partition scenario completed:\n\
             - Round 1: Normal operation with 4 nodes\n\
             - Round 2: Network partition (nodes 0,1 vs 2,3)\n\
             - Total rounds processed: 2"
        )).await;

        Ok(())
    }
}

/// Custom failure injection for the 4-node partition scenario
struct FourNodeScenarioFailureInjection {
    swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
    validator_peer_ids: Vec<PeerId>,
    partition_applied: bool,
}

impl FourNodeScenarioFailureInjection {
    fn new(
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        validator_peer_ids: Vec<PeerId>,
    ) -> Self {
        Self {
            swarm,
            validator_peer_ids,
            partition_applied: false,
        }
    }

    async fn apply_partition(&mut self) -> Result<()> {
        info!("Applying network partition: [0,1] vs [2,3]");
        
        // Create partition using NetEm chaos to block communication between groups
        let group_netems = vec![
            // Block communication from nodes 0,1 to nodes 2,3
            GroupNetEm {
                name: "partition_group_a_to_b".to_string(),
                source_nodes: vec![self.validator_peer_ids[0], self.validator_peer_ids[1]],
                target_nodes: vec![self.validator_peer_ids[2], self.validator_peer_ids[3]],
                delay_latency_ms: 0,
                delay_jitter_ms: 0,
                delay_correlation_percentage: 0,
                loss_percentage: 100, // 100% packet loss = complete partition
                loss_correlation_percentage: 0,
                rate_in_mbps: 0,
            },
            // Block communication from nodes 2,3 to nodes 0,1
            GroupNetEm {
                name: "partition_group_b_to_a".to_string(),
                source_nodes: vec![self.validator_peer_ids[2], self.validator_peer_ids[3]],
                target_nodes: vec![self.validator_peer_ids[0], self.validator_peer_ids[1]],
                delay_latency_ms: 0,
                delay_jitter_ms: 0,
                delay_correlation_percentage: 0,
                loss_percentage: 100, // 100% packet loss = complete partition
                loss_correlation_percentage: 0,
                rate_in_mbps: 0,
            },
        ];

        let chaos = SwarmChaos::NetEm(SwarmNetEm { group_netems });
        
        self.swarm.write().await.inject_chaos(chaos).await?;
        self.partition_applied = true;
        
        info!("Network partition applied successfully");
        Ok(())
    }

    async fn remove_partition(&mut self) -> Result<()> {
        if self.partition_applied {
            info!("Removing network partition");
            self.swarm.write().await.remove_all_chaos().await?;
            self.partition_applied = false;
            info!("Network partition removed");
        }
        Ok(())
    }
}

#[async_trait]
impl FailureInjection for FourNodeScenarioFailureInjection {
    async fn inject(
        &mut self,
        _validator_clients: &[(String, RestClient)],
        cycle: usize,
        part: usize,
    ) {
        match (cycle, part) {
            (0, 0) => {
                // Round 1: Normal operation - ensure no partitions
                info!("Round 1 (cycle {}): Normal operation", cycle);
                if let Err(e) = self.remove_partition().await {
                    eprintln!("Failed to ensure no partition in Round 1: {:?}", e);
                }
            },
            (1, 0) => {
                // Round 2: Apply partition
                info!("Round 2 (cycle {}): Applying network partition", cycle);
                if let Err(e) = self.apply_partition().await {
                    eprintln!("Failed to apply partition in Round 2: {:?}", e);
                }
            },
            _ => {
                // Should not reach here with our 2-round scenario
                info!("Unexpected cycle/part: {}/{}", cycle, part);
            }
        }
    }

    async fn clear(&mut self, _validator_clients: &[(String, RestClient)]) {
        info!("Cleaning up - removing all network chaos");
        if let Err(e) = self.remove_partition().await {
            eprintln!("Failed to clear partition: {:?}", e);
        }
    }
}

#[async_trait]
impl NetworkLoadTest for FourNodePartitionScenario {
    async fn setup<'a>(&self, _ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        // Send load to all validators to test behavior under partition
        Ok(LoadDestination::AllValidators)
    }
}
