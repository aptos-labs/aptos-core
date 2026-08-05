// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! A mockable seam over the subset of [`aptos_rest_client::Client`] that Rosetta uses.
//!
//! Rosetta handlers historically called the concrete `aptos_rest_client::Client`
//! directly, which made them impossible to unit-test offline.  This trait captures
//! exactly the node calls the handlers make, so tests can inject a fake node
//! (see `mockall::automock`) while production wraps the real REST client.
//!
//! # Object safety
//!
//! The trait is deliberately object-safe (used as `Arc<dyn NodeClient>`).  The
//! REST client's generic BCS getters (`get_account_resource_at_version_bcs::<T>`,
//! `view_bcs::<T>`) are replaced here with byte-returning methods; typed
//! deserialization is done by the free helpers at the bottom of this module,
//! which mirror the REST client's own `and_then(bcs::from_bytes)?` behavior so
//! error semantics (including `RestError::Bcs`) are preserved.

use aptos_rest_client::{
    aptos_api_types::{BcsBlock, GasEstimation, TransactionOnChainData, ViewFunction, ViewRequest},
    error::RestError,
    Account, Client, Response, State,
};
use aptos_types::{account_address::AccountAddress, transaction::SignedTransaction};
use serde::de::DeserializeOwned;
use std::{fmt::Debug, sync::Arc};

/// Result type matching `aptos_rest_client`'s, so callers keep matching on
/// [`RestError`] variants (e.g. `AptosErrorCode::AccountNotFound`) unchanged.
pub type NodeResult<T> = Result<T, RestError>;

/// The subset of node (REST) operations that Rosetta depends on.
#[async_trait::async_trait]
#[cfg_attr(test, mockall::automock)]
pub trait NodeClient: Debug + Send + Sync {
    /// Current ledger state (chain id, versions, block height, timestamp).
    async fn get_ledger_information(&self) -> NodeResult<Response<State>>;

    /// Account summary (sequence number, authentication key).
    async fn get_account(&self, address: AccountAddress) -> NodeResult<Response<Account>>;

    /// Raw BCS bytes of a resource at a specific ledger version.
    async fn get_account_resource_at_version_bytes(
        &self,
        address: AccountAddress,
        resource_type: &str,
        version: u64,
    ) -> NodeResult<Response<Vec<u8>>>;

