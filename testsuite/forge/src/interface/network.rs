// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{
    prometheus_metrics::LatencyBreakdown,
    success_criteria::{SuccessCriteria, SuccessCriteriaChecker},
    CoreContext, Result, Swarm, TestReport,
};
use aptos_transaction_emitter_lib::{EmitJobRequest, TxnStats};
use std::time::Duration;
use tokio::runtime::Runtime;

/// The testing interface which defines a test written with full control over an existing network.
/// Tests written against this interface will have access to both the Root account as well as the
/// nodes which comprise the network.
pub trait NetworkTest: Test {
    /// Executes the test against the given context.
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()>;
}

pub struct NetworkContext<'t> {
    core: CoreContext,
    pub swarm: &'t mut dyn Swarm,
    pub report: &'t mut TestReport,
    pub global_duration: Duration,
    pub emit_job: EmitJobRequest,
    pub success_criteria: SuccessCriteria,
    pub runtime: Runtime,
}

impl<'t> NetworkContext<'t> {
    pub fn new(
        core: CoreContext,
        swarm: &'t mut dyn Swarm,
        report: &'t mut TestReport,
        global_duration: Duration,
        emit_job: EmitJobRequest,
        success_criteria: SuccessCriteria,
    ) -> Self {
        Self {
            core,
            swarm,
            report,
            global_duration,
            emit_job,
            success_criteria,
            runtime: Runtime::new().unwrap(),
        }
    }

    pub fn swarm(&mut self) -> &mut dyn Swarm {
        self.swarm
    }

    pub fn core(&mut self) -> &mut CoreContext {
        &mut self.core
    }

    pub fn check_for_success(
        &mut self,
        stats: &TxnStats,
        window: Duration,
        latency_breakdown: &LatencyBreakdown,
        start_time: i64,
        end_time: i64,
        start_version: u64,
        end_version: u64,
    ) -> Result<()> {
        self.runtime
            .block_on(SuccessCriteriaChecker::check_for_success(
                &self.success_criteria,
                self.swarm,
                self.report,
                stats,
                window,
                latency_breakdown,
                start_time,
                end_time,
                start_version,
                end_version,
            ))
    }
}
