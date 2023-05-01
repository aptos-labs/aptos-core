// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_forge::{NetworkContext, NetworkTest, Swarm, SwarmExt, Test};
use aptos_logger::info;
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
