// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
pub use diem_api_types::{MoveModuleBytecode, PendingTransaction, Transaction};
use diem_client::{Response, State};
use diem_crypto::HashValue;
use diem_types::{account_address::AccountAddress, transaction::SignedTransaction};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use reqwest::{header::CONTENT_TYPE, Client as ReqwestClient};
use serde::Deserialize;
use std::time::Duration;
use url::Url;

pub mod types;
pub use diem_api_types;
pub use types::{DiemAccount, Resource, RestError};

macro_rules! cfg_dpn {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "dpn")]
            #[cfg_attr(docsrs, doc(cfg(feature = "dpn")))]
            $item
        )*
    }
}

cfg_dpn! {
    pub mod dpn;
}

const BCS_CONTENT_TYPE: &str = "application/x.diem.signed_transaction+bcs";
const USER_AGENT: &str = concat!("diem-client-sdk-rust / ", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Debug)]
pub struct Client {
    inner: ReqwestClient,
    base_url: Url,
}

impl Client {
    pub fn new(base_url: Url) -> Self {
        let inner = ReqwestClient::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .build()
            .unwrap();

        Self { inner, base_url }
    }

    cfg_dpn! {
        pub async fn get_account_balances(
            &self,
            address: AccountAddress,
        ) -> Result<Response<Vec<dpn::AccountBalance>>> {
            let resp = self
                .get_account_resources_by_type(
                    address,
                    dpn::CORE_CODE_ADDRESS,
                    &dpn::BalanceResource::module_identifier(),
                    &dpn::BalanceResource::struct_identifier(),
                )
                .await?;
            resp.and_then(|resources| {
                resources
                    .into_iter()
                    .map(|res| {
                        let currency_tag = res.resource_type.type_params.get(0);
                        if let Some(TypeTag::Struct(currency)) = currency_tag {
                            Ok(dpn::AccountBalance {
                                currency: currency.clone(),
                                amount: serde_json::from_value::<dpn::Balance>(res.data)?
                                    .coin
                                    .value
                                    .0,
                            })
                        } else {
                            Err(anyhow!("invalid account balance resource: {:?}", &res))
                        }
                    })
                    .collect::<Result<Vec<dpn::AccountBalance>>>()
            })
        }
    }

    pub async fn get_ledger_information(&self) -> Result<Response<State>> {
        #[derive(Deserialize)]
        struct Response {
            chain_id: u8,
            #[serde(deserialize_with = "types::deserialize_from_string")]
            ledger_version: u64,
            #[serde(deserialize_with = "types::deserialize_from_string")]
            ledger_timestamp: u64,
        }

        let response = self.inner.get(self.base_url.clone()).send().await?;

        let response = self.json::<Response>(response).await?.map(|r| State {
            chain_id: r.chain_id,
            version: r.ledger_version,
            timestamp_usecs: r.ledger_timestamp,
        });

        Ok(response)
    }

    pub async fn submit(&self, txn: &SignedTransaction) -> Result<Response<PendingTransaction>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.base_url.join("transactions")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .body(txn_payload)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn wait_for_transaction(
        &self,
        pending_transaction: &PendingTransaction,
    ) -> Result<Response<Transaction>> {
        const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
        const DEFAULT_DELAY: Duration = Duration::from_millis(500);

        let start = std::time::Instant::now();
        let hash = pending_transaction.hash.into();
        while start.elapsed() < DEFAULT_TIMEOUT {
            let (transaction, state) = self.get_transaction(hash).await?.into_parts();
            match transaction {
                Transaction::PendingTransaction(_) => {}
                Transaction::UserTransaction(_)
                | Transaction::GenesisTransaction(_)
                | Transaction::BlockMetadataTransaction(_) => {
                    return Ok(Response::new(transaction, state))
                }
            }

            if *pending_transaction
                .request
                .expiration_timestamp_secs
                .inner()
                <= state.timestamp_usecs / 1_000_000
            {
                return Err(anyhow!("transaction expired"));
            }

            tokio::time::sleep(DEFAULT_DELAY).await;
        }

        Err(anyhow!("timeout"))
    }

    pub async fn get_transactions(
        &self,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Response<Vec<Transaction>>> {
        let url = self.base_url.join("transactions")?;

        let mut request = self.inner.get(url);
        if let Some(start) = start {
            request = request.query(&[("start", start)])
        }

        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)])
        }

        let response = request.send().await?;

        self.json(response).await
    }

    pub async fn get_transaction(&self, hash: HashValue) -> Result<Response<Transaction>> {
        let url = self
            .base_url
            .join(&format!("transactions/{}", hash.to_hex_literal()))?;

        let response = self.inner.get(url).send().await?;

        self.json(response).await
    }

    pub async fn get_account_transactions(
        &self,
        address: AccountAddress,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> Result<Response<Vec<Transaction>>> {
        let url = self
            .base_url
            .join(&format!("accounts/{}/transactions", address))?;

        let mut request = self.inner.get(url);
        if let Some(start) = start {
            request = request.query(&[("start", start)])
        }

        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)])
        }

        let response = request.send().await?;

        self.json(response).await
    }

    pub async fn get_account_resources(
        &self,
        address: AccountAddress,
    ) -> Result<Response<Vec<Resource>>> {
        let url = self
            .base_url
            .join(&format!("accounts/{}/resources", address))?;

        let response = self.inner.get(url).send().await?;

        self.json(response).await
    }

    pub async fn get_account_resources_by_type(
        &self,
        address: AccountAddress,
        module_address: AccountAddress,
        module_id: &Identifier,
        struct_name: &Identifier,
    ) -> Result<Response<Vec<Resource>>> {
        self.get_account_resources(address).await.map(|resp| {
            resp.map(|resources| {
                resources
                    .into_iter()
                    .filter(|res| {
                        res.resource_type.address == module_address
                            && (&res.resource_type.module) == module_id
                            && (&res.resource_type.name) == struct_name
                    })
                    .collect()
            })
        })
    }

    pub async fn get_account_resource(
        &self,
        address: AccountAddress,
        resource_type: &StructTag,
    ) -> Result<Response<Option<serde_json::Value>>> {
        self.get_account_resources(address).await.map(|response| {
            response.map(|resources| {
                resources
                    .into_iter()
                    .find(|resource| &resource.resource_type == resource_type)
                    .map(|resource| resource.data)
            })
        })
    }

    pub async fn get_account_modules(
        &self,
        address: AccountAddress,
    ) -> Result<Response<Vec<MoveModuleBytecode>>> {
        let url = self
            .base_url
            .join(&format!("accounts/{}/modules", address))?;

        let response = self.inner.get(url).send().await?;

        self.json(response).await
    }

    pub async fn get_account(&self, address: AccountAddress) -> Result<Response<DiemAccount>> {
        let url = self.base_url.join(&format!("accounts/{}", address))?;
        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    async fn check_response(
        &self,
        response: reqwest::Response,
    ) -> Result<(reqwest::Response, State)> {
        if !response.status().is_success() {
            let error_response = response.json::<RestError>().await?;
            return Err(anyhow::anyhow!("Request failed: {:?}", error_response));
        }

        let state = State::from_headers(response.headers())?;

        Ok((response, state))
    }

    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> Result<Response<T>> {
        let (response, state) = self.check_response(response).await?;
        let json = response.json().await?;
        Ok(Response::new(json, state))
    }

    pub async fn health_check(&self, seconds: u64) -> Result<()> {
        let url = self.base_url.join("-/healthy")?;
        let response = self
            .inner
            .get(url)
            .query(&[("duration_secs", seconds)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("health check failed",));
        }

        Ok(())
    }
}
