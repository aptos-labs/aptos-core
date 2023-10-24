// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::LoadDestination;
use aptos_forge::{NetworkContext, NetworkTest, SwarmExt, Test};
use aptos_logger::info;
use aptos_rest_client::Client;
use std::time::Duration;

pub struct ApiHealthCheck {
    period_ms: u64,
    timeout_ms: u64,
    threshold_secs: u64,
}

impl ApiHealthCheck {
    pub fn new(period_ms: u64, timeout_ms: u64, threshold_secs: u64) -> Self {
        Self {
            period_ms,
            timeout_ms,
            threshold_secs,
        }
    }

    async fn run_health_checks_once(&self, rest_clients: Vec<Client>) -> anyhow::Result<()> {
        for (i, rest_client) in rest_clients.iter().enumerate() {
            info!("Checking health of node {}", i);
            rest_client.health_check(self.threshold_secs).await?;
        }
        Ok(())
    }
}

impl NetworkTest for ApiHealthCheck {
    fn run(&self, ctx: &mut NetworkContext<'_>) -> anyhow::Result<()> {
        let runtime = ctx.runtime.handle().clone();
        let api_destinations = LoadDestination::AllValidators.get_destination_nodes(ctx.swarm());
        let rest_clients = ctx
            .swarm()
            .get_clients_for_peers(&api_destinations, Duration::from_millis(self.timeout_ms));
        runtime.block_on(self.run_health_checks_once(rest_clients))
    }
}

impl Test for ApiHealthCheck {
    fn name(&self) -> &'static str {
        "ApiHealthCheck"
    }
}
