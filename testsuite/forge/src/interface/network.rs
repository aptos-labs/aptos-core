// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use super::Test;
use crate::success_criteria::SuccessCriteria;
use crate::{CoreContext, Result, Swarm, TestReport};
use tokio::runtime::Runtime;
use transaction_emitter_lib::{EmitJobRequest, TxnStats};

/// The testing interface which defines a test written with full control over an existing network.
/// Tests written against this interface will have access to both the Root account as well as the
/// nodes which comprise the network.
pub trait NetworkTest: Test {
    /// Executes the test against the given context.
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()>;
}

pub struct NetworkContext<'t> {
    core: CoreContext,
    swarm: &'t mut dyn Swarm,
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
        window: &Duration,
        start_time: i64,
        end_time: i64,
        start_version: u64,
        end_version: u64,
    ) -> Result<()> {
        self.runtime
            .block_on(self.success_criteria.check_for_success(
                stats,
                window,
                self.swarm,
                start_time,
                end_time,
                start_version,
                end_version,
            ))
    }
}