    /// Raw BCS bytes of a resource at the latest ledger version.
    async fn get_account_resource_bytes(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> NodeResult<Response<Vec<u8>>>;

    /// A BCS view-function call returning a vector of `u64`s (the only shape
    /// Rosetta uses for BCS views: balances and stake amounts).
    async fn view_bcs_u64s(
        &self,
        request: &ViewFunction,
        version: Option<u64>,
    ) -> NodeResult<Response<Vec<u64>>>;

    /// A JSON view-function call (used for delegation-pool balances/lockup).
    async fn view_json(
        &self,
        request: &ViewRequest,
        version: Option<u64>,
    ) -> NodeResult<Response<Vec<serde_json::Value>>>;

    /// A block by height, without transactions.
    async fn get_block_by_height_bcs(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> NodeResult<Response<BcsBlock>>;

    /// A block by height, paging in all of its transactions.
    async fn get_full_block_by_height_bcs(
        &self,
        height: u64,
        page_size: u16,
    ) -> NodeResult<Response<BcsBlock>>;

    /// The current gas price estimate.
    async fn estimate_gas_price(&self) -> NodeResult<Response<GasEstimation>>;

    /// Simulate a transaction, optionally estimating max gas / gas unit price.
    async fn simulate_bcs_with_gas_estimation(
        &self,
        txn: &SignedTransaction,
        estimate_max_gas_amount: bool,
        estimate_max_gas_unit_price: bool,
    ) -> NodeResult<Response<TransactionOnChainData>>;

    /// Submit a signed transaction (BCS).
    async fn submit_bcs(&self, txn: &SignedTransaction) -> NodeResult<Response<()>>;

    /// Node health check; errors if the node is more than `seconds` behind.
    async fn health_check(&self, seconds: u64) -> NodeResult<()>;
}

/// Production [`NodeClient`] that delegates to the real REST client.
#[derive(Clone, Debug)]
pub struct RestNodeClient {
    inner: Arc<Client>,
}

impl RestNodeClient {
    pub fn new(client: Client) -> Self {
        Self {
            inner: Arc::new(client),
        }
    }
}

#[async_trait::async_trait]
impl NodeClient for RestNodeClient {
    async fn get_ledger_information(&self) -> NodeResult<Response<State>> {
        self.inner.get_ledger_information().await
    }

    async fn get_account(&self, address: AccountAddress) -> NodeResult<Response<Account>> {
        self.inner.get_account(address).await
    }

    async fn get_account_resource_at_version_bytes(
        &self,
        address: AccountAddress,
        resource_type: &str,
        version: u64,
    ) -> NodeResult<Response<Vec<u8>>> {
        self.inner
            .get_account_resource_at_version_bytes(address, resource_type, version)
            .await
    }

    async fn get_account_resource_bytes(
        &self,
        address: AccountAddress,
        resource_type: &str,
    ) -> NodeResult<Response<Vec<u8>>> {
        self.inner
            .get_account_resource_bytes(address, resource_type)
            .await
    }

    async fn view_bcs_u64s(
        &self,
        request: &ViewFunction,
        version: Option<u64>,
    ) -> NodeResult<Response<Vec<u64>>> {
        self.inner.view_bcs::<Vec<u64>>(request, version).await
    }

    async fn view_json(
        &self,
        request: &ViewRequest,
        version: Option<u64>,
    ) -> NodeResult<Response<Vec<serde_json::Value>>> {
        self.inner.view(request, version).await
    }

    async fn get_block_by_height_bcs(
        &self,
        height: u64,
        with_transactions: bool,
    ) -> NodeResult<Response<BcsBlock>> {
        self.inner
            .get_block_by_height_bcs(height, with_transactions)
            .await
    }

    async fn get_full_block_by_height_bcs(
        &self,
        height: u64,
        page_size: u16,
    ) -> NodeResult<Response<BcsBlock>> {
        self.inner
            .get_full_block_by_height_bcs(height, page_size)
            .await
    }

    async fn estimate_gas_price(&self) -> NodeResult<Response<GasEstimation>> {
        self.inner.estimate_gas_price().await
    }

    async fn simulate_bcs_with_gas_estimation(
        &self,
        txn: &SignedTransaction,
        estimate_max_gas_amount: bool,
        estimate_max_gas_unit_price: bool,
    ) -> NodeResult<Response<TransactionOnChainData>> {
        self.inner
            .simulate_bcs_with_gas_estimation(
                txn,
                estimate_max_gas_amount,
                estimate_max_gas_unit_price,
            )
            .await
    }

    async fn submit_bcs(&self, txn: &SignedTransaction) -> NodeResult<Response<()>> {
        self.inner.submit_bcs(txn).await
    }

    async fn health_check(&self, seconds: u64) -> NodeResult<()> {
        self.inner.health_check(seconds).await
    }
}

/// Typed resource fetch at a version, mirroring
/// `Client::get_account_resource_at_version_bcs::<T>` (same error behavior).
pub async fn get_account_resource_at_version_bcs<T: DeserializeOwned>(
    client: &dyn NodeClient,
    address: AccountAddress,
    resource_type: &str,
    version: u64,
) -> NodeResult<Response<T>> {
    let response = client
        .get_account_resource_at_version_bytes(address, resource_type, version)
        .await?;
    Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
}

/// Typed resource fetch at the latest version, mirroring
/// `Client::get_account_resource_bcs::<T>` (same error behavior).
pub async fn get_account_resource_bcs<T: DeserializeOwned>(
    client: &dyn NodeClient,
    address: AccountAddress,
    resource_type: &str,
) -> NodeResult<Response<T>> {
    let response = client
        .get_account_resource_bytes(address, resource_type)
        .await?;
    Ok(response.and_then(|bytes| bcs::from_bytes(&bytes))?)
}
