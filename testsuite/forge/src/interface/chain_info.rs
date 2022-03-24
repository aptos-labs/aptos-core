// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosPublicInfo, NFTPublicInfo};
use aptos_rest_client::Client as RestClient;
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, LocalAccount},
};
use reqwest::Url;

#[derive(Debug)]
pub struct ChainInfo<'t> {
    pub root_account: &'t mut LocalAccount,
    pub treasury_compliance_account: &'t mut LocalAccount,
    pub designated_dealer_account: &'t mut LocalAccount,
    pub rest_api_url: String,
    pub chain_id: ChainId,
}

impl<'t> ChainInfo<'t> {
    pub fn new(
        root_account: &'t mut LocalAccount,
        treasury_compliance_account: &'t mut LocalAccount,
        designated_dealer_account: &'t mut LocalAccount,
        rest_api_url: String,
        chain_id: ChainId,
    ) -> Self {
        Self {
            root_account,
            treasury_compliance_account,
            designated_dealer_account,
            rest_api_url,
            chain_id,
        }
    }

    pub fn designated_dealer_account(&mut self) -> &mut LocalAccount {
        self.designated_dealer_account
    }

    pub fn root_account(&mut self) -> &mut LocalAccount {
        self.root_account
    }

    pub fn treasury_compliance_account(&mut self) -> &mut LocalAccount {
        self.treasury_compliance_account
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

    pub fn into_nft_public_info(self) -> NFTPublicInfo<'t> {
        NFTPublicInfo::new(self.chain_id, self.rest_api_url.clone(), self.root_account)
    }

    pub fn into_aptos_public_info(self) -> AptosPublicInfo<'t> {
        AptosPublicInfo::new(self.chain_id, self.rest_api_url.clone(), self.root_account)
    }
}
