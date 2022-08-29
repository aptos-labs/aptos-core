// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use forge::{NetworkContext, NetworkTest, Result, Test};

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

impl NetworkLoadTest for PerformanceBenchmark {}

impl NetworkTest for PerformanceBenchmark {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
