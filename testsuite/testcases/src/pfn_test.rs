// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use aptos_sdk::move_types::account_address::AccountAddress;
use forge::{NetworkContext, NetworkTest, NodeExt, Test};
use std::time::{Duration, Instant};
use tokio::runtime::Runtime;

pub struct PfnTest;

impl Test for PfnTest {
    fn name(&self) -> &'static str {
        "pfn"
    }
}

impl NetworkLoadTest for PfnTest {
    fn setup(&self, _ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }
}

impl NetworkTest for PfnTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
