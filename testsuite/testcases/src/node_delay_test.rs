// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::generate_traffic;
use forge::{NetworkContext, NetworkTest, NodeChaos, NodeNetworkDelay, Result, Test};

pub struct NodeDelayTest;

impl Test for NodeDelayTest {
    fn name(&self) -> &'static str {
        "node::delay-test"
    }
}

// Delay
// us_east<->eu one way delay
pub const AB_LATENCY_MS: u64 = 60;
// us_west<->eu one way delay
pub const AC_LATENCY_MS: u64 = 95;
// us_west<->us_east one way delay
pub const BC_LATENCY_MS: u64 = 40;

pub const JITTER_MS: u64 = 20;
pub const CORRELATION_PERCENTAGE: u64 = 10;

fn create_node_delay(target_node: String, latency_ms: u64) -> NodeChaos {
    NodeChaos::Delay(NodeNetworkDelay {
        latency_ms,
        jitter_ms: JITTER_MS,
        correlation_percentage: CORRELATION_PERCENTAGE,
        target_node,
    })
}

impl NetworkTest for NodeDelayTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let duration = ctx.global_job.duration;

        // emit to all validator
        let all_validators = ctx
            .swarm()
            .validators()
            .map(|v| v.peer_id())
            .collect::<Vec<_>>();

        // INJECT DELAY AND EMIT TXNS
        // partition the network into three groups A,B,C
        // apply delay chaos n^2
        // apply A --> B delay
        // apply A --> C delay
        // apply B --> A delay
        let num_validators = ctx.swarm().validators().count();

        let mut region_a_validators = vec![];
        let mut region_b_validators = vec![];
        let mut region_c_validators = vec![];

        // partition validators into regions
        for (i, v) in ctx.swarm().validators_mut().enumerate() {
            if i < num_validators / 3 {
                region_a_validators.push(v);
            } else if i < num_validators / 3 * 2 {
                region_b_validators.push(v);
            } else {
                region_c_validators.push(v);
            }
        }

        for v_a in &mut region_a_validators {
            for v_b in &mut region_b_validators {
                let chaos_a_b = create_node_delay(v_b.name().to_string(), AB_LATENCY_MS);
                v_a.inject_chaos(chaos_a_b)?;
                let chaos_b_a = create_node_delay(v_a.name().to_string(), AB_LATENCY_MS);
                v_b.inject_chaos(chaos_b_a)?;
            }
            for v_c in &mut region_b_validators {
                let chaos_a_c = create_node_delay(v_c.name().to_string(), AC_LATENCY_MS);
                v_a.inject_chaos(chaos_a_c)?;
                let chaos_c_a = create_node_delay(v_a.name().to_string(), AC_LATENCY_MS);
                v_c.inject_chaos(chaos_c_a)?;
            }
        }
        for v_b in &mut region_b_validators {
            for v_c in &mut region_c_validators {
                let chaos_b_c = create_node_delay(v_c.name().to_string(), AB_LATENCY_MS);
                v_b.inject_chaos(chaos_b_c)?;
                let chaos_c_b = create_node_delay(v_b.name().to_string(), AB_LATENCY_MS);
                v_c.inject_chaos(chaos_c_b)?;
            }
        }
        // let msg = format!(
        //     "Injected {}ms +- {}ms with {}% correlation latency between all nodes",
        //     LATENCY_MS, JITTER_MS, CORRELATION_PERCENTAGE
        // );
        // println!("{}", msg);
        // ctx.report.report_text(msg);
        let txn_stat = generate_traffic(ctx, &all_validators, duration, 1, None)?;
        ctx.report
            .report_txn_stats(format!("{}:delay", self.name()), &txn_stat, duration);

        // ensure we meet the success criteria
        ctx.success_criteria()
            .check_for_success(&txn_stat, &duration)?;

        Ok(())
    }
}
