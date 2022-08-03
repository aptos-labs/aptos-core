// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::success_criteria::SuccessCriteria;
use crate::{CoreContext, Result, Swarm, TestReport};
use transaction_emitter_lib::EmitJobRequest;

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
    pub global_job: EmitJobRequest,
    success_criteria: SuccessCriteria,
}

impl<'t> NetworkContext<'t> {
    pub fn new(
        core: CoreContext,
        swarm: &'t mut dyn Swarm,
        report: &'t mut TestReport,
        global_job: EmitJobRequest,
        success_criteria: SuccessCriteria,
    ) -> Self {
        Self {
            core,
            swarm,
            report,
            global_job,
            success_criteria,
        }
    }

    pub fn swarm(&mut self) -> &mut dyn Swarm {
        self.swarm
    }

    pub fn core(&mut self) -> &mut CoreContext {
        &mut self.core
    }
    pub fn success_criteria(&self) -> &SuccessCriteria {
        &self.success_criteria
    }
}
