// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

extern crate core;

pub mod aptos;
pub mod error;
pub mod faucet;
use error::AptosErrorResponse;
pub use faucet::FaucetClient;
pub mod response;
pub use response::Response;
pub mod client_builder;
pub mod state;
pub mod types;

pub use crate::client_builder::{AptosBaseUrl, ClientBuilder};
use crate::{aptos::AptosVersion, error::RestError};
use anyhow::{anyhow, Result};
pub use aptos_api_types::{
    self, IndexResponseBcs, MoveModuleBytecode, PendingTransaction, Transaction,
};
use aptos_api_types::{
    deserialize_from_string,
    mime_types::{BCS, BCS_SIGNED_TRANSACTION, BCS_VIEW_FUNCTION, JSON},
    AptosError, AptosErrorCode, BcsBlock, Block, GasEstimation, HexEncodedBytes, IndexResponse,
    MoveModuleId, TransactionData, TransactionOnChainData, TransactionsBatchSubmissionResult,
    UserTransaction, VersionedEvent, ViewFunction, ViewRequest,
};
use aptos_crypto::HashValue;
use aptos_logger::{debug, error, info, sample, sample::SampleRate, warn};
use aptos_types::{
    account_address::AccountAddress,
    account_config::{AccountResource, NewBlockEvent, CORE_CODE_ADDRESS},
    contract_event::EventWithVersion,
    keyless::{Groth16Proof, Pepper, ZeroKnowledgeSig, ZKP},
    state_store::state_key::StateKey,
    transaction::{
        authenticator::EphemeralSignature, IndexedTransactionSummary, SignedTransaction,
    },
};
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, StructTag, TypeTag},
};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client as ReqwestClient, StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
pub use state::State;
use std::{collections::BTreeMap, future::Future, str::FromStr, time::Duration};
use tokio::time::Instant;
pub use types::{deserialize_from_prefixed_hex_string, Account, Resource};
use url::Url;

pub const DEFAULT_VERSION_PATH_BASE: &str = "v1/";
const DEFAULT_MAX_WAIT_MS: u64 = 60000;
const DEFAULT_INTERVAL_MS: u64 = 1000;
static DEFAULT_MAX_WAIT_DURATION: Duration = Duration::from_millis(DEFAULT_MAX_WAIT_MS);
static DEFAULT_INTERVAL_DURATION: Duration = Duration::from_millis(DEFAULT_INTERVAL_MS);
const DEFAULT_MAX_SERVER_LAG_WAIT_DURATION: Duration = Duration::from_secs(60);
const RESOURCES_PER_CALL_PAGINATION: u64 = 9999;
const MODULES_PER_CALL_PAGINATION: u64 = 1000;
const X_APTOS_SDK_HEADER_VALUE: &str = concat!("aptos-rust-sdk/", env!("CARGO_PKG_VERSION"));

type AptosResult<T> = Result<T, RestError>;

#[derive(Deserialize)]
pub struct Table {
    pub handle: AccountAddress,
}

#[derive(Deserialize)]
pub struct OriginatingAddress {
    pub address_map: Table,
}

#[derive(Clone, Debug)]
pub struct Client {
    inner: ReqwestClient,
    base_url: Url,
    version_path_base: String,
}

// TODO: Dedupe the pepper/prover request/response types with the ones defined in the service.
#[derive(Clone, Debug, serde::Serialize)]
pub struct PepperRequest {
    pub jwt_b64: String,
    #[serde(with = "hex")]
    pub epk: Vec<u8>,
    #[serde(with = "hex")]
    pub epk_blinder: Vec<u8>,
    pub exp_date_secs: u64,
    pub uid_key: String,
}

#[derive(Debug, serde::Deserialize)]
struct PepperResponse {
    #[serde(with = "hex")]
    pub pepper: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct ProverRequest {
    pub jwt_b64: String,
    #[serde(with = "hex")]
    pub epk: Vec<u8>,
    #[serde(with = "hex")]
    pub epk_blinder: Vec<u8>,
    pub exp_date_secs: u64,
    pub exp_horizon_secs: u64,
    #[serde(with = "hex")]
    pub pepper: Vec<u8>,
    pub uid_key: String,
}

#[derive(Debug, serde::Deserialize)]
struct ProverResponse {
    proof: Groth16Proof,
    #[serde(with = "hex")]
    #[allow(dead_code)]
    public_inputs_hash: [u8; 32],
    #[serde(with = "hex")]
    training_wheels_signature: Vec<u8>,
}

impl Client {
    pub fn builder(aptos_base_url: AptosBaseUrl) -> ClientBuilder {
        ClientBuilder::new(aptos_base_url)
    }

