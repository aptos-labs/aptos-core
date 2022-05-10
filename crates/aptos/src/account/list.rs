// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliConfig, CliError, CliTypedResult, ProfileOptions, RestOptions,
};
use aptos_rest_client::Client;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::{ArgEnum, Parser};
use serde_json::json;
use std::str::FromStr;

#[derive(ArgEnum, Clone, Copy, Debug)]
enum ListQuery {
    Modules,
    Resources,
}

impl FromStr for ListQuery {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "modules" => Ok(ListQuery::Modules),
            "resources" => Ok(ListQuery::Resources),
            _ => Err("Invalid query. Valid values are modules, resources"),
        }
    }
}

/// Command to list items owned by an address
///
#[derive(Debug, Parser)]
pub struct ListAccount {
    #[clap(flatten)]
    rest_options: RestOptions,

    #[clap(flatten)]
    profile_options: ProfileOptions,

    /// Address of account you want to list resources/modules for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    account: Option<AccountAddress>,

    /// Type of items to list: resources, modules. (Defaults to 'resources').
    /// TODO: add options like --tokens --nfts etc
    #[clap(long, default_value = "resources")]
    query: ListQuery,
}

#[async_trait]
impl CliCommand<Vec<serde_json::Value>> for ListAccount {
    fn command_name(&self) -> &'static str {
        "ListAccount"
    }

    // TODO: Format this in a reasonable way while providing all information
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
        let map_err_func = |err: anyhow::Error| CliError::ApiError(err.to_string());
        let response = match self.query {
            ListQuery::Modules => client
                .get_account_modules(account)
                .await
                .map_err(map_err_func)?
                .into_inner()
                .iter()
                .cloned()
                .map(|module| module.try_parse_abi().unwrap())
                .map(|module| json!(module))
                .collect::<Vec<serde_json::Value>>(),
            ListQuery::Resources => client
                .get_account_resources(account)
                .await
                .map_err(map_err_func)?
                .into_inner()
                .iter()
                .map(|json| json.data.clone())
                .collect::<Vec<serde_json::Value>>(),
        };

        Ok(response)
    }
}
