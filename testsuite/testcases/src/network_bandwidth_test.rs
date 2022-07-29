// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, Result, SwarmChaos, SwarmNetworkBandwidth, Test};

pub struct NetworkBandwidthTest;

// Bandwidth
// Indicates the rate of bandwidth limit
pub const RATE_MBPS: u64 = 100;
// Indicates the number of bytes waiting in queue
pub const LIMIT_BYTES: u64 = 20971520;
// Indicates the maximum number of bytes that can be sent instantaneously
pub const BUFFER_BYTES: u64 = 10000;

impl Test for NetworkBandwidthTest {
    fn name(&self) -> &'static str {
        "network::bandwidth-test"
    }
}

impl NetworkTest for NetworkBandwidthTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_job.duration;
        let bandwidth = SwarmChaos::Bandwidth(SwarmNetworkBandwidth {
            rate: RATE_MBPS,
            limit: LIMIT_BYTES,
            buffer: BUFFER_BYTES,
        });

        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // INJECT BANDWIDTH LIMIT AND EMIT TXNS
        ctx.swarm().inject_chaos(bandwidth.clone())?;
        let msg = format!(
            "Limited bandwidth to {}mbps with limit {} and buffer {} to namespace",
            RATE_MBPS, LIMIT_BYTES, BUFFER_BYTES
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(format!("{}:bandwidth", self.name()), &txn_stat, duration);
        ctx.swarm().remove_chaos(bandwidth)?;

        // ensure we meet the success criteria
        ctx.success_criteria()
            .check_for_success(&txn_stat, &duration)?;

        Ok(())
    }
}
