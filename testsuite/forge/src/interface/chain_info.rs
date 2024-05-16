// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::sync::{Arc, Mutex};
use crate::AptosPublicInfo;
use anyhow::Result;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, LocalAccount},
};
use reqwest::Url;

#[derive(Debug)]
pub struct ChainInfo {
    pub root_account: Arc<Mutex<LocalAccount>>,
    pub rest_api_url: String,
    pub inspection_service_url: String,
    pub chain_id: ChainId,
}

impl ChainInfo {
    pub fn new(
        root_account: Arc<Mutex<LocalAccount>>,
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

    pub fn root_account(&mut self) -> Arc<std::sync::Mutex<LocalAccount>> {
        self.root_account.clone()
    }

    pub async fn resync_root_account_seq_num(&mut self, client: &RestClient) -> Result<()> {
        let account = client
            .get_account(self.root_account.lock().unwrap().address())
            .await?
            .into_inner();
        self.root_account.lock().unwrap()
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

    pub fn into_aptos_public_info(self) -> AptosPublicInfo {
        AptosPublicInfo::new(
            self.chain_id,
            self.inspection_service_url.clone(),
            self.rest_api_url.clone(),
            self.root_account.clone(),
        )
    }
}
