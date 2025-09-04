// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::{ChainInfo, CoreContext, Test};
use crate::{Result, TestReport};
use velor_rest_client::Client as RestClient;
use velor_sdk::types::LocalAccount;
use reqwest::Url;

/// The testing interface which defines a test written from the perspective of the Admin of the
/// network. This means that the test will have access to the Root account but do not control any
/// of the validators or full nodes running on the network.
pub trait AdminTest: Test {
    /// Executes the test against the given context.
    fn run(&self, ctx: &mut AdminContext<'_>) -> Result<()>;
}

#[derive(Debug)]
pub struct AdminContext<'t> {
    core: CoreContext,

    chain_info: ChainInfo,
    pub report: &'t mut TestReport,
}

impl<'t> AdminContext<'t> {
    pub fn new(core: CoreContext, chain_info: ChainInfo, report: &'t mut TestReport) -> Self {
        Self {
            core,
            chain_info,
            report,
        }
    }

    pub fn core(&self) -> &CoreContext {
        &self.core
    }

    pub fn rng(&mut self) -> &mut ::rand::rngs::StdRng {
        self.core.rng()
    }

    pub fn rest_client(&self) -> RestClient {
        RestClient::new(Url::parse(self.chain_info.rest_api()).unwrap())
    }

    pub fn chain_info(&mut self) -> &mut ChainInfo {
        &mut self.chain_info
    }

    pub fn random_account(&mut self) -> LocalAccount {
        LocalAccount::generate(self.core.rng())
    }
}
