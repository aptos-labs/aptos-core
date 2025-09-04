// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{multi_region_network_test::chunk_peers, LoadDestination, NetworkLoadTest};
use velor_forge::{
    GroupCpuStress, NetworkContext, NetworkContextSynchronizer, NetworkTest, Swarm, SwarmChaos,
    SwarmCpuStress, SwarmExt, Test,
};
use velor_types::PeerId;
use async_trait::async_trait;
use log::info;
use rand::Rng;
use std::sync::Arc;

async fn add_execution_delay(
    swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
    config: &ExecutionDelayConfig,
) -> anyhow::Result<()> {
    let validators = { swarm.read().await.get_validator_clients_with_names() };

    for (name, validator) in validators {
        let sleep_percentage = {
            let mut rng = rand::thread_rng();
            if rng.gen_bool(config.inject_delay_node_fraction) {
                rng.gen_range(1_u32, config.inject_delay_max_transaction_percentage)
            } else {
                0
            }
        };
        info!(
            "Validator {} adding {}% of transactions with {}ms execution delay",
            name, sleep_percentage, config.inject_delay_per_transaction_ms
        );
        validator
            .set_failpoint(
                "velor_vm::execution::user_transaction".to_string(),
                format!(
                    "{}%delay({})",
                    sleep_percentage, config.inject_delay_per_transaction_ms
                ),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "set_failpoint to add execution delay on {} failed, {:?}",
                    name,
                    e
                )
            })?;
    }
    Ok(())
}

async fn remove_execution_delay(
    swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
) -> anyhow::Result<()> {
    let validators = { swarm.read().await.get_validator_clients_with_names() };

    for (name, validator) in validators {
        validator
            .set_failpoint(
                "velor_vm::execution::block_metadata".to_string(),
                "off".to_string(),
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "set_failpoint to remove execution delay on {} failed, {:?}",
                    name,
                    e
                )
            })?;
    }
    Ok(())
}

/// Config for adding variable processing overhead/delay into
/// execution, to make different nodes have different processing speed.
pub struct ExecutionDelayConfig {
    /// Fraction (0.0 - 1.0) of nodes on which any delay will be introduced
    pub inject_delay_node_fraction: f64,
    /// For nodes with delay, what percentage (0-100) of transaction will be delayed.
    /// (this is needed because delay that can be introduced is integer number of ms)
    /// Different node speed come from this setting, each node is selected a number
    /// between 1 and given max.
    pub inject_delay_max_transaction_percentage: u32,
    /// Fixed busy-loop delay applied to each transaction that is delayed,
    /// before it is executed.
    pub inject_delay_per_transaction_ms: u32,
}

pub struct ExecutionDelayTest {
    pub add_execution_delay: ExecutionDelayConfig,
}

#[async_trait]
impl NetworkLoadTest for ExecutionDelayTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        add_execution_delay(ctx.swarm.clone(), &self.add_execution_delay).await?;
        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        remove_execution_delay(ctx.swarm.clone()).await
    }
}

#[async_trait]
impl NetworkTest for ExecutionDelayTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

impl Test for ExecutionDelayTest {
    fn name(&self) -> &'static str {
        "ExecutionDelayWrapper"
    }
}

pub struct NetworkUnreliabilityConfig {
    pub inject_unreliability_fraction: f64,
    pub inject_max_unreliability_percentage: f32,
}

pub struct NetworkUnreliabilityTest {
    pub config: NetworkUnreliabilityConfig,
}

#[async_trait]
impl NetworkLoadTest for NetworkUnreliabilityTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        let validators = { ctx.swarm.read().await.get_validator_clients_with_names() };

        for (name, validator) in validators {
            let drop_percentage = {
                let mut rng = rand::thread_rng();
                if rng.gen_bool(self.config.inject_unreliability_fraction) {
                    rng.gen_range(
                        1_u32,
                        (self.config.inject_max_unreliability_percentage * 1000.0) as u32,
                    ) as f32
                        / 1000.0
                } else {
                    0.0
                }
            };
            info!(
                "Validator {} dropping {}% of messages",
                name, drop_percentage
            );
            validator
                .set_failpoint(
                    "consensus::send::any".to_string(),
                    format!("{}%return", drop_percentage),
                )
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "set_failpoint to add unreliability on {} failed, {:?}",
                        name,
                        e
                    )
                })?;
        }

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        let validators = { ctx.swarm.read().await.get_validator_clients_with_names() };

        for (name, validator) in validators {
            validator
                .set_failpoint("consensus::send::any".to_string(), "off".to_string())
                .await
                .map_err(|e| {
                    anyhow::anyhow!(
                        "set_failpoint to remove unreliability on {} failed, {:?}",
                        name,
                        e
                    )
                })?;
        }
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for NetworkUnreliabilityTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

