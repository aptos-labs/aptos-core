// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod common;
mod fake;
mod mint;
mod transfer;

pub use self::{
    common::{ApiConnectionConfig, TransactionSubmissionConfig},
    mint::MintFunderConfig,
};
use self::{fake::FakeFunderConfig, transfer::TransferFunderConfig};
use crate::endpoints::VelorTapError;
use anyhow::{Context, Result};
use velor_sdk::types::{account_address::AccountAddress, transaction::SignedTransaction};
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
pub use fake::FakeFunder;
pub use mint::MintFunder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use transfer::TransferFunder;

/// explain
#[async_trait]
#[enum_dispatch]
pub trait FunderTrait: Sync + Send + 'static {
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
        // True if a Bypasser let this request bypass the Checkers.
        did_bypass_checkers: bool,
    ) -> Result<Vec<SignedTransaction>, VelorTapError>;

    /// Given a requested amount and any configuration internal to this funder,
    /// determine the amount that can be funded.
    fn get_amount(
        &self,
        amount: Option<u64>,
        // True if a Bypasser let this request bypass the Checkers.
        did_bypass_checkers: bool,
    ) -> u64;

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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum FunderConfig {
    /// This funder does nothing and returns nothing.
    FakeFunder(FakeFunderConfig),

    /// This funder uses the delegation + minting mechanism to fund.
    MintFunder(MintFunderConfig),

    /// This funder creates and funds accounts by using + transferring
    /// coins from a pre-funded account provided in configuration.
    TransferFunder(TransferFunderConfig),
}

impl FunderConfig {
    pub async fn build(self) -> Result<Arc<Funder>> {
        match self {
            FunderConfig::FakeFunder(_) => Ok(Arc::new(Funder::from(FakeFunder))),
            FunderConfig::MintFunder(config) => Ok(Arc::new(Funder::from(
                config
                    .build_funder()
                    .await
                    .context("Failed to build MintFunder")?,
            ))),
            FunderConfig::TransferFunder(config) => Ok(Arc::new(Funder::from(
                config
                    .build_funder()
                    .await
                    .context("Failed to build TransferFunder")?,
            ))),
        }
    }
}

/// This enum has as its variants all possible implementations of FunderTrait.
#[enum_dispatch(FunderTrait)]
pub enum Funder {
    FakeFunder,
    MintFunder,
    TransferFunder,
}

#[derive(Debug, Clone)]
pub struct FunderHealthMessage {
    /// If the Funder is able to handle more requests, it should return true.
    pub can_process_requests: bool,
    pub message: Option<String>,
}
