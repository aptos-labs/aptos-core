// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos_forge::{NetworkContext, NetworkTest, Result, Test};

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

impl NetworkLoadTest for PerformanceBenchmark {}

impl NetworkTest for PerformanceBenchmark {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