impl Test for NetworkUnreliabilityTest {
    fn name(&self) -> &'static str {
        "NetworkUnreliabilityWrapper"
    }
}

#[derive(Clone)]
pub struct CpuChaosConfig {
    pub num_groups: usize,
    pub load_per_worker: u64,
}

impl Default for CpuChaosConfig {
    fn default() -> Self {
        Self {
            num_groups: 4,
            load_per_worker: 100,
        }
    }
}

#[derive(Default)]
pub struct CpuChaosTest {
    cpu_chaos_config: CpuChaosConfig,
}

impl CpuChaosTest {
    pub fn new_with_config(cpu_chaos_config: CpuChaosConfig) -> Self {
        Self { cpu_chaos_config }
    }

    /// Creates a new SwarmCpuStress to be injected via chaos. Note:
    /// CPU chaos is only done for the validators in the swarm (and
    /// not the fullnodes).
    async fn create_cpu_chaos(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<(dyn Swarm)>>>,
    ) -> SwarmCpuStress {
        let all_validators = swarm
            .read()
            .await
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();
        let cpu_chaos_config = self.cpu_chaos_config.clone();
        create_swarm_cpu_stress(all_validators, Some(cpu_chaos_config))
    }
}

impl Test for CpuChaosTest {
    fn name(&self) -> &'static str {
        "CpuChaosWrapper"
    }
}

/// Creates a SwarmCpuStress to be injected via chaos. CPU chaos
/// is added to all the given peers using the specified config.
pub fn create_swarm_cpu_stress(
    all_peers: Vec<PeerId>,
    cpu_chaos_config: Option<CpuChaosConfig>,
) -> SwarmCpuStress {
    // Determine the CPU chaos config to use
    let cpu_chaos_config = cpu_chaos_config.unwrap_or_default();

    // Chunk the peers into groups and create a GroupCpuStress for each group
    let all_peers = all_peers.iter().map(|id| vec![*id]).collect();
    let peer_chunks = chunk_peers(all_peers, cpu_chaos_config.num_groups);
    let group_cpu_stresses = peer_chunks
        .into_iter()
        .enumerate()
        .map(|(idx, chunk)| {
            // Lower bound the number of workers
            let num_workers = if cpu_chaos_config.num_groups > idx {
                (cpu_chaos_config.num_groups - idx) as u64
            } else {
                1
            };

            // Create the cpu stress for the group
            info!(
                "Creating CPU stress for group {} with {} workers",
                idx, num_workers
            );
            GroupCpuStress {
                name: format!("group-{}-cpu-stress", idx),
                target_nodes: chunk,
                num_workers,
                load_per_worker: cpu_chaos_config.load_per_worker,
            }
        })
        .collect();

    SwarmCpuStress { group_cpu_stresses }
}

#[async_trait]
impl NetworkLoadTest for CpuChaosTest {
    async fn setup<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<LoadDestination> {
        let swarm_cpu_stress = self.create_cpu_chaos(ctx.swarm.clone()).await;

        ctx.swarm
            .write()
            .await
            .inject_chaos(SwarmChaos::CpuStress(swarm_cpu_stress))
            .await?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn finish<'a>(&self, ctx: &mut NetworkContext<'a>) -> anyhow::Result<()> {
        let swarm_cpu_stress = self.create_cpu_chaos(ctx.swarm.clone()).await;

        ctx.swarm
            .write()
            .await
            .remove_chaos(SwarmChaos::CpuStress(swarm_cpu_stress))
            .await
    }
}

#[async_trait]
impl NetworkTest for CpuChaosTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