    pub fn new(base_url: Url) -> Self {
        Self::builder(AptosBaseUrl::Custom(base_url)).build()
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

    pub fn build_path(&self, path: &str) -> AptosResult<Url> {
        Ok(self.base_url.join(&self.version_path_base)?.join(path)?)
    }

    pub fn get_prover_url(&self) -> Url {
        self.base_url.join("keyless/prover/v0/prove").unwrap()
    }

    pub fn get_pepper_url(&self) -> Url {
        self.base_url.join("keyless/pepper/v0/fetch").unwrap()
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

    pub async fn lookup_address(
        &self,
        address_key: AccountAddress,
        must_exist: bool,
    ) -> AptosResult<Response<AccountAddress>> {
        let originating_address_table: Response<OriginatingAddress> = self
            .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::account::OriginatingAddress")
            .await?;

        let table_handle = originating_address_table.inner().address_map.handle;

        // The derived address that can be used to look up the original address
        match self
            .get_table_item_bcs(
                table_handle,
                "address",
                "address",
                address_key.to_hex_literal(),
            )
            .await
        {
            Ok(inner) => Ok(inner),
            Err(RestError::Api(AptosErrorResponse {
                error:
                    AptosError {
                        error_code: AptosErrorCode::TableItemNotFound,
                        ..
                    },
                ..
            })) => {
                // If the table item wasn't found, we may check if the account exists
                if !must_exist {
                    Ok(Response::new(
                        address_key,
                        originating_address_table.state().clone(),
                    ))
                } else {
                    self.get_account_bcs(address_key)
                        .await
                        .map(|account_resource| {
                            Response::new(address_key, account_resource.state().clone())
                        })
                }
            },
            Err(err) => Err(err),
        }
    }

    /// Gets the balance of a specific asset type for an account.
    /// The `asset_type` parameter can be either:
    /// * A coin type (e.g. "0x1::aptos_coin::AptosCoin")
    /// * A fungible asset metadata address (e.g. "0xa")
    /// For more details, see: https://aptos.dev/en/build/apis/fullnode-rest-api-reference#tag/accounts/GET/accounts/{address}/balance/{asset_type}
    pub async fn get_account_balance(
        &self,
        address: AccountAddress,
        asset_type: &str,
    ) -> AptosResult<Response<u64>> {
        let url = self.build_path(&format!(
            "accounts/{}/balance/{}",
            address.to_hex(),
            asset_type
        ))?;
        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    /// Internal implementation for viewing coin balance at a specific version.
    /// Note: This function should only be used when you need to check coin balance at a specific version.
    /// This function does not support fungible assets - use `get_account_balance` instead for fungible assets.
    pub async fn view_account_balance_bcs_impl(
        &self,
        address: AccountAddress,
        coin_type: &str,
        version: Option<u64>,
    ) -> AptosResult<Response<u64>> {
        let resp: Response<Vec<u64>> = self
            .view_bcs(
                &ViewFunction {
                    module: ModuleId::new(AccountAddress::ONE, ident_str!("coin").into()),
                    function: ident_str!("balance").into(),
                    ty_args: vec![TypeTag::Struct(Box::new(
                        StructTag::from_str(coin_type).unwrap(),
                    ))],
                    args: vec![bcs::to_bytes(&address).unwrap()],
                },
                version,
            )
            .await?;

        resp.and_then(|result| {
            if result.len() != 1 {
                Err(anyhow!("Wrong data size returned: {:?}", result).into())
            } else {
                Ok(result[0])
            }
        })
    }

    pub async fn view_apt_account_balance_at_version(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> AptosResult<Response<u64>> {
        self.view_account_balance_bcs_impl(address, "0x1::aptos_coin::AptosCoin", Some(version))
            .await
    }

    pub async fn view_apt_account_balance(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<u64>> {
        self.view_account_balance_bcs_impl(address, "0x1::aptos_coin::AptosCoin", None)
            .await
    }

    pub async fn get_index(&self) -> AptosResult<Response<IndexResponse>> {
        self.get(self.build_path("")?).await
    }

    pub async fn get_index_bcs(&self) -> AptosResult<Response<IndexResponseBcs>> {
        let url = self.build_path("")?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    // TODO: Remove this, just use `get_index`: https://github.com/aptos-labs/aptos-core/issues/5597.
    pub async fn get_ledger_information(&self) -> AptosResult<Response<State>> {
        let response = self.get_index_bcs().await?.map(|r| State {
            chain_id: r.chain_id,
            epoch: r.epoch.into(),
            version: r.ledger_version.into(),
            timestamp_usecs: r.ledger_timestamp.into(),
            oldest_ledger_version: r.oldest_ledger_version.into(),
            oldest_block_height: r.oldest_block_height.into(),
            block_height: r.block_height.into(),
            cursor: None,
        });
        assert_eq!(response.inner().chain_id, response.state().chain_id);
        assert_eq!(response.inner().epoch, response.state().epoch);
        assert_eq!(response.inner().version, response.state().version);
        assert_eq!(response.inner().block_height, response.state().block_height);

        Ok(response)
    }

    pub async fn view(
        &self,
        request: &ViewRequest,
        version: Option<u64>,
    ) -> AptosResult<Response<Vec<serde_json::Value>>> {
        let request = serde_json::to_string(request)?;
        let mut url = self.build_path("view")?;
        if let Some(version) = version {
            url.set_query(Some(format!("ledger_version={}", version).as_str()));
        }

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, JSON)
            .body(request)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn view_bcs<T: DeserializeOwned>(
        &self,
        request: &ViewFunction,
        version: Option<u64>,
    ) -> AptosResult<Response<T>> {
        let txn_payload = bcs::to_bytes(request)?;
        let mut url = self.build_path("view")?;
        if let Some(version) = version {
            url.set_query(Some(format!("ledger_version={}", version).as_str()));
        }

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_VIEW_FUNCTION)
            .header(ACCEPT, BCS)
            .body(txn_payload)
            .send()
            .await?;

        let response = self.check_and_parse_bcs_response(response).await?;
        Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
    }

    pub async fn view_bcs_with_json_response(
        &self,
        request: &ViewFunction,
        version: Option<u64>,
    ) -> AptosResult<Response<Vec<serde_json::Value>>> {
        let txn_payload = bcs::to_bytes(request)?;
        let mut url = self.build_path("view")?;
        if let Some(version) = version {
            url.set_query(Some(format!("ledger_version={}", version).as_str()));
        }

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_VIEW_FUNCTION)
            .header(ACCEPT, JSON)
            .body(txn_payload)
            .send()
            .await?;

        self.json(response).await
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
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
            .body(txn_payload)
            .send()
            .await?;

        self.json(response).await
    }

    pub async fn simulate_with_gas_estimation(
        &self,
        txn: &SignedTransaction,
        estimate_max_gas_amount: bool,
        estimate_max_gas_unit_price: bool,
    ) -> AptosResult<Response<Vec<UserTransaction>>> {
        let txn_payload = bcs::to_bytes(txn)?;

        let url = self.build_path(&format!(
            "transactions/simulate?estimate_max_gas_amount={}&estimate_gas_unit_price={}",
            estimate_max_gas_amount, estimate_max_gas_unit_price
        ))?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
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
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
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
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
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
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
            .body(txn_payload)
            .send()
            .await?;

        self.json::<PendingTransaction>(response).await
    }

    pub async fn submit_without_deserializing_response(
        &self,
        txn: &SignedTransaction,
    ) -> Result<()> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path("transactions")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
            .body(txn_payload)
            .send()
            .await?;

        self.check_response(response).await?;
        Ok(())
    }

    pub async fn submit_bcs(&self, txn: &SignedTransaction) -> AptosResult<Response<()>> {
        let txn_payload = bcs::to_bytes(txn)?;
        let url = self.build_path("transactions")?;

        let response = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
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
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
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
            .header(CONTENT_TYPE, BCS_SIGNED_TRANSACTION)
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
            Some(DEFAULT_MAX_SERVER_LAG_WAIT_DURATION),
            None,
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
            Some(DEFAULT_MAX_SERVER_LAG_WAIT_DURATION),
            None,
        )
        .await
    }

    pub async fn wait_for_signed_transaction(
        &self,
        transaction: &SignedTransaction,
    ) -> AptosResult<Response<Transaction>> {
        let expiration_timestamp = transaction.expiration_timestamp_secs();
        self.wait_for_transaction_by_hash(
            transaction.committed_hash(),
            expiration_timestamp,
            Some(DEFAULT_MAX_SERVER_LAG_WAIT_DURATION),
            None,
        )
        .await
    }

    pub async fn wait_for_signed_transaction_bcs(
        &self,
        transaction: &SignedTransaction,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        let expiration_timestamp = transaction.expiration_timestamp_secs();
        self.wait_for_transaction_by_hash_bcs(
            transaction.committed_hash(),
            expiration_timestamp,
            Some(DEFAULT_MAX_SERVER_LAG_WAIT_DURATION),
            None,
        )
        .await
    }

    /// Implementation of waiting for a transaction
    /// * `hash`: hash of the submitted transaction
    /// * `expiration_timestamp_secs`: expiration time of the submitted transaction
    /// * `max_server_lag_wait`:
    ///     Fullnodes generally lag some amount behind the authoritative blockchain ledger state.
    ///     This field gives the node some time to update its ledger state to the point
    ///     where your transaction might have expired.
    ///     We recommend setting this value to at least 60 seconds.
    /// * `timeout_from_call`:
    ///     When an absolute timeout for this function is needed,
    ///     irrespective of whether expiry time is reached.
    async fn wait_for_transaction_by_hash_inner<F, Fut, T>(
        &self,
        hash: HashValue,
        expiration_timestamp_secs: u64,
        max_server_lag_wait: Option<Duration>,

        timeout_from_call: Option<Duration>,
        fetch: F,
    ) -> AptosResult<Response<T>>
    where
        F: Fn(HashValue) -> Fut,
        Fut: Future<Output = AptosResult<WaitForTransactionResult<T>>>,
    {
        // TODO: make this configurable
        const DEFAULT_DELAY: Duration = Duration::from_millis(500);
        let mut reached_mempool = false;
        let start = std::time::Instant::now();
        loop {
            let mut chain_timestamp_usecs = None;
            match fetch(hash).await {
                Ok(WaitForTransactionResult::Success(result)) => {
                    return Ok(result);
                },
                Ok(WaitForTransactionResult::FailedExecution(vm_status)) => {
                    return Err(anyhow!(
                        "Transaction committed on chain, but failed execution: {}",
                        vm_status
                    ))?;
                },
                Ok(WaitForTransactionResult::Pending(state)) => {
                    reached_mempool = true;
                    if expiration_timestamp_secs <= state.timestamp_usecs / 1_000_000 {
                        return Err(anyhow!("Transaction expired. It is guaranteed it will not be committed on chain.").into());
                    }
                    chain_timestamp_usecs = Some(state.timestamp_usecs);
                },
                Ok(WaitForTransactionResult::NotFound(error)) => {
                    sample!(
                        SampleRate::Duration(Duration::from_secs(5)),
                        warn!(
                            "Cannot yet find transaction in mempool on {:?}, continuing to wait, error is {:?}.",
                            self.path_prefix_string(), error
                        )
                    );
                    if let RestError::Api(aptos_error_response) = error {
                        if let Some(state) = aptos_error_response.state {
                            if expiration_timestamp_secs <= state.timestamp_usecs / 1_000_000 {
                                if reached_mempool {
                                    return Err(anyhow!("Used to be pending and now not found. Transaction expired. It is guaranteed it will not be committed on chain.").into());
                                } else {
                                    // We want to know whether we ever got Pending state from the mempool,
                                    // to warn in case we didn't.
                                    // Unless we are calling endpoint that is a very large load-balanced pool of nodes,
                                    // we should always see pending after submitting a transaction.
                                    // (i.e. if we hit the node we submitted a transaction to,
                                    // it shouldn't return NotFound on the first call)
                                    //
                                    // At the end, when the expiration happens, we might get NotFound or Pending
                                    // based on whether GC run on the full node to remove expired transaction,
                                    // so that information is not useful. So we need to keep this variable as state.
                                    return Err(anyhow!("Transaction expired, without being seen in mempool. It is guaranteed it will not be committed on chain.").into());
                                }
                            }
                            chain_timestamp_usecs = Some(state.timestamp_usecs);
                        }
                    } else {
                        return Err(error);
                    }
                    sample!(
                        SampleRate::Duration(Duration::from_secs(30)),
                        debug!(
                            "Cannot yet find transaction in mempool on {:?}, continuing to wait.",
                            self.path_prefix_string(),
                        )
                    );
                },
                Err(err) => {
                    debug!("Fetching error, will retry: {}", err);
                },
            }

            if let Some(max_server_lag_wait_duration) = max_server_lag_wait {
                if aptos_infallible::duration_since_epoch().as_secs()
                    > expiration_timestamp_secs + max_server_lag_wait_duration.as_secs()
                {
                    return Err(anyhow!(
                        "Ledger on endpoint ({}) is more than {}s behind current time, timing out waiting for the transaction. Warning, transaction ({}) might still succeed.",
                        self.path_prefix_string(),
                        max_server_lag_wait_duration.as_secs(),
                        hash,
                    ).into());
                }
            }

            let elapsed = start.elapsed();
            if let Some(timeout_duration) = timeout_from_call {
                if elapsed > timeout_duration {
                    return Err(anyhow!(
                        "Timeout of {}s after calling wait_for_transaction reached. Warning, transaction ({}) might still succeed.",
                        timeout_duration.as_secs(),
                        hash,
                    ).into());
                }
            }

            if elapsed.as_secs() > 30 {
                sample!(
                    SampleRate::Duration(Duration::from_secs(30)),
                    debug!(
                        "Continuing to wait for transaction {}, ledger on endpoint ({}) is {}",
                        hash,
                        self.path_prefix_string(),
                        if let Some(timestamp_usecs) = chain_timestamp_usecs {
                            format!(
                                "{}s behind current time",
                                aptos_infallible::duration_since_epoch()
                                    .saturating_sub(Duration::from_micros(timestamp_usecs))
                                    .as_secs()
                            )
                        } else {
                            "unreachable".to_string()
                        },
                    )
                );
            }

            tokio::time::sleep(DEFAULT_DELAY).await;
        }
    }

    pub async fn wait_for_transaction_by_hash(
        &self,
        hash: HashValue,
        expiration_timestamp_secs: u64,
        max_server_lag_wait: Option<Duration>,
        timeout_from_call: Option<Duration>,
    ) -> AptosResult<Response<Transaction>> {
        self.wait_for_transaction_by_hash_inner(
            hash,
            expiration_timestamp_secs,
            max_server_lag_wait,
            timeout_from_call,
            |hash| async move {
                let resp = self.get_transaction_by_hash_inner(hash).await?;
                if resp.status() != StatusCode::NOT_FOUND {
                    let txn_resp: Response<Transaction> = self.json(resp).await?;
                    let (transaction, state) = txn_resp.into_parts();

                    if !transaction.is_pending() {
                        if !transaction.success() {
                            Ok(WaitForTransactionResult::FailedExecution(
                                transaction.vm_status(),
                            ))
                        } else {
                            Ok(WaitForTransactionResult::Success(Response::new(
                                transaction,
                                state,
                            )))
                        }
                    } else {
                        Ok(WaitForTransactionResult::Pending(state))
                    }
                } else {
                    let error_response = parse_error(resp).await;
                    Ok(WaitForTransactionResult::NotFound(error_response))
                }
            },
        )
        .await
    }

    pub async fn wait_for_transaction_by_hash_bcs(
        &self,
        hash: HashValue,
        expiration_timestamp_secs: u64,
        max_server_lag_wait: Option<Duration>,
        timeout_from_call: Option<Duration>,
    ) -> AptosResult<Response<TransactionOnChainData>> {
        self.wait_for_transaction_by_hash_inner(
            hash,
            expiration_timestamp_secs,
            max_server_lag_wait,
            timeout_from_call,
            |hash| async move {
                let resp = self.get_transaction_by_hash_bcs_inner(hash).await?;
                if resp.status() != StatusCode::NOT_FOUND {
                    let resp = self.check_and_parse_bcs_response(resp).await?;
                    let resp = resp.and_then(|bytes| bcs::from_bytes(&bytes))?;
                    let (maybe_pending_txn, state) = resp.into_parts();

                    // If we have a committed transaction, determine if it failed or not
                    if let TransactionData::OnChain(txn) = maybe_pending_txn {
                        let status = txn.info.status();

                        if status.is_success() {
                            Ok(WaitForTransactionResult::Success(Response::new(txn, state)))
                        } else {
                            Ok(WaitForTransactionResult::FailedExecution(format!(
                                "{:?}",
                                status
                            )))
                        }
                    } else {
                        Ok(WaitForTransactionResult::Pending(state))
                    }
                } else {
                    let error_response = parse_error(resp).await;
                    Ok(WaitForTransactionResult::NotFound(error_response))
                }
            },
        )
        .await
    }

    pub async fn wait_for_version(&self, version: u64) -> Result<State> {
        const DEFAULT_TIMEOUT: Duration = Duration::from_secs(240);
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

    pub async fn get_account_ordered_transactions(
        &self,
        address: AccountAddress,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> AptosResult<Response<Vec<Transaction>>> {
        let url = self.build_path(&format!("accounts/{}/transactions", address.to_hex()))?;

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

    pub async fn get_account_ordered_transactions_bcs(
        &self,
        address: AccountAddress,
        start: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<TransactionOnChainData>>> {
        let url = self.build_path(&format!("accounts/{}/transactions", address.to_hex()))?;
        let response = self.get_bcs_with_page(url, start, limit).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_resources(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<Vec<Resource>>> {
        self.paginate_with_cursor(
            &format!("accounts/{}/resources", address.to_hex()),
            RESOURCES_PER_CALL_PAGINATION,
            None,
        )
        .await
    }

    pub async fn get_account_resources_bcs(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<BTreeMap<StructTag, Vec<u8>>>> {
        self.paginate_with_cursor_bcs(
            &format!("accounts/{}/resources", address.to_hex()),
            RESOURCES_PER_CALL_PAGINATION,
            None,
        )
        .await
    }

    pub async fn get_account_resources_at_version(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> AptosResult<Response<Vec<Resource>>> {
        self.paginate_with_cursor(
            &format!("accounts/{}/resources", address.to_hex()),
            RESOURCES_PER_CALL_PAGINATION,
            Some(version),
        )
        .await
    }

    pub async fn get_account_resources_at_version_bcs(
        &self,
        address: AccountAddress,
        version: u64,
    ) -> AptosResult<Response<BTreeMap<StructTag, Vec<u8>>>> {
        self.paginate_with_cursor_bcs(
            &format!("accounts/{}/resources", address.to_hex()),
            RESOURCES_PER_CALL_PAGINATION,
            Some(version),
        )
        .await
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
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}",
            address.to_hex(),
            resource_type
        ))?;

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
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}",
            address.to_hex(),
            resource_type
        ))?;
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
            address.to_hex(),
            resource_type,
            version
        ))?;

        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_resource_at_version_bytes(
        &self,
        address: AccountAddress,
        resource_type: &str,
        version: u64,
    ) -> AptosResult<Response<Vec<u8>>> {
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}?ledger_version={}",
            address.to_hex(),
            resource_type,
            version
        ))?;

        let response = self.get_bcs(url).await?;
        Ok(response.map(|inner| inner.to_vec()))
    }

