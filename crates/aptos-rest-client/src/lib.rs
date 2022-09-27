// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

extern crate core;

pub mod aptos;
pub mod error;
pub mod faucet;

pub use faucet::FaucetClient;
pub mod response;
pub use response::Response;
pub mod state;
pub mod types;

pub use aptos_api_types::{
    self, IndexResponse, MoveModuleBytecode, PendingTransaction, Transaction,
};
pub use state::State;
pub use types::{deserialize_from_prefixed_hex_string, Account, Resource};

use crate::aptos::{AptosVersion, Balance};
use crate::error::RestError;
use anyhow::{anyhow, Result};
use aptos_api_types::{
    deserialize_from_string,
    mime_types::{BCS, BCS_SIGNED_TRANSACTION as BCS_CONTENT_TYPE},
    AptosError, BcsBlock, Block, Bytecode, ExplainVMStatus, GasEstimation, HexEncodedBytes,
    MoveModuleId, TransactionData, TransactionOnChainData, TransactionsBatchSubmissionResult,
    UserTransaction, VersionedEvent,
};
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, CoinStoreResource, NewBlockEvent, CORE_CODE_ADDRESS},
    contract_event::EventWithVersion,
    transaction::{ExecutionStatus, SignedTransaction},
};
use futures::executor::block_on;
use move_deps::move_binary_format::CompiledModule;
use move_deps::move_core_types::language_storage::{ModuleId, StructTag};
use reqwest::header::ACCEPT;
use reqwest::{header::CONTENT_TYPE, Client as ReqwestClient, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::future::Future;
use std::rc::Rc;
use std::time::Duration;
use tokio::time::Instant;
use url::Url;

pub const USER_AGENT: &str = concat!("aptos-client-sdk-rust / ", env!("CARGO_PKG_VERSION"));
pub const DEFAULT_VERSION_PATH_BASE: &str = "v1/";
const DEFAULT_MAX_WAIT_MS: u64 = 60000;
const DEFAULT_INTERVAL_MS: u64 = 1000;
static DEFAULT_MAX_WAIT_DURATION: Duration = Duration::from_millis(DEFAULT_MAX_WAIT_MS);
static DEFAULT_INTERVAL_DURATION: Duration = Duration::from_millis(DEFAULT_INTERVAL_MS);

type AptosResult<T> = Result<T, RestError>;

#[derive(Clone, Debug)]
pub struct Client {
    inner: ReqwestClient,
    base_url: Url,
    version_path_base: String,
}

impl Client {
    pub fn new_with_timeout(base_url: Url, timeout: Duration) -> Self {
        let inner = ReqwestClient::builder()
            .timeout(timeout)
            .user_agent(USER_AGENT)
            .cookie_store(true)
            .build()
            .unwrap();

        // If the user provided no version in the path, use the default. If the
        // provided version has no trailing slash, add it, otherwise url.join
        // will ignore the version path base.
        let version_path_base = match base_url.path() {
            "/" => DEFAULT_VERSION_PATH_BASE.to_string(),
            path => {
                if !path.ends_with('/') {
                    format!("{}/", path)
                } else {
                    path.to_string()
                }
            }
        };

        Self {
            inner,
            base_url,
            version_path_base,
        }
    }

    pub fn new(base_url: Url) -> Self {
        Self::new_with_timeout(base_url, Duration::from_secs(10))
    }

    pub fn path_prefix_string(&self) -> String {
        self.base_url
            .join(&self.version_path_base)
            .map(|path| path.to_string())
            .unwrap_or_else(|_| "<bad_base_url>".to_string())
    }

    /// Set a different version path base, e.g. "v1/" See
    /// DEFAULT_VERSION_PATH_BASE for the default value.
    pub fn version_path_base(mut self, version_path_base: String) -> AptosResult<Self> {
        if !version_path_base.ends_with('/') {
            return Err(anyhow!("version_path_base must end with '/', e.g. 'v1/'").into());
        }
        self.version_path_base = version_path_base;
        Ok(self)
    }

    fn build_path(&self, path: &str) -> AptosResult<Url> {
        Ok(self.base_url.join(&self.version_path_base)?.join(path)?)
    }

    pub async fn get_aptos_version(&self) -> AptosResult<Response<AptosVersion>> {
        self.get_resource::<AptosVersion>(CORE_CODE_ADDRESS, "0x1::version::Version")
            .await
    }

    pub async fn get_block_by_height(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> AptosResult<Response<Block>> {
        self.get(self.build_path(&format!(
            "blocks/by_height/{}?with_transactions={}",
            height, with_transactions
        ))?)
        .await
    }

    pub async fn get_block_by_height_bcs(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> AptosResult<Response<BcsBlock>> {
        let url = self.build_path(&format!(
            "blocks/by_height/{}?with_transactions={}",
            height, with_transactions
        ))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    /// This will get all the transactions from the block in successive calls
    /// and will handle the successive calls
    ///
    /// Note: This could take a long time to run
    pub async fn get_full_block_by_height_bcs(
        &self,
        height: u64,
        page_size: u16,
    ) -> AptosResult<Response<BcsBlock>> {
        let (mut block, state) = self
            .get_block_by_height_bcs(height, true)
            .await?
            .into_parts();

        let mut current_version = block.first_version;

        // Set the current version to the last known transaction
        if let Some(ref txns) = block.transactions {
            if let Some(txn) = txns.last() {
                current_version = txn.version + 1;
            }
        } else {
            return Err(RestError::Unknown(anyhow!(
                "No transactions were returned in the block"
            )));
        }

        // Add in all transactions by paging through the other transactions
        while current_version <= block.last_version {
            let page_end_version =
                std::cmp::min(block.last_version, current_version + page_size as u64 - 1);

            let transactions = self
                .get_transactions_bcs(
                    Some(current_version),
                    Some((page_end_version - current_version + 1) as u16),
                )
                .await?
                .into_inner();
            if let Some(txn) = transactions.last() {
                current_version = txn.version + 1;
            };
            block.transactions.as_mut().unwrap().extend(transactions);
        }

        Ok(Response::new(block, state))
    }

    pub async fn get_block_by_version(
        &self,
        version: u64,
        with_transactions: bool,
    ) -> AptosResult<Response<Block>> {
        self.get(self.build_path(&format!(
            "blocks/by_version/{}?with_transactions={}",
            version, with_transactions
        ))?)
        .await
    }

    pub async fn get_block_by_version_bcs(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> AptosResult<Response<BcsBlock>> {
        let url = self.build_path(&format!(
            "blocks/by_version/{}?with_transactions={}",
            height, with_transactions
        ))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_balance(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<Balance>> {
        let resp = self
            .get_account_resource(address, "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>")
            .await?;
        resp.and_then(|resource| {
            if let Some(res) = resource {
                Ok(serde_json::from_value::<Balance>(res.data)?)
            } else {
                Err(anyhow!("No data returned").into())
            }
        })
    }

    pub async fn get_account_balance_bcs(
        &self,
        address: AccountAddress,
        coin_type: &str,
    ) -> AptosResult<Response<u64>> {
        let resp = self
            .get_account_resource_bcs::<CoinStoreResource>(
                address,
                &format!("0x1::coin::CoinStore<{}>", coin_type),
            )
            .await?;
        resp.and_then(|resource| Ok(resource.coin()))
    }

    pub async fn get_account_balance_at_version(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> AptosResult<Response<Balance>> {
        let resp = self
            .get_account_resource_at_version(
                address,
                "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
                version,
            )
            .await?;
        resp.and_then(|resource| {
            if let Some(res) = resource {
                Ok(serde_json::from_value::<Balance>(res.data)?)
            } else {
                Err(anyhow!("No data returned").into())
            }
        })
    }

    pub async fn get_index(&self) -> AptosResult<Response<IndexResponse>> {
        self.get(self.build_path("")?).await
    }

    pub async fn get_index_bcs(&self) -> AptosResult<Response<IndexResponse>> {
        let url = self.build_path("")?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_ledger_information(&self) -> AptosResult<Response<State>> {
        let response = self.get_index_bcs().await?.map(|r| State {
            chain_id: r.chain_id,
            epoch: r.epoch.into(),
            version: r.ledger_version.into(),
            timestamp_usecs: r.ledger_timestamp.into(),
            oldest_ledger_version: r.oldest_ledger_version.into(),
            oldest_block_height: r.oldest_block_height.into(),
            block_height: r.block_height.into(),
        });
        assert_eq!(response.inner().chain_id, response.state().chain_id);
        assert_eq!(response.inner().epoch, response.state().epoch);
        assert_eq!(response.inner().version, response.state().version);
        assert_eq!(response.inner().block_height, response.state().block_height);

        Ok(response)
    }

    pub async fn simulate(
        &self,
        txn: &SignedTransaction,
    ) -> AptosResult<Response<Vec<UserTransaction>>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path("transactions/simulate")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .body(txn_payload)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn simulate_bcs(
        &self,
        txn: &SignedTransaction,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path("transactions/simulate")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .header(ACCEPT, BCS)
            .body(txn_payload)
            .send()
            .await?;

        let response = self.check_and_parse_bcs_response(response).await?;
        Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
    }

    pub async fn simulate_bcs_with_gas_estimation(
        &self,
        txn: &SignedTransaction,
        estimate_max_gas_amount: bool,
        estimate_max_gas_unit_price: bool,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path(&format!(
            "transactions/simulate?estimate_max_gas_amount={}&estimate_gas_unit_price={}",
            estimate_max_gas_amount, estimate_max_gas_unit_price
        ))?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .header(ACCEPT, BCS)
            .body(txn_payload)
            .send()
            .await?;

        let response = self.check_and_parse_bcs_response(response).await?;
        Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
    }

    pub async fn submit(
        &self,
        txn: &SignedTransaction,
    ) -> AptosResult<Response<PendingTransaction>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path("transactions")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .body(txn_payload)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn submit_bcs(&self, txn: &SignedTransaction) -> AptosResult<Response<()>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path("transactions")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .header(ACCEPT, BCS)
            .body(txn_payload)
            .send()
            .await?;

        let response = self.check_and_parse_bcs_response(response).await?;
        Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
    }

    pub async fn submit_batch(
        &self,
        txns: &[SignedTransaction],
    ) -> AptosResult<Response<TransactionsBatchSubmissionResult>> {
        let txn_payload = bcs::to_bytes(&txns.to_vec())?;
        let url = self.build_path("transactions/batch")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .body(txn_payload)
            .send()
            .await?;
        self.json(response).await
    }
    pub async fn submit_batch_bcs(
        &self,
        txns: &[SignedTransaction],
    ) -> AptosResult<Response<TransactionsBatchSubmissionResult>> {
        let txn_payload = bcs::to_bytes(&txns.to_vec())?;
        let url = self.build_path("transactions/batch")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
            .header(ACCEPT, BCS)
            .body(txn_payload)
            .send()
            .await?;

        let response = self.check_and_parse_bcs_response(response).await?;
        Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
    }

    pub async fn submit_and_wait(
        &self,
        txn: &SignedTransaction,
    ) -> AptosResult<Response<Transaction>> {
        self.submit(txn).await?;
        self.wait_for_signed_transaction(txn).await
    }

    pub async fn submit_and_wait_bcs(
        &self,
        txn: &SignedTransaction,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        self.submit_bcs(txn).await?;
        self.wait_for_signed_transaction_bcs(txn).await
    }

    pub async fn wait_for_transaction(
        &self,
        pending_transaction: &PendingTransaction,
    ) -> AptosResult<Response<Transaction>> {
        self.wait_for_transaction_by_hash(
            pending_transaction.hash.into(),
            *pending_transaction
                .request
                .expiration_timestamp_secs
                .inner(),
        )
        .await
    }

    pub async fn wait_for_transaction_bcs(
        &self,
        pending_transaction: &PendingTransaction,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        self.wait_for_transaction_by_hash_bcs(
            pending_transaction.hash.into(),
            *pending_transaction
                .request
                .expiration_timestamp_secs
                .inner(),
        )
        .await
    }

    pub async fn wait_for_signed_transaction(
        &self,
        transaction: &SignedTransaction,
    ) -> AptosResult<Response<Transaction>> {
        let expiration_timestamp = transaction.expiration_timestamp_secs();
        self.wait_for_transaction_by_hash(
            transaction.clone().committed_hash(),
            expiration_timestamp,
        )
        .await
    }

    pub async fn wait_for_signed_transaction_bcs(
        &self,
        transaction: &SignedTransaction,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        let expiration_timestamp = transaction.expiration_timestamp_secs();
        self.wait_for_transaction_by_hash_bcs(
            transaction.clone().committed_hash(),
            expiration_timestamp,
        )
        .await
    }

    pub async fn wait_for_transaction_by_hash(
        &self,
        hash: HashValue,
        expiration_timestamp_secs: u64,
    ) -> AptosResult<Response<Transaction>> {
        const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
        const DEFAULT_DELAY: Duration = Duration::from_millis(500);

        let start = std::time::Instant::now();
        while start.elapsed() < DEFAULT_TIMEOUT {
            let resp = self.get_transaction_by_hash_inner(hash).await?;
            if resp.status() != StatusCode::NOT_FOUND {
                let txn_resp: Response<Transaction> = self.json(resp).await?;
                let (transaction, state) = txn_resp.into_parts();

                if !transaction.is_pending() {
                    if !transaction.success() {
                        return Err(anyhow!(
                            "transaction execution failed: {}",
                            transaction.vm_status()
                        ))?;
                    }
                    return Ok(Response::new(transaction, state));
                }
                if expiration_timestamp_secs <= state.timestamp_usecs / 1_000_000 {
                    return Err(anyhow!("transaction expired").into());
                }
            }

            tokio::time::sleep(DEFAULT_DELAY).await;
        }

        Err(anyhow!("timeout").into())
    }

    pub async fn wait_for_transaction_by_hash_bcs(
        &self,
        hash: HashValue,
        expiration_timestamp_secs: u64,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
        const DEFAULT_DELAY: Duration = Duration::from_millis(500);

        let start = std::time::Instant::now();
        while start.elapsed() < DEFAULT_TIMEOUT {
            let resp = self.get_transaction_by_hash_bcs_inner(hash).await?;

            // If it's not found, keep waiting for it
            if resp.status() != StatusCode::NOT_FOUND {
                let resp = self.check_and_parse_bcs_response(resp).await?;
                let resp = resp.and_then(|bytes| bcs::from_bytes(&bytes))?;
                let (maybe_pending_txn, state) = resp.into_parts();

                // If we have a committed transaction, determine if it failed or not
                if let TransactionData::OnChain(txn) = maybe_pending_txn {
                    let status = txn.info.status();

                    // The user can handle the error
                    return match status {
                        ExecutionStatus::Success => Ok(Response::new(txn, state)),
                        _ => Err(anyhow!("Transaction failed").into()),
                    };
                }

                // If it's expired lets give up
                if Duration::from_secs(expiration_timestamp_secs)
                    <= Duration::from_micros(state.timestamp_usecs)
                {
                    return Err(anyhow!("Transaction expired").into());
                }
            }

            tokio::time::sleep(DEFAULT_DELAY).await;
        }

        Err(anyhow!("Timed out waiting for transaction").into())
    }

    pub async fn wait_for_version(&self, version: u64) -> Result<State> {
        const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);
        const DEFAULT_DELAY: Duration = Duration::from_millis(500);

        let start = std::time::Instant::now();
        loop {
            let state = self.get_ledger_information().await?.into_inner();
            if state.version >= version {
                return Ok(state);
            }

            if start.elapsed() >= DEFAULT_TIMEOUT {
                return Err(anyhow!(
                    "timeout when waiting for version {}, only got to {}",
                    version,
                    state.version
                ));
            }

            tokio::time::sleep(DEFAULT_DELAY).await;
        }
    }

    pub async fn get_transactions(
        &self,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<Transaction>>> {
        let url = self.build_path("transactions")?;

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

    pub async fn get_transactions_bcs(
        &self,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<TransactionOnChainData>>> {
        let url = self.build_path("transactions")?;
        let response = self.get_bcs_with_page(url, start, limit).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_transaction_by_hash(
        &self,
        hash: HashValue,
    ) -> AptosResult<Response<Transaction>> {
        self.json(self.get_transaction_by_hash_inner(hash).await?)
            .await
    }

    pub async fn get_transaction_by_hash_bcs(
        &self,
        hash: HashValue,
    ) -> AptosResult<Response<TransactionData>> {
        let response = self.get_transaction_by_hash_bcs_inner(hash).await?;
        let response = self.check_and_parse_bcs_response(response).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_transaction_by_hash_bcs_inner(
        &self,
        hash: HashValue,
    ) -> AptosResult<reqwest::Response> {
        let url = self.build_path(&format!("transactions/by_hash/{}", hash.to_hex_literal()))?;
        let response = self.inner.get(url).header(ACCEPT, BCS).send().await?;
        Ok(response)
    }

    async fn get_transaction_by_hash_inner(
        &self,
        hash: HashValue,
    ) -> AptosResult<reqwest::Response> {
        let url = self.build_path(&format!("transactions/by_hash/{}", hash.to_hex_literal()))?;
        Ok(self.inner.get(url).send().await?)
    }

    pub async fn get_transaction_by_version(
        &self,
        version: u64,
    ) -> AptosResult<Response<Transaction>> {
        self.json(self.get_transaction_by_version_inner(version).await?)
            .await
    }

    pub async fn get_transaction_by_version_bcs(
        &self,
        version: u64,
    ) -> AptosResult<Response<TransactionData>> {
        let url = self.build_path(&format!("transactions/by_version/{}", version))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    async fn get_transaction_by_version_inner(
        &self,
        version: u64,
    ) -> AptosResult<reqwest::Response> {
        let url = self.build_path(&format!("transactions/by_version/{}", version))?;
        Ok(self.inner.get(url).send().await?)
    }

    pub async fn get_account_transactions(
        &self,
        address: AccountAddress,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> AptosResult<Response<Vec<Transaction>>> {
        let url = self.build_path(&format!("accounts/{}/transactions", address))?;

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

    pub async fn get_account_transactions_bcs(
        &self,
        address: AccountAddress,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<TransactionOnChainData>>> {
        let url = self.build_path(&format!("accounts/{}/transactions", address))?;
        let response = self.get_bcs_with_page(url, start, limit).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_resources(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<Vec<Resource>>> {
        let url = self.build_path(&format!("accounts/{}/resources", address))?;

        let response = self.inner.get(url).send().await?;

        self.json(response).await
    }

    pub async fn get_account_resources_bcs(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<BTreeMap<StructTag, Vec<u8>>>> {
        let url = self.build_path(&format!("accounts/{}/resources", address))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_resources_at_version(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> AptosResult<Response<Vec<Resource>>> {
        let url = self.build_path(&format!(
            "accounts/{}/resources?ledger_version={}",
            address, version
        ))?;

        let response = self.inner.get(url).send().await?;

        self.json(response).await
    }

    pub async fn get_account_resources_at_version_bcs(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> AptosResult<Response<BTreeMap<StructTag, Vec<u8>>>> {
        let url = self.build_path(&format!(
            "accounts/{}/resources?ledger_version={}",
            address, version
        ))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_resource<T: DeserializeOwned>(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> AptosResult<Response<T>> {
        let resp = self.get_account_resource(address, resource_type).await?;
        resp.and_then(|conf| {
            if let Some(res) = conf {
                serde_json::from_value(res.data)
                    .map_err(|e| anyhow!("deserialize {} failed: {}", resource_type, e).into())
            } else {
                Err(anyhow!(
                    "could not find resource {} in account {}",
                    resource_type,
                    address
                )
                .into())
            }
        })
    }

    pub async fn get_account_resource(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> AptosResult<Response<Option<Resource>>> {
        let url = self.build_path(&format!("accounts/{}/resource/{}", address, resource_type))?;

        let response = self
            .inner
            .get(url)
            .send()
            .await
            .map_err(anyhow::Error::from)?;
        self.json(response).await
    }

    pub async fn get_account_resource_bcs<T: DeserializeOwned>(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> AptosResult<Response<T>> {
        let url = self.build_path(&format!("accounts/{}/resource/{}", address, resource_type))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_resource_at_version_bcs<T: DeserializeOwned>(
        &self,
        address: AccountAddress,
        resource_type: &str,
        version: u64,
    ) -> AptosResult<Response<T>> {
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}?ledger_version={}",
            address, resource_type, version
        ))?;

        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_resource_at_version(
        &self,
        address: AccountAddress,
        resource_type: &str,
        version: u64,
    ) -> AptosResult<Response<Option<Resource>>> {
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}?ledger_version={}",
            address, resource_type, version
        ))?;

        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    pub async fn get_account_modules(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<Vec<MoveModuleBytecode>>> {
        let url = self.build_path(&format!("accounts/{}/modules", address))?;

        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    pub async fn get_account_modules_bcs(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<BTreeMap<MoveModuleId, Vec<u8>>>> {
        let url = self.build_path(&format!("accounts/{}/modules", address))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_module(
        &self,
        address: AccountAddress,
        module_name: &str,
    ) -> AptosResult<Response<MoveModuleBytecode>> {
        let url = self.build_path(&format!("accounts/{}/module/{}", address, module_name))?;
        self.get(url).await
    }

    pub async fn get_account_module_bcs(
        &self,
        address: AccountAddress,
        module_name: &str,
    ) -> AptosResult<Response<bytes::Bytes>> {
        let url = self.build_path(&format!("accounts/{}/module/{}", address, module_name))?;
        self.get_bcs(url).await
    }

    pub async fn get_account_events(
        &self,
        address: AccountAddress,
        struct_tag: &str,
        field_name: &str,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<VersionedEvent>>> {
        let url = self.build_path(&format!(
            "accounts/{}/events/{}/{}",
            address.to_hex_literal(),
            struct_tag,
            field_name
        ))?;
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

    pub async fn get_account_events_bcs(
        &self,
        address: AccountAddress,
        struct_tag: &str,
        field_name: &str,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<EventWithVersion>>> {
        let url = self.build_path(&format!(
            "accounts/{}/events/{}/{}",
            address.to_hex_literal(),
            struct_tag,
            field_name
        ))?;

        let response = self.get_bcs_with_page(url, start, limit).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_new_block_events_bcs(
        &self,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> Result<Response<Vec<VersionedNewBlockEvent>>> {
        #[derive(Clone, Debug, Serialize, Deserialize)]
        pub struct NewBlockEventResponse {
            hash: String,
            #[serde(deserialize_with = "deserialize_from_string")]
            epoch: u64,
            #[serde(deserialize_with = "deserialize_from_string")]
            round: u64,
            #[serde(deserialize_with = "deserialize_from_string")]
            height: u64,
            #[serde(deserialize_with = "deserialize_from_prefixed_hex_string")]
            previous_block_votes_bitvec: HexEncodedBytes,
            proposer: String,
            failed_proposer_indices: Vec<String>,
            #[serde(deserialize_with = "deserialize_from_string")]
            time_microseconds: u64,
        }

        let response = self
            .get_account_events_bcs(
                CORE_CODE_ADDRESS,
                "0x1::block::BlockResource",
                "new_block_events",
                start,
                limit,
            )
            .await?;

        response.and_then(|events| {
            let new_events: Result<Vec<_>> = events
                .into_iter()
                .map(|event| {
                    let version = event.transaction_version;
                    let sequence_number = event.event.sequence_number();

                    Ok(VersionedNewBlockEvent {
                        event: bcs::from_bytes(event.event.event_data())?,
                        version,
                        sequence_number,
                    })
                })
                .collect();
            new_events
        })
    }

    pub async fn get_table_item<K: Serialize>(
        &self,
        table_handle: AccountAddress,
        key_type: &str,
        value_type: &str,
        key: K,
    ) -> AptosResult<Response<Value>> {
        let url = self.build_path(&format!("tables/{}/item", table_handle))?;
        let data = json!({
            "key_type": key_type,
            "value_type": value_type,
            "key": json!(key),
        });

        let response = self.inner.post(url).json(&data).send().await?;
        self.json(response).await
    }

    pub async fn get_table_item_bcs<K: Serialize, T: DeserializeOwned>(
        &self,
        table_handle: AccountAddress,
        key_type: &str,
        value_type: &str,
        key: K,
    ) -> AptosResult<Response<T>> {
        let url = self.build_path(&format!("tables/{}/item", table_handle))?;
        let data = json!({
            "key_type": key_type,
            "value_type": value_type,
            "key": json!(key),
        });

        let response = self.post_bcs(url, data).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account(&self, address: AccountAddress) -> AptosResult<Response<Account>> {
        let url = self.build_path(&format!("accounts/{}", address))?;
        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    pub async fn get_account_bcs(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<AccountResource>> {
        let url = self.build_path(&format!("accounts/{}", address))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn estimate_gas_price(&self) -> AptosResult<Response<GasEstimation>> {
        let url = self.build_path("estimate_gas_price")?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn set_failpoint(&self, name: String, actions: String) -> AptosResult<String> {
        let mut base = self.build_path("set_failpoint")?;
        let url = base
            .query_pairs_mut()
            .append_pair("name", &name)
            .append_pair("actions", &actions)
            .finish();
        let response = self.inner.get(url.clone()).send().await?;

        if !response.status().is_success() {
            Err(parse_error(response).await)
        } else {
            Ok(response
                .text()
                .await
                .map_err(|e| anyhow::anyhow!("To text failed: {:?}", e))?)
        }
    }

    async fn check_response(
        &self,
        response: reqwest::Response,
    ) -> AptosResult<(reqwest::Response, State)> {
        if !response.status().is_success() {
            Err(parse_error(response).await)
        } else {
            let state = parse_state(&response)?;

            Ok((response, state))
        }
    }

    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        response: reqwest::Response,
    ) -> AptosResult<Response<T>> {
        let (response, state) = self.check_response(response).await?;
        let json = response.json().await.map_err(anyhow::Error::from)?;
        Ok(Response::new(json, state))
    }

    pub async fn health_check(&self, seconds: u64) -> AptosResult<()> {
        let url = self.build_path("-/healthy")?;
        let response = self
            .inner
            .get(url)
            .query(&[("duration_secs", seconds)])
            .send()
            .await?;

        if !response.status().is_success() {
            Err(parse_error(response).await)
        } else {
            Ok(())
        }
    }

    async fn get<T: DeserializeOwned>(&self, url: Url) -> AptosResult<Response<T>> {
        self.json(self.inner.get(url).send().await?).await
    }

    async fn get_bcs(&self, url: Url) -> AptosResult<Response<bytes::Bytes>> {
        let response = self.inner.get(url).header(ACCEPT, BCS).send().await?;
        self.check_and_parse_bcs_response(response).await
    }

    async fn post_bcs(
        &self,
        url: Url,
        data: serde_json::Value,
    ) -> AptosResult<Response<bytes::Bytes>> {
        let response = self
            .inner
            .post(url)
            .header(ACCEPT, BCS)
            .json(&data)
            .send()
            .await?;
        self.check_and_parse_bcs_response(response).await
    }

    async fn get_bcs_with_page(
        &self,
        url: Url,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<bytes::Bytes>> {
        let mut request = self.inner.get(url).header(ACCEPT, BCS);
        if let Some(start) = start {
            request = request.query(&[("start", start)])
        }

        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)])
        }

        let response = request.send().await?;
        self.check_and_parse_bcs_response(response).await
    }

    async fn check_and_parse_bcs_response(
        &self,
        response: reqwest::Response,
    ) -> AptosResult<Response<bytes::Bytes>> {
        let (response, state) = self.check_response(response).await?;
        Ok(Response::new(response.bytes().await?, state))
    }

    pub async fn try_until_ok<F, Fut, RetryFun, T>(
        total_wait: Option<Duration>,
        initial_interval: Option<Duration>,
        should_retry: RetryFun,
        function: F,
    ) -> AptosResult<T>
    where
        F: Fn() -> Fut,
        RetryFun: Fn(StatusCode, Option<AptosError>) -> bool,
        Fut: Future<Output = AptosResult<T>>,
    {
        let total_wait = total_wait.unwrap_or(DEFAULT_MAX_WAIT_DURATION);
        let mut backoff = initial_interval.unwrap_or(DEFAULT_INTERVAL_DURATION);
        let mut result = Err(RestError::Unknown(anyhow!("Failed to run function")));
        let start = Instant::now();

        // TODO: Add jitter
        while start.elapsed() < total_wait {
            result = function().await;

            let retry = match &result {
                Ok(_) => break,
                Err(err) => match err {
                    RestError::Api(inner) => {
                        should_retry(inner.status_code, Some(inner.error.clone()))
                    }
                    RestError::Http(status_code, _e) => should_retry(*status_code, None),
                    RestError::Bcs(_)
                    | RestError::Json(_)
                    | RestError::Timeout(_)
                    | RestError::Unknown(_) => true,
                    RestError::UrlParse(_) => false,
                },
            };

            if !retry {
                break;
            }

            aptos_logger::info!(
                "Failed to call API, retrying in {}ms: {:?}",
                backoff.as_millis(),
                result.as_ref().err().unwrap()
            );

            tokio::time::sleep(backoff).await;
            backoff = backoff.saturating_mul(2);
        }

        result
    }
}

pub fn retriable_with_404(status_code: StatusCode, aptos_error: Option<AptosError>) -> bool {
    retriable(status_code, aptos_error) | matches!(status_code, StatusCode::NOT_FOUND)
}

pub fn retriable(status_code: StatusCode, _aptos_error: Option<AptosError>) -> bool {
    matches!(
        status_code,
        StatusCode::TOO_MANY_REQUESTS
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::GATEWAY_TIMEOUT
            | StatusCode::BAD_GATEWAY
            | StatusCode::INSUFFICIENT_STORAGE
    )
}

impl From<(ReqwestClient, Url)> for Client {
    fn from((inner, base_url): (ReqwestClient, Url)) -> Self {
        Client {
            inner,
            base_url,
            version_path_base: DEFAULT_VERSION_PATH_BASE.to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VersionedNewBlockEvent {
    /// event
    pub event: NewBlockEvent,
    /// version
    pub version: u64,
    /// sequence number
    pub sequence_number: u64,
}

fn parse_state(response: &reqwest::Response) -> AptosResult<State> {
    Ok(State::from_headers(response.headers())?)
}

fn parse_state_optional(response: &reqwest::Response) -> Option<State> {
    State::from_headers(response.headers())
        .map(Some)
        .unwrap_or(None)
}

async fn parse_error(response: reqwest::Response) -> RestError {
    let status_code = response.status();
    let maybe_state = parse_state_optional(&response);
    match response.json::<AptosError>().await {
        Ok(error) => (error, maybe_state, status_code).into(),
        Err(e) => RestError::Http(status_code, e),
    }
}

pub struct GasEstimationParams {
    pub estimated_gas_used: u64,
    pub estimated_gas_price: u64,
}

impl ExplainVMStatus for Client {
    // TODO: Add some caching
    fn get_module_bytecode(&self, module_id: &ModuleId) -> Result<Rc<dyn Bytecode>> {
        let bytes =
            block_on(self.get_account_module_bcs(*module_id.address(), module_id.name().as_str()))?
                .into_inner();

        let compiled_module = CompiledModule::deserialize(bytes.as_ref())?;
        Ok(Rc::new(compiled_module) as Rc<dyn Bytecode>)
    }
}
