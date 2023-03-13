// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    create_emitter_and_request, three_region_simulation_test::ThreeRegionSimulationTest,
    traffic_emitter_runtime, LoadDestination, NetworkLoadTest,
};
use anyhow::{bail, Ok};
use aptos_forge::{
    success_criteria::{LatencyType, SuccessCriteriaChecker},
    EmitJobMode, EmitJobRequest, NetworkContext, NetworkTest, Result, Swarm, Test,
};
use aptos_logger::info;
use rand::{rngs::OsRng, Rng, SeedableRng};
use std::time::{Duration, Instant};

pub struct TwoTrafficsTest {
    // cannot have 'static EmitJobRequest, like below, so need to have inner fields
    // pub inner_emit_job_request: EmitJobRequest,
    pub inner_tps: usize,
    pub inner_gas_price: u64,
    pub inner_init_gas_price_multiplier: u64,

    pub avg_tps: usize,
    pub latency_thresholds: &'static [(f32, LatencyType)],
}

impl Test for TwoTrafficsTest {
    fn name(&self) -> &'static str {
        "two traffics test"
    }
}

impl NetworkLoadTest for TwoTrafficsTest {
    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        info!(
            "Running TwoTrafficsTest test for duration {}s",
            duration.as_secs_f32()
        );
        let nodes_to_send_load_to = LoadDestination::AllFullnodes.get_destination_nodes(swarm);
        let rng = ::rand::rngs::StdRng::from_seed(OsRng.gen());

        let (emitter, emit_job_request) = create_emitter_and_request(
            swarm,
            EmitJobRequest::default()
                .mode(EmitJobMode::ConstTps {
                    tps: self.inner_tps,
                })
                .gas_price(self.inner_gas_price)
                .init_gas_price_multiplier(self.inner_init_gas_price_multiplier),
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
        info!(
            "End to end duration: {}s, while txn emitter lasted: {}s",
            actual_test_duration.as_secs(),
            stats.lasted.as_secs()
        );

        let rate = stats.rate();
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
}

impl NetworkTest for TwoTrafficsTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}

pub struct ThreeRegionSimulationTwoTrafficsTest {
    pub traffic_test: TwoTrafficsTest,
    pub three_region_simulation_test: ThreeRegionSimulationTest,
}

impl Test for ThreeRegionSimulationTwoTrafficsTest {
    fn name(&self) -> &'static str {
        "three region simulation two traffics test"
    }
}

impl NetworkLoadTest for ThreeRegionSimulationTwoTrafficsTest {
    fn setup(&self, ctx: &mut NetworkContext) -> Result<LoadDestination> {
        self.traffic_test.setup(ctx)?;
        self.three_region_simulation_test.setup(ctx)?;

        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        info!(
            "Running ThreeRegionSimulationTwoTrafficsTest test for duration {}s",
            duration.as_secs_f32()
        );
        self.traffic_test.test(swarm, duration)
    }

    fn finish(&self, swarm: &mut dyn Swarm) -> Result<()> {
        self.traffic_test.finish(swarm)?;
        self.three_region_simulation_test.finish(swarm)?;
        Ok(())
    }
}

impl NetworkTest for ThreeRegionSimulationTwoTrafficsTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