    pub async fn get_account_resource_bytes(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> AptosResult<Response<Vec<u8>>> {
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}",
            address.to_hex(),
            resource_type
        ))?;

        let response = self.get_bcs(url).await?;
        Ok(response.map(|inner| inner.to_vec()))
    }

    pub async fn get_account_resource_at_version(
        &self,
        address: AccountAddress,
        resource_type: &str,
        version: u64,
    ) -> AptosResult<Response<Option<Resource>>> {
        let url = self.build_path(&format!(
            "accounts/{}/resource/{}?ledger_version={}",
            address.to_hex(),
            resource_type,
            version
        ))?;

        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    pub async fn get_account_modules(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<Vec<MoveModuleBytecode>>> {
        self.paginate_with_cursor(
            &format!("accounts/{}/modules", address.to_hex()),
            MODULES_PER_CALL_PAGINATION,
            None,
        )
        .await
    }

    pub async fn get_account_modules_bcs(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<BTreeMap<MoveModuleId, Vec<u8>>>> {
        self.paginate_with_cursor_bcs(
            &format!("accounts/{}/modules", address.to_hex()),
            MODULES_PER_CALL_PAGINATION,
            None,
        )
        .await
    }

    pub async fn get_account_module(
        &self,
        address: AccountAddress,
        module_name: &str,
    ) -> AptosResult<Response<MoveModuleBytecode>> {
        let url = self.build_path(&format!(
            "accounts/{}/module/{}",
            address.to_hex(),
            module_name
        ))?;
        self.get(url).await
    }

    pub async fn get_account_module_bcs(
        &self,
        address: AccountAddress,
        module_name: &str,
    ) -> AptosResult<Response<bytes::Bytes>> {
        let url = self.build_path(&format!(
            "accounts/{}/module/{}",
            address.to_hex(),
            module_name
        ))?;
        self.get_bcs(url).await
    }

    pub async fn get_account_module_bcs_at_version(
        &self,
        address: AccountAddress,
        module_name: &str,
        version: u64,
    ) -> AptosResult<Response<bytes::Bytes>> {
        let url = self.build_path(&format!(
            "accounts/{}/module/{}?ledger_version={}",
            address.to_hex(),
            module_name,
            version
        ))?;
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

    pub async fn get_account_sequence_number(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<u64>> {
        let res = self.get_account_bcs(address).await;

        match res {
            Ok(account) => account.and_then(|account| Ok(account.sequence_number())),
            Err(error) => match error {
                RestError::Api(error) => {
                    if matches!(error.error.error_code, AptosErrorCode::AccountNotFound) {
                        if let Some(state) = error.state {
                            Ok(Response::new(0, state))
                        } else {
                            let ledger_info = self.get_ledger_information().await?;
                            Ok(Response::new(0, ledger_info.state().clone()))
                        }
                    } else {
                        Err(error::RestError::Api(error))
                    }
                },
                _ => Err(error),
            },
        }
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
                    let event = event.event.v1()?;
                    let sequence_number = event.sequence_number();

                    Ok(VersionedNewBlockEvent {
                        event: bcs::from_bytes(event.event_data())?,
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

    pub async fn get_table_item_at_version<K: Serialize>(
        &self,
        table_handle: AccountAddress,
        key_type: &str,
        value_type: &str,
        key: K,
        version: u64,
    ) -> AptosResult<Response<Value>> {
        let url = self.build_path(&format!(
            "tables/{}/item?ledger_version={}",
            table_handle, version
        ))?;
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

    pub async fn get_table_item_bcs_at_version<K: Serialize, T: DeserializeOwned>(
        &self,
        table_handle: AccountAddress,
        key_type: &str,
        value_type: &str,
        key: K,
        version: u64,
    ) -> AptosResult<Response<T>> {
        let url = self.build_path(&format!(
            "tables/{}/item?ledger_version={}",
            table_handle, version
        ))?;
        let data = json!({
            "key_type": key_type,
            "value_type": value_type,
            "key": json!(key),
        });

        let response = self.post_bcs(url, data).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_raw_table_item(
        &self,
        table_handle: AccountAddress,
        key: &[u8],
        version: u64,
    ) -> AptosResult<Response<Vec<u8>>> {
        let url = self.build_path(&format!(
            "tables/{}/raw_item?ledger_version={}",
            table_handle, version
        ))?;
        let data = json!({
            "key": hex::encode(key),
        });

        let response = self.post_bcs(url, data).await?;
        Ok(response.map(|inner| inner.to_vec()))
    }

    pub async fn get_raw_state_value(
        &self,
        state_key: &StateKey,
        version: u64,
    ) -> AptosResult<Response<Vec<u8>>> {
        let url = self.build_path(&format!(
            "experimental/state_values/raw?ledger_version={}",
            version
        ))?;
        let data = json!({
            "key": hex::encode(bcs::to_bytes(state_key)?),
        });

        let response = self.post_bcs(url, data).await?;
        Ok(response.map(|inner| inner.to_vec()))
    }

    pub async fn get_account(&self, address: AccountAddress) -> AptosResult<Response<Account>> {
        let url = self.build_path(&format!("accounts/{}", address.to_hex()))?;
        let response = self.inner.get(url).send().await?;
        self.json(response).await
    }

    pub async fn get_account_bcs(
        &self,
        address: AccountAddress,
    ) -> AptosResult<Response<AccountResource>> {
        let url = self.build_path(&format!("accounts/{}", address.to_hex()))?;
        let response = self.get_bcs(url).await?;
        Ok(response.and_then(|inner| bcs::from_bytes(&inner))?)
    }

    pub async fn get_account_transaction_summaries(
        &self,
        address: AccountAddress,
        start_version: Option<u64>,
        end_version: Option<u64>,
        limit: Option<u16>,
    ) -> AptosResult<Response<Vec<IndexedTransactionSummary>>> {
        let url = self.build_path(&format!(
            "accounts/{}/transaction_summaries",
            address.to_hex()
        ))?;

        let mut request = self.inner.get(url).header(ACCEPT, BCS);
        if let Some(start_version) = start_version {
            request = request.query(&[("start_version", start_version)])
        }

        if let Some(end_version) = end_version {
            request = request.query(&[("end_version", end_version)])
        }

        if let Some(limit) = limit {
            request = request.query(&[("limit", limit)])
        }

        let response = request.send().await?;
        match self.check_and_parse_bcs_response(response).await {
            Ok(response) => match response.and_then(|inner| bcs::from_bytes(&inner)) {
                Ok(resp) => {
                    let txns: &Vec<IndexedTransactionSummary> = resp.inner();
                    for txn in txns {
                        info!("Got account transaction summaries successfully. (address: {:?}, replay_protector: {:?})", txn.sender(), txn.replay_protector());
                    }
                    Ok(resp)
                },
                Err(e) => {
                    error!("Failed to deserialize account transaction summaries: {:?}, address: {:?}, start_version: {:?}", e, address, start_version);
                    Err(e)?
                },
            },
            Err(e) => {
                error!("Failed to get account transaction summaries: {:?}", e);
                Err(e)
            },
        }
    }

    pub async fn estimate_gas_price(&self) -> AptosResult<Response<GasEstimation>> {
        let url = self.build_path("estimate_gas_price")?;
        let response = self.inner.get(url).send().await?;
        self.json(response).await
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

    pub async fn make_prover_request(&self, req: ProverRequest) -> AptosResult<ZeroKnowledgeSig> {
        let response: ProverResponse = self
            .post_json_no_state(self.get_prover_url(), serde_json::to_value(req.clone())?)
            .await?;
        let proof = response.proof;
        let ephem_sig = Some(
            EphemeralSignature::try_from(response.training_wheels_signature.as_slice())
                .map_err(anyhow::Error::from)?,
        );
        Ok(ZeroKnowledgeSig {
            proof: ZKP::Groth16(proof),
            exp_horizon_secs: req.exp_horizon_secs,
            extra_field: None,
            override_aud_val: None,
            training_wheels_signature: ephem_sig,
        })
    }

    pub async fn make_pepper_request(&self, req: PepperRequest) -> AptosResult<Pepper> {
        let response: PepperResponse = self
            .post_json_no_state(self.get_pepper_url(), serde_json::to_value(req.clone())?)
            .await?;
        let pepper = response.pepper;
        Ok(Pepper::new(
            pepper.as_slice().try_into().map_err(anyhow::Error::from)?,
        ))
    }

    async fn post_json_no_state<T: serde::de::DeserializeOwned>(
        &self,
        url: Url,
        data: serde_json::Value,
    ) -> AptosResult<T> {
        let response = self
            .inner
            .post(url)
            .header(ACCEPT, JSON)
            .json(&data)
            .send()
            .await?;
        if !response.status().is_success() {
            Err(parse_error(response).await)
        } else {
            Ok(response.json().await.map_err(anyhow::Error::from)?)
        }
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
                    },
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

            info!(
                "Failed to call API, retrying in {}ms: {:?}",
                backoff.as_millis(),
                result.as_ref().err().unwrap()
            );

            tokio::time::sleep(backoff).await;
            backoff = backoff.saturating_mul(2);
        }

        result
    }

    /// This function builds a URL for use in pagination. It handles setting a limit,
    /// adding the cursor, and adding a ledger version if given.
    pub fn build_url_for_pagination(
        &self,
        base: &str,
        limit_per_request: u64,
        ledger_version: Option<u64>,
        cursor: &Option<String>,
    ) -> AptosResult<Url> {
        let mut path = format!("{}?limit={}", base, limit_per_request);
        if let Some(ledger_version) = ledger_version {
            path = format!("{}&ledger_version={}", path, ledger_version);
        }
        if let Some(cursor) = cursor {
            path = format!("{}&start={}", path, cursor);
        }
        self.build_path(&path)
    }

    /// This function calls an endpoint that has pagination support and paginates
    /// using the cursor the API returns. It keeps paginating until the API doesn't
    /// return a cursor anymore. Since the functions calling this function are
    /// expected to return the data wrapped in a Response (exactly one), we return
    /// the full results merged together wrapped in the Response we received from
    /// the final call.
    pub async fn paginate_with_cursor<T: for<'a> Deserialize<'a>>(
        &self,
        base_path: &str,
        limit_per_request: u64,
        ledger_version: Option<u64>,
    ) -> AptosResult<Response<Vec<T>>> {
        let mut result = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let url = self.build_url_for_pagination(
                base_path,
                limit_per_request,
                ledger_version,
                &cursor,
            )?;
            let raw_response = self.inner.get(url).send().await?;
            let response: Response<Vec<T>> = self.json(raw_response).await?;
            cursor.clone_from(&response.state().cursor);
            if cursor.is_none() {
                break Ok(response.map(|mut v| {
                    result.append(&mut v);
                    result
                }));
            } else {
                result.extend(response.into_inner());
            }
        }
    }

    /// This function works just like `paginate_with_cursor`, but it calls the internal
    /// helper functions for dealing with BCS data and collects data in the format we
    /// use for BCS endpoint functions.
    pub async fn paginate_with_cursor_bcs<T: for<'a> Deserialize<'a> + Ord>(
        &self,
        base_path: &str,
        limit_per_request: u64,
        ledger_version: Option<u64>,
    ) -> AptosResult<Response<BTreeMap<T, Vec<u8>>>> {
        let mut result = BTreeMap::new();
        let mut cursor: Option<String> = None;

        loop {
            let url = self.build_url_for_pagination(
                base_path,
                limit_per_request,
                ledger_version,
                &cursor,
            )?;
            let response: Response<BTreeMap<T, Vec<u8>>> = self
                .get_bcs(url)
                .await?
                .and_then(|inner| bcs::from_bytes(&inner))?;
            cursor.clone_from(&response.state().cursor);
            if cursor.is_none() {
                break Ok(response.map(|mut v| {
                    result.append(&mut v);
                    result
                }));
            } else {
                result.extend(response.into_inner());
            }
        }
    }
}

// If the user provided no version in the path, use the default. If the
// provided version has no trailing slash, add it, otherwise url.join
// will ignore the version path base.
pub fn get_version_path_with_base(base_url: Url) -> String {
    match base_url.path() {
        "/" => DEFAULT_VERSION_PATH_BASE.to_string(),
        path => {
            if !path.ends_with('/') {
                format!("{}/", path)
            } else {
                path.to_string()
            }
        },
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

enum WaitForTransactionResult<T> {
    NotFound(RestError),
    FailedExecution(String),
    Pending(State),
    Success(Response<T>),
}
