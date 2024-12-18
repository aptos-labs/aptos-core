// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{sync::Arc, time::Duration};
use anyhow::{anyhow, bail, Context};

use crate::NetworkLoadTest;
use aptos_forge::{NetworkContextSynchronizer, NetworkTest, Result, Swarm, SwarmExt, Test, TestReport};
use async_trait::async_trait;

pub struct PerformanceBenchmark;

impl Test for PerformanceBenchmark {
    fn name(&self) -> &'static str {
        "performance benchmark"
    }
}

#[async_trait]
impl NetworkLoadTest for PerformanceBenchmark {
    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        let validators = { swarm.read().await.get_validator_clients_with_names() };
        // 10 vals, test 1,2,3 failures
        let num_bad_leaders = 3;
        for (name, validator)  in validators[..num_bad_leaders].iter() {
            validator
                    .set_failpoint(
                        "consensus::leader_equivocation".to_string(),
                        "return".to_string(),
                    )
                    .await
                    .map_err(|e| {
                        anyhow!(
                            "set_failpoint to set consensus leader equivocation on {} failed, {:?}",
                            name,
                            e
                        )
                    })?;
        };
        Ok(())
    }
}

#[async_trait]
impl NetworkTest for PerformanceBenchmark {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
