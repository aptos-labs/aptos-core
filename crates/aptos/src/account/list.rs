// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to list resources owned by an address
//!
//! TODO: Examples
//!

use crate::{
    common::{types::NodeOptions, utils::to_common_result},
    CliResult, Error,
};
use aptos_types::account_address::AccountAddress;
use clap::Parser;
use reqwest;

/// Command to list resources owned by an address
///
#[derive(Debug, Parser)]
pub struct ListResources {
    #[clap(flatten)]
    node: NodeOptions,

    /// Address of account you want to list resources for
    account: AccountAddress,
}

impl ListResources {
    async fn get_resources(self) -> Result<Vec<serde_json::Value>, reqwest::Error> {
        reqwest::get(format!(
            "{}/accounts/{}/resources",
            self.node.url, self.account
        ))
        .await?
        .json()
        .await
    }

    // TODO: Format this in a reasonable way while providing all information
    // add options like --tokens --nfts etc
    pub async fn execute(self) -> CliResult {
        let result = self
            .get_resources()
            .await
            .map_err(|err| Error::UnexpectedError(err.to_string()));
        to_common_result(result)
    }
}
