// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliConfig, CliError, CliTypedResult, ProfileOptions, RestOptions,
};
use aptos_rest_client::{types::Resource, Client};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};

/// Command to list resources owned by an address
///
#[derive(Debug, Parser)]
pub struct ListResources {
    #[clap(flatten)]
    verbosity: Verbosity<WarnLevel>,
    #[clap(flatten)]
    rest_options: RestOptions,

    #[clap(flatten)]
    profile_options: ProfileOptions,

    /// Address of account you want to list resources for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    account: Option<AccountAddress>,
}

#[async_trait]
impl CliCommand<Vec<serde_json::Value>> for ListResources {
    fn command_name(&self) -> &'static str {
        "ListResources"
    }

    fn verbosity(&self) -> &Verbosity<WarnLevel> {
        &self.verbosity
    }

    // TODO: Format this in a reasonable way while providing all information
    // add options like --tokens --nfts etc
    async fn execute(self) -> CliTypedResult<Vec<serde_json::Value>> {
        let account = if let Some(account) = self.account {
            account
        } else if let Some(Some(account)) =
            CliConfig::load_profile(&self.profile_options.profile)?.map(|p| p.account)
        {
            account
        } else {
            return Err(CliError::CommandArgumentError(
                "Please provide an account using --account or run aptos init".to_string(),
            ));
        };

        let client = Client::new(self.rest_options.url(&self.profile_options.profile)?);
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
