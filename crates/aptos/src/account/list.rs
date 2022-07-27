// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, RestOptions};
use aptos_cli_base::config::{CliConfig, ProfileOptions};
use aptos_cli_base::types::{CliError, CliTypedResult};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::{ArgEnum, Parser};
use serde_json::json;
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

#[derive(ArgEnum, Clone, Copy, Debug)]
pub enum ListQuery {
    Balance,
    Modules,
    Resources,
}

impl Display for ListQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            ListQuery::Balance => "balance",
            ListQuery::Modules => "modules",
            ListQuery::Resources => "resources",
        };
        write!(f, "{}", str)
    }
}

impl FromStr for ListQuery {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "balance" => Ok(ListQuery::Balance),
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
    pub(crate) rest_options: RestOptions,

    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,

    /// Address of account you want to list resources/modules for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) account: Option<AccountAddress>,

    /// Type of items to list: resources, modules. (Defaults to 'resources').
    /// TODO: add options like --tokens --nfts etc
    #[clap(long, default_value_t = ListQuery::Resources)]
    pub(crate) query: ListQuery,
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

        let client = self.rest_options.client(&self.profile_options.profile)?;
        let response = match self.query {
            ListQuery::Balance => vec![
                client
                    .get_account_resource(
                        account,
                        "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>",
                    )
                    .await
                    .map_err(|err| CliError::ApiError(err.to_string()))?
                    .into_inner()
                    .unwrap()
                    .data,
            ],
            ListQuery::Modules => client
                .get_account_modules(account)
                .await
                .map_err(|err| CliError::ApiError(err.to_string()))?
                .into_inner()
                .iter()
                .cloned()
                .map(|module| module.try_parse_abi().unwrap())
                .map(|module| json!(module))
                .collect::<Vec<serde_json::Value>>(),
            ListQuery::Resources => client
                .get_account_resources(account)
                .await
                .map_err(|err| CliError::ApiError(err.to_string()))?
                .into_inner()
                .iter()
                .map(|json| json.data.clone())
                .collect::<Vec<serde_json::Value>>(),
        };

        Ok(response)
    }
}
