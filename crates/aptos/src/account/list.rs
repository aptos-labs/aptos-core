// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to list resources owned by an address
//!
//! TODO: Examples
//!

use crate::{
    common::{types::NodeOptions, utils::to_common_result},
    CliResult, Error as CommonError,
};
use anyhow::Error;
use aptos_rest_client::{types::Resource, Client};
use aptos_types::account_address::AccountAddress;
use clap::Parser;

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
    async fn get_resources(self) -> Result<Vec<serde_json::Value>, Error> {
        let client = Client::new(self.node.url);
        let response: Vec<Resource> = client
            .get_account_resources(self.account)
            .await?
            .into_inner();
        Ok(response
            .iter()
            .map(|json| json.data.clone())
            .collect::<Vec<serde_json::Value>>())
    }

    // TODO: Format this in a reasonable way while providing all information
    // add options like --tokens --nfts etc
    pub async fn execute(self) -> CliResult {
        let result = self
            .get_resources()
            .await
            .map_err(|err| CommonError::UnexpectedError(err.to_string()));
        to_common_result(result)
    }
}
