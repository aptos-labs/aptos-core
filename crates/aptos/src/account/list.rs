// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to list resources owned by an address
//!
//! TODO: Examples
//!

use crate::common::types::{
    account_address_from_public_key, CliConfig, CliError, CliTypedResult, ProfileOptions,
    RestOptions,
};
use aptos_crypto::PrivateKey;
use aptos_rest_client::{types::Resource, Client};
use aptos_types::account_address::AccountAddress;
use clap::Parser;

/// Command to list resources owned by an address
///
#[derive(Debug, Parser)]
pub struct ListResources {
    #[clap(flatten)]
    rest_options: RestOptions,

    #[clap(flatten)]
    profile: ProfileOptions,

    /// Address of account you want to list resources for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    account: Option<AccountAddress>,
}

impl ListResources {
    // TODO: Format this in a reasonable way while providing all information
    // add options like --tokens --nfts etc
    pub(crate) async fn execute(self) -> CliTypedResult<Vec<serde_json::Value>> {
        let account = if let Some(account) = self.account {
            account
        } else if let Some(Some(private_key)) =
            CliConfig::load_profile(&self.profile.profile)?.map(|p| p.private_key)
        {
            let public_key = private_key.public_key();
            account_address_from_public_key(&public_key)
        } else {
            return Err(CliError::CommandArgumentError(
                "Please provide an account using --account or run aptos init".to_string(),
            ));
        };

        let client = Client::new(self.rest_options.url(&self.profile.profile)?);
        let response: Vec<Resource> = client
            .get_account_resources(account)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner();
        Ok(response
            .iter()
            .map(|json| json.data.clone())
            .collect::<Vec<serde_json::Value>>())
    }
}
