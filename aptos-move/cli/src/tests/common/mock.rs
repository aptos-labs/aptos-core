// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Mock definitions for network-dependent test scenarios.
//!
//! Provides [`MockAptosCtx`] — a mockall-generated implementation of
//! [`AptosContext`] — so we can test commands like `publish`, `run`, and
//! `view` without a live network.

use crate::AptosContext;
use aptos_cli_common::{CliTypedResult, TransactionOptions, TransactionSummary};
use aptos_rest_client::aptos_api_types::ViewFunction;
use aptos_types::transaction::TransactionPayload;
use async_trait::async_trait;
use mockall::mock;

mock! {
    pub AptosCtx {}

    #[async_trait]
    impl AptosContext for AptosCtx {
        async fn submit_transaction(
            &self,
            options: &TransactionOptions,
            payload: TransactionPayload,
        ) -> CliTypedResult<TransactionSummary>;

        async fn view(
            &self,
            options: &TransactionOptions,
            request: ViewFunction,
        ) -> CliTypedResult<Vec<serde_json::Value>>;
    }
}
