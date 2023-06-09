// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{multi_region_network_test::chunk_validators, LoadDestination, NetworkLoadTest};
use aptos_forge::{
    GroupCpuStress, NetworkContext, NetworkTest, Swarm, SwarmChaos, SwarmCpuStress, SwarmExt, Test,
};
use aptos_logger::info;
use aptos_types::PeerId;
use rand::Rng;
use tokio::runtime::Runtime;

fn add_execution_delay(swarm: &mut dyn Swarm, config: &ExecutionDelayConfig) -> anyhow::Result<()> {
    let runtime = Runtime::new().unwrap();
    let validators = swarm.get_validator_clients_with_names();

    runtime.block_on(async {
        let mut rng = rand::thread_rng();
        for (name, validator) in validators {
            let sleep_percentage = if rng.gen_bool(config.inject_delay_node_fraction) {
                rng.gen_range(1_u32, config.inject_delay_max_transaction_percentage)
            } else {
                0
            };
            info!(
                "Validator {} adding {}% of transactions with {}ms execution delay",
                name, sleep_percentage, config.inject_delay_per_transaction_ms
            );
            validator
                .set_failpoint(
                    "aptos_vm::execution::user_transaction".to_string(),
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
    })
}

fn remove_execution_delay(swarm: &mut dyn Swarm) -> anyhow::Result<()> {
    let runtime = Runtime::new().unwrap();
    let validators = swarm.get_validator_clients_with_names();

    runtime.block_on(async {
        for (name, validator) in validators {
            validator
                .set_failpoint(
                    "aptos_vm::execution::block_metadata".to_string(),
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
    })
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

impl NetworkLoadTest for ExecutionDelayTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        add_execution_delay(ctx.swarm(), &self.add_execution_delay)?;
        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        remove_execution_delay(swarm)
    }
}

impl NetworkTest for ExecutionDelayTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
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

impl NetworkLoadTest for NetworkUnreliabilityTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        let swarm = ctx.swarm();
        let runtime = Runtime::new().unwrap();
        let validators = swarm.get_validator_clients_with_names();

        runtime.block_on(async {
            let mut rng = rand::thread_rng();
            for (name, validator) in validators {
                let drop_percentage = if rng.gen_bool(self.config.inject_unreliability_fraction) {
                    rng.gen_range(
                        1_u32,
                        (self.config.inject_max_unreliability_percentage * 1000.0) as u32,
                    ) as f32
                        / 1000.0
                } else {
                    0.0
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
            Ok::<(), anyhow::Error>(())
        })?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        let runtime = Runtime::new().unwrap();
        let validators = swarm.get_validator_clients_with_names();

        runtime.block_on(async {
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
        })
    }
}

impl NetworkTest for NetworkUnreliabilityTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
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

pub struct CpuChaosTest {
    pub override_config: Option<CpuChaosConfig>,
}

impl CpuChaosTest {
    fn create_cpu_chaos(&self, swarm: &mut dyn Swarm) -> SwarmCpuStress {
        let all_validators = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

        let config = self.override_config.as_ref().cloned().unwrap_or_default();

        create_cpu_stress_template(all_validators, &config)
    }
}

impl Test for CpuChaosTest {
    fn name(&self) -> &'static str {
        "CpuChaosWrapper"
    }
}

fn create_cpu_stress_template(
    all_validators: Vec<PeerId>,
    config: &CpuChaosConfig,
) -> SwarmCpuStress {
    let validator_chunks = chunk_validators(all_validators, config.num_groups);

    let group_cpu_stresses = validator_chunks
        .into_iter()
        .enumerate()
        .map(|(idx, chunk)| GroupCpuStress {
            name: format!("group-{}-cpu-stress", idx),
            target_nodes: chunk,
            num_workers: (config.num_groups - idx) as u64,
            load_per_worker: config.load_per_worker,
        })
        .collect();
    SwarmCpuStress { group_cpu_stresses }
}

impl NetworkLoadTest for CpuChaosTest {
    fn setup(&self, ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        let swarm_cpu_stress = self.create_cpu_chaos(ctx.swarm());

        ctx.swarm()
            .inject_chaos(SwarmChaos::CpuStress(swarm_cpu_stress))?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> anyhow::Result<()> {
        let swarm_cpu_stress = self.create_cpu_chaos(swarm);

        swarm.remove_chaos(SwarmChaos::CpuStress(swarm_cpu_stress))
    }
}

impl NetworkTest for CpuChaosTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
