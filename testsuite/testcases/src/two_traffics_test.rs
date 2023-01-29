// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    create_emitter_and_request,
    three_region_simulation_test::{
        add_execution_delay, create_bandwidth_limit, create_three_region_swarm_network_delay,
        remove_execution_delay, ExecutionDelayConfig,
    },
    traffic_emitter_runtime, LoadDestination, NetworkLoadTest,
};
use anyhow::bail;
use aptos_forge::{
    success_criteria::{LatencyType, SuccessCriteriaChecker},
    EmitJobMode, EmitJobRequest, NetworkContext, NetworkTest, Result, Swarm, SwarmChaos, Test,
};
use aptos_logger::info;
use rand::{rngs::OsRng, Rng, SeedableRng};
use std::time::{Duration, Instant};

pub struct TwoTrafficsTest {
    // cannot have 'static EmitJobRequest, like below, so need to have inner fields
    // pub inner_emit_job_request: EmitJobRequest,
    pub inner_tps: usize,
    pub inner_gas_price: u64,

    pub avg_tps: usize,
    pub latency_thresholds: &'static [(f32, LatencyType)],

    pub add_execution_delay: Option<ExecutionDelayConfig>,
}

impl Test for TwoTrafficsTest {
    fn name(&self) -> &'static str {
        "two traffics test"
    }
}

impl NetworkLoadTest for TwoTrafficsTest {
    fn setup(&self, ctx: &mut NetworkContext) -> Result<LoadDestination> {
        // inject network delay
        let delay = create_three_region_swarm_network_delay(ctx.swarm());
        let chaos = SwarmChaos::Delay(delay);
        ctx.swarm().inject_chaos(chaos)?;

        // inject bandwidth limit
        let bandwidth = create_bandwidth_limit();
        let chaos = SwarmChaos::Bandwidth(bandwidth);
        ctx.swarm().inject_chaos(chaos)?;

        if let Some(config) = &self.add_execution_delay {
            add_execution_delay(ctx.swarm(), config)?;
        }

        Ok(LoadDestination::AllNodes)
    }

    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        info!(
            "Running TwoTrafficsTest test for duration {}s",
            duration.as_secs_f32()
        );
        let nodes_to_send_load_to = LoadDestination::AllFullnodes.get_destination_nodes(swarm);
        let rng = ::rand::rngs::StdRng::from_seed(OsRng.gen());

        let (mut emitter, emit_job_request) = create_emitter_and_request(
            swarm,
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps {
                    tps: self.inner_tps,
                })
                .gas_price(self.inner_gas_price),
            &nodes_to_send_load_to,
            rng,
        )?;

        let rt = traffic_emitter_runtime()?;

        let test_start = Instant::now();

        let stats = rt.block_on(emitter.emit_txn_for(
            swarm.chain_info().root_account,
            emit_job_request,
            duration,
        ))?;

        let actual_test_duration = test_start.elapsed();

        let rate = stats.rate(actual_test_duration);
        info!("Inner traffic: {:?}", rate);

        let avg_tps = rate.committed;
        if avg_tps < self.avg_tps as u64 {
            bail!(
                "TPS requirement for inner traffic failed. Average TPS {}, minimum TPS requirement {}. Full inner stats: {:?}",
                avg_tps,
                self.avg_tps,
                rate,
            )
        }

        SuccessCriteriaChecker::check_latency(
            &self
                .latency_thresholds
                .iter()
                .map(|(s, t)| (Duration::from_secs_f32(*s), t.clone()))
                .collect::<Vec<_>>(),
            &rate,
        )?;

        Ok(())
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> Result<()> {
        if self.add_execution_delay.is_some() {
            remove_execution_delay(swarm)?;
        }

        swarm.remove_all_chaos()
    }
}

impl NetworkTest for TwoTrafficsTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
