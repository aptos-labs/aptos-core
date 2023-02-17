// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_forge::{NetworkContext, NetworkTest, Result, Test};

pub struct PerformanceBenchmarkWithFN;

impl Test for PerformanceBenchmarkWithFN {
    fn name(&self) -> &'static str {
        "performance benchmark with full nodes"
    }
}

impl NetworkLoadTest for PerformanceBenchmarkWithFN {}

impl NetworkTest for PerformanceBenchmarkWithFN {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
