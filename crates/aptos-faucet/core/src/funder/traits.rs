// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::endpoints::AptosTapError;
use aptos_sdk::types::{account_address::AccountAddress, transaction::SignedTransaction};
use async_trait::async_trait;

/// explain
#[async_trait]
pub trait Funder: Sync + Send + 'static {
    /// This function is responsible for doing any Funder-relevant eligibility
    /// checks, such as ensuring the account does not already exists (so, mostly
    /// anything that we can check on chain), and if everything looks good,
    /// creating and funding the account.
    ///
    /// If `check_only` is set, this function will only do the initial checks
    /// without actually submitting any transactions.
    async fn fund(
        &self,
        amount: Option<u64>,
        receiver_address: AccountAddress,
        check_only: bool,
    ) -> Result<Vec<SignedTransaction>, AptosTapError>;

    /// Given a requested amount and any configuration internal to this funder,
    /// determine the amount that can be funded.
    fn get_amount(&self, amount: Option<u64>) -> u64;

    /// This should return whether the Funder is healthy and able to accept
    /// requests. With this a Funder can indicate some issue that will get
    /// exposed at the `/` (the healthcheck endpoint), e.g. that that it
    /// doesn't have enough funds in the case of a TransferFunder.
    async fn is_healthy(&self) -> FunderHealthMessage {
        FunderHealthMessage {
            can_process_requests: true,
            message: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunderHealthMessage {
    /// If the Funder is able to handle more requests, it should return true.
    pub can_process_requests: bool,
    pub message: Option<String>,
}
