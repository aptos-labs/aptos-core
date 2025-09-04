// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use velor_forge::{NetworkContextSynchronizer, NetworkTest, Result, Test};
use async_trait::async_trait;

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

impl NetworkLoadTest for PerformanceBenchmark {}

#[async_trait]
impl NetworkTest for PerformanceBenchmark {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
