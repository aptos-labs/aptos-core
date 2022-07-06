// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::EmptyRequest,
    types::{
        AccountBalanceRequest, AccountBalanceResponse, BlockRequest, BlockResponse,
        ConstructionCombineRequest, ConstructionCombineResponse, ConstructionDeriveRequest,
        ConstructionDeriveResponse, ConstructionHashRequest, ConstructionMetadataRequest,
        ConstructionMetadataResponse, ConstructionParseRequest, ConstructionParseResponse,
        ConstructionPayloadsRequest, ConstructionPayloadsResponse, ConstructionPreprocessRequest,
        ConstructionPreprocessResponse, ConstructionSubmitRequest, ConstructionSubmitResponse,
        Error, NetworkListResponse, NetworkOptionsResponse, NetworkRequest, NetworkStatusResponse,
        TransactionIdentifierResponse,
    },
};
use anyhow::anyhow;
use aptos_rest_client::aptos_api_types::mime_types::JSON;
use reqwest::{header::CONTENT_TYPE, Client as ReqwestClient};
use serde::{de::DeserializeOwned, Serialize};
use url::Url;

/// Client for testing & interacting with a Rosetta service
pub struct RosettaClient {
    address: Url,
    inner: ReqwestClient,
}

impl RosettaClient {
    pub fn new(address: Url) -> RosettaClient {
        RosettaClient {
            address,
            inner: ReqwestClient::new(),
        }
    }

    pub async fn account_balance(
        &self,
        request: &AccountBalanceRequest,
    ) -> anyhow::Result<AccountBalanceResponse> {
        self.make_call("account/balance", request).await
    }

    pub async fn block(&self, request: &BlockRequest) -> anyhow::Result<BlockResponse> {
        self.make_call("block", request).await
    }

    pub async fn combine(
        &self,
        request: &ConstructionCombineRequest,
    ) -> anyhow::Result<ConstructionCombineResponse> {
        self.make_call("construction/combine", request).await
    }

    pub async fn derive(
        &self,
        request: &ConstructionDeriveRequest,
    ) -> anyhow::Result<ConstructionDeriveResponse> {
        self.make_call("construction/derive", request).await
    }

    pub async fn hash(
        &self,
        request: &ConstructionHashRequest,
    ) -> anyhow::Result<TransactionIdentifierResponse> {
        self.make_call("construction/hash", request).await
    }

    pub async fn metadata(
        &self,
        request: &ConstructionMetadataRequest,
    ) -> anyhow::Result<ConstructionMetadataResponse> {
        self.make_call("construction/metadata", request).await
    }

    pub async fn parse(
        &self,
        request: &ConstructionParseRequest,
    ) -> anyhow::Result<ConstructionParseResponse> {
        self.make_call("construction/parse", request).await
    }
    pub async fn payloads(
        &self,
        request: &ConstructionPayloadsRequest,
    ) -> anyhow::Result<ConstructionPayloadsResponse> {
        self.make_call("construction/payloads", request).await
    }
    pub async fn preprocess(
        &self,
        request: &ConstructionPreprocessRequest,
    ) -> anyhow::Result<ConstructionPreprocessResponse> {
        self.make_call("construction/preprocess", request).await
    }

    pub async fn submit(
        &self,
        request: &ConstructionSubmitRequest,
    ) -> anyhow::Result<ConstructionSubmitResponse> {
        self.make_call("construction/submit", request).await
    }

    pub async fn network_list(&self) -> anyhow::Result<NetworkListResponse> {
        self.make_call("network/list", &EmptyRequest).await
    }

    pub async fn network_options(
        &self,
        request: &NetworkRequest,
    ) -> anyhow::Result<NetworkOptionsResponse> {
        self.make_call("network/options", request).await
    }

    pub async fn network_status(
        &self,
        request: &NetworkRequest,
    ) -> anyhow::Result<NetworkStatusResponse> {
        self.make_call("network/status", request).await
    }

    async fn make_call<'a, I: Serialize, O: DeserializeOwned>(
        &'a self,
        path: &'static str,
        request: &'a I,
    ) -> anyhow::Result<O> {
        let response = self
            .inner
            .post(self.address.join(path)?)
            .header(CONTENT_TYPE, JSON)
            .body(serde_json::to_string(request)?)
            .send()
            .await?;

        if !response.status().is_success() {
            let error: Error = response.json().await?;
            return Err(anyhow!("Failed API with: {:?}", error));
        }

        Ok(response.json().await?)
    }
}
