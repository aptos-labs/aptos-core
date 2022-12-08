// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use crate::{
    create_emitter_and_request, traffic_emitter_runtime, LoadDestination, NetworkLoadTest,
};
use anyhow::bail;
use aptos_logger::info;
use forge::{
    success_criteria::{LatencyType, SuccessCriteriaChecker},
    EmitJobMode, EmitJobRequest, NetworkContext, NetworkTest, Result, Swarm, Test,
};
use rand::{rngs::OsRng, Rng, SeedableRng};

pub struct TwoTrafficsTest {
    // cannot have 'static EmitJobRequest, like below, so need to have inner fields
    // pub inner_emit_job_request: EmitJobRequest,
    pub inner_tps: usize,
    pub inner_gas_price: u64,

    pub avg_tps: usize,
    pub latency_thresholds: &'static [(f32, LatencyType)],
}

impl Test for TwoTrafficsTest {
    fn name(&self) -> &'static str {
        "two traffics test"
    }
}

impl NetworkLoadTest for TwoTrafficsTest {
    fn setup(&self, _ctx: &mut NetworkContext) -> Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
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
}

impl NetworkTest for TwoTrafficsTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
