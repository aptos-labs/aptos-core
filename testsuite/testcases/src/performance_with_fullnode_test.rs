// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Result, Test};

pub struct PerformanceBenchmarkWithFN;

impl Test for PerformanceBenchmarkWithFN {
    fn name(&self) -> &'static str {
        "performance benchmark with full nodes"
    }
}

impl NetworkLoadTest for PerformanceBenchmarkWithFN {
    fn setup(&self, _ctx: &mut NetworkContext) -> Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }
}

impl NetworkTest for PerformanceBenchmarkWithFN {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
