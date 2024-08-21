// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::error::QuorumStoreError;
use aptos_consensus_types::{
    common::{Author, Payload, PayloadFilter},
    payload_pull_params::PayloadPullParameters,
    utils::PayloadTxnsSize,
};
use aptos_types::validator_txn::ValidatorTransaction;
use aptos_validator_transaction_pool::TransactionFilter;
use core::fmt;
use futures::future::BoxFuture;
use std::{collections::HashSet, time::Duration};

pub mod mixed;
pub mod user;
pub mod validator;

#[async_trait::async_trait]
pub trait PayloadClient: Send + Sync {
    async fn pull_payload(
        &self,
        config: PayloadPullParameters,
        validator_txn_filter: TransactionFilter,
        wait_callback: BoxFuture<'static, ()>,
    ) -> anyhow::Result<(Vec<ValidatorTransaction>, Payload), QuorumStoreError>;
}
