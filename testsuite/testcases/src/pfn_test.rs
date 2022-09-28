// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use forge::{NetworkContext, NetworkTest, Test};

pub struct PfnTest;

impl Test for PfnTest {
    fn name(&self) -> &'static str {
        "public fullnode test"
    }
}

impl NetworkLoadTest for PfnTest {
    fn setup(&self, _ctx: &mut NetworkContext) -> anyhow::Result<LoadDestination> {
        Ok(LoadDestination::AllFullnodes)
    }
}
impl NetworkTest for PfnTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        todo!()
    }
}
