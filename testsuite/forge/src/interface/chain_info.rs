// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::VelorPublicInfo;
use anyhow::Result;
use velor_rest_client::Client as RestClient;
use velor_sdk::{
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, LocalAccount},
};
use reqwest::Url;
use std::sync::Arc;

#[derive(Debug)]
pub struct ChainInfo {
    pub root_account: Arc<LocalAccount>,
    pub rest_api_url: String,
    pub inspection_service_url: String,
    pub chain_id: ChainId,
}

impl ChainInfo {
    pub fn new(
        root_account: Arc<LocalAccount>,
        rest_api_url: String,
        inspection_service_url: String,
        chain_id: ChainId,
    ) -> Self {
        Self {
            root_account,
            rest_api_url,
            inspection_service_url,
            chain_id,
        }
    }

    pub fn root_account(&self) -> Arc<LocalAccount> {
        self.root_account.clone()
    }

    pub async fn resync_root_account_seq_num(&mut self, client: &RestClient) -> Result<()> {
        let root_address = { self.root_account.address() };
        let account = client.get_account(root_address).await?.into_inner();
        self.root_account
            .set_sequence_number(account.sequence_number);
        Ok(())
    }

    pub fn rest_api(&self) -> &str {
        &self.rest_api_url
    }

    pub fn rest_client(&self) -> RestClient {
        RestClient::new(Url::parse(self.rest_api()).unwrap())
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn transaction_factory(&self) -> TransactionFactory {
        TransactionFactory::new(self.chain_id())
    }

    pub fn into_velor_public_info(self) -> VelorPublicInfo {
        VelorPublicInfo::new(
            self.chain_id,
            self.inspection_service_url.clone(),
            self.rest_api_url.clone(),
            self.root_account.clone(),
        )
    }
}
