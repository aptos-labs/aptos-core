// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use super::Test;
use crate::{CoreContext, Result, Swarm, TestReport};
use transaction_emitter::EmitJobRequest;

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
}

impl<'t> NetworkContext<'t> {
    pub fn new(
        core: CoreContext,
        swarm: &'t mut dyn Swarm,
        report: &'t mut TestReport,
        global_job: EmitJobRequest,
    ) -> Self {
        Self {
            core,
            swarm,
            report,
            global_job,
        }
    }

    pub fn swarm(&mut self) -> &mut dyn Swarm {
        self.swarm
    }

    pub fn core(&mut self) -> &mut CoreContext {
        &mut self.core
    }
}
