// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_logger::info;
use aptos_sdk::move_types::account_address::AccountAddress;
use forge::{FullNode, NetworkContext, NetworkTest, NodeExt, Test};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub struct PfnTest;

impl Test for PfnTest {
    fn name(&self) -> &'static str {
        "PFN"
    }
}

impl NetworkLoadTest for PfnTest {
    fn setup(&self, _ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }
}

impl NetworkTest for PfnTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        let runtime = Runtime::new().unwrap();
        let version = ctx.swarm().versions().max().unwrap();
        let fullnode_id = ctx.swarm().full_nodes().next().unwrap().peer_id();
        let pfn1 = ctx
            .swarm()
            .add_full_node(&version, ctx.swarm().generate_full_node_config(fullnode_id))
            .unwrap();
        let pfn2 = ctx
            .swarm()
            .add_full_node(&version, ctx.swarm().generate_full_node_config(pfn1))
            .unwrap();
        let pfn3 = ctx
            .swarm()
            .add_full_node(&version, ctx.swarm().generate_full_node_config(pfn2))
            .unwrap();
        let pfn4 = ctx
            .swarm()
            .add_full_node(&version, ctx.swarm().generate_full_node_config(pfn3))
            .unwrap();
        let pfn5 = ctx
            .swarm()
            .add_full_node(&version, ctx.swarm().generate_full_node_config(pfn4))
            .unwrap();
        info!("{pfn5}");
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
