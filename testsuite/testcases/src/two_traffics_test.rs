// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    create_buffered_load, LoadDestination, NetworkLoadTest, COOLDOWN_DURATION_FRACTION,
    WARMUP_DURATION_FRACTION,
};
use aptos_forge::{
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    EmitJobRequest, NetworkContextSynchronizer, NetworkTest, Result, Swarm, Test, TestReport,
};
use async_trait::async_trait;
use log::info;
use std::{sync::Arc, time::Duration};

pub struct TwoTrafficsTest {
    pub inner_traffic: EmitJobRequest,
    pub inner_success_criteria: SuccessCriteria,
}

impl Test for TwoTrafficsTest {
    fn name(&self) -> &'static str {
        "two traffics test"
    }
}

#[async_trait]
impl NetworkLoadTest for TwoTrafficsTest {
    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        info!(
            "Running TwoTrafficsTest test for duration {}s",
            duration.as_secs_f32()
        );
        let nodes_to_send_load_to = LoadDestination::FullnodesOtherwiseValidators
            .get_destination_nodes(swarm.clone())
            .await;

        let stats_by_phase = create_buffered_load(
            swarm,
            &nodes_to_send_load_to,
            self.inner_traffic.clone(),
            duration,
            WARMUP_DURATION_FRACTION,
            COOLDOWN_DURATION_FRACTION,
            None,
            None,
        )
        .await?;

        for phase_stats in stats_by_phase.into_iter() {
            report.report_txn_stats(
                format!("{}: inner traffic", self.name()),
                &phase_stats.emitter_stats,
            );

            SuccessCriteriaChecker::check_core_for_success(
                &self.inner_success_criteria,
                report,
                &phase_stats.emitter_stats.rate(),
                None,
                Some("inner traffic".to_string()),
            )?;
        }

        Ok(())
    }
}

#[async_trait]
impl NetworkTest for TwoTrafficsTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
