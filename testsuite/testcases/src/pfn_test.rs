// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use forge::{NetworkContext, NetworkTest, Test};

pub struct PfnTest;

impl Test for PfnTest {
    fn name(&self) -> &'static str {
        todo!()
    }
}

impl NetworkLoadTest for PfnTest {}
impl NetworkTest for PfnTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> anyhow::Result<()> {
        todo!()
    }
}
