// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    create_emitter_and_request, traffic_emitter_runtime, LoadDestination, NetworkLoadTest,
};
use aptos_forge::{
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJobRequest, NetworkContext, NetworkTest, Result, Swarm, Test, TestReport,
};
use aptos_logger::info;
use rand::{rngs::OsRng, Rng, SeedableRng};
use std::time::{Duration, Instant};

pub struct TwoTrafficsTest {
    pub inner_traffic: EmitJobRequest,
    pub inner_success_criteria: SuccessCriteria,
}

impl Test for TwoTrafficsTest {
    fn name(&self) -> &'static str {
        "two traffics test"
    }
}

impl NetworkLoadTest for TwoTrafficsTest {
    fn test(
        &self,
        swarm: &mut dyn Swarm,
        report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        info!(
            "Running TwoTrafficsTest test for duration {}s",
            duration.as_secs_f32()
        );
        let nodes_to_send_load_to =
            LoadDestination::FullnodesOtherwiseValidators.get_destination_nodes(swarm);
        let rng = ::rand::rngs::StdRng::from_seed(OsRng.gen());

        let (emitter, emit_job_request) = create_emitter_and_request(
            swarm,
            self.inner_traffic.clone(),
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

        report.report_txn_stats(format!("{}: inner traffic", self.name()), &stats);

        SuccessCriteriaChecker::check_core_for_success(
            &self.inner_success_criteria,
            report,
            &rate,
            Some("inner traffic".to_string()),
        )?;
        Ok(())
    }
}

impl NetworkTest for TwoTrafficsTest {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
