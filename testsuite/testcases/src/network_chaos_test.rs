// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use crate::generate_traffic;
use forge::{
    NetworkContext, NetworkTest, Result, SwarmChaos, SwarmNetworkBandwidth, SwarmNetworkDelay,
    SwarmNetworkPartition, Test,
};

pub struct NetworkChaosTest;

// Delay
pub const LATENCY_MS: u64 = 80;
pub const JITTER_MS: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

// Bandwidth
// Indicates the rate of bandwidth limit
pub const RATE_MBPS: u64 = 100;
// Indicates the number of bytes waiting in queue
pub const LIMIT_BYTES: u64 = 20971520;
// Indicates the maximum number of bytes that can be sent instantaneously
pub const BUFFER_BYTES: u64 = 10000;

// Partition
pub const PARTITION_PERCENTAGE: u64 = 30;

impl Test for NetworkChaosTest {
    fn name(&self) -> &'static str {
        "network::inject-chaos"
    }
}

impl NetworkTest for NetworkChaosTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        // test each phase with 30s txn emission
        let duration = Duration::from_secs(30);
        let delay = SwarmChaos::Delay(SwarmNetworkDelay {
            latency_ms: LATENCY_MS,
            jitter_ms: JITTER_MS,
            correlation_percentage: CORRELATION_PERCENTAGE,
        });
        let bandwidth = SwarmChaos::Bandwidth(SwarmNetworkBandwidth {
            rate: RATE_MBPS,
            limit: LIMIT_BYTES,
            buffer: BUFFER_BYTES,
        });
        let partition = SwarmChaos::Partition(SwarmNetworkPartition {
            partition_percentage: PARTITION_PERCENTAGE,
        });

        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // INJECT DELAY AND EMIT TXNS
        ctx.swarm().inject_chaos(delay.clone())?;
        let msg = format!(
            "Injected {}ms +- {}ms with {}% correlation latency to namespace",
            LATENCY_MS, JITTER_MS, CORRELATION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(format!("{}:delay", self.name()), txn_stat, duration);
        ctx.swarm().remove_chaos(delay)?;

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
            .report_txn_stats(format!("{}:bandwidth", self.name()), txn_stat, duration);
        ctx.swarm().remove_chaos(bandwidth)?;

        // INJECT PARTITION AND EMIT TXNS
        ctx.swarm().inject_chaos(partition.clone())?;
        let msg = format!(
            "Partitioned {}% validators in namespace",
            PARTITION_PERCENTAGE
        );
        println!("{}", msg);
        ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(format!("{}:partition", self.name()), txn_stat, duration);
        ctx.swarm().remove_chaos(partition)?;

        Ok(())
    }
}
