// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod common;
mod fake;
mod mint;
mod traits;
mod transfer;

pub use self::{
    common::{ApiConnectionConfig, TransactionSubmissionConfig},
    mint::MintFunderConfig,
};
use self::{fake::FakeFunderConfig, transfer::TransferFunderConfig};
use anyhow::{Context, Result};
pub use fake::FakeFunder;
pub use mint::MintFunder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use traits::Funder;
pub use transfer::TransferFunder;

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
    pub async fn build_funder(&self) -> Result<Arc<dyn Funder>> {
        match self {
            FunderConfig::FakeFunder(_) => Ok(Arc::new(FakeFunder)),
            FunderConfig::MintFunder(config) => Ok(Arc::new(
                config
                    .build_funder()
                    .await
                    .context("Failed to build MintFunder")?,
            )),
            FunderConfig::TransferFunder(config) => Ok(Arc::new(
                config
                    .build_funder()
                    .await
                    .context("Failed to build TransferFunder")?,
            )),
        }
    }
}
