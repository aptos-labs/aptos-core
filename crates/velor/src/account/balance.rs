// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliConfig, CliError, CliTypedResult, ConfigSearchMode, ProfileOptions, RestOptions,
};
use velor_api_types::ViewFunction;
use velor_types::{account_address::AccountAddress, VelorCoinType, CoinType};
use async_trait::async_trait;
use clap::Parser;
use move_core_types::{
    ident_str,
    language_storage::{ModuleId, StructTag, TypeTag},
    parser::parse_type_tag,
};
use serde::Serialize;

/// Show the account's balance of different coins
///
/// TODO: Fungible assets
#[derive(Debug, Parser)]
pub struct Balance {
    /// Address of the account you want to list resources/modules/balance for
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) account: Option<AccountAddress>,

    /// Coin type to lookup.  Defaults to 0x1::velor_coin::VelorCoin
    #[clap(long)]
    pub(crate) coin_type: Option<String>,

    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[derive(Debug, Clone, Serialize)]
pub struct AccountBalance {
    asset_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    coin_type: Option<String>,
    balance: u64,
}

#[async_trait]
impl CliCommand<Vec<AccountBalance>> for Balance {
    fn command_name(&self) -> &'static str {
        "Balance"
    }

    async fn execute(self) -> CliTypedResult<Vec<AccountBalance>> {
        let account = if let Some(account) = self.account {
            account
        } else if let Some(Some(account)) = CliConfig::load_profile(
            self.profile_options.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| p.account)
        {
            account
        } else {
            return Err(CliError::CommandArgumentError(
                "Please provide an account using --account or run velor init".to_string(),
            ));
        };
        if let Some(ref coin) = self.coin_type {
            if let Ok(addr) = AccountAddress::from_hex_literal(coin) {
                self.fa_balance(account, addr).await
            } else {
                self.coin_balance(account).await
            }
        } else {
            self.coin_balance(account).await
        }
    }
}

impl Balance {
    async fn coin_balance(self, account: AccountAddress) -> CliTypedResult<Vec<AccountBalance>> {
        let coin_type = if let Some(coin) = self.coin_type {
            parse_type_tag(&coin).map_err(|err| {
                CliError::CommandArgumentError(format!("Invalid coin type '{}': {:#?}", coin, err))
            })?
        } else {
            // If nothing is given, use the default APT
            VelorCoinType::type_tag()
        };

        let client = self.rest_options.client(&self.profile_options)?;
        let response = client
            .view_bcs_with_json_response(
                &ViewFunction {
                    module: ModuleId::new(AccountAddress::ONE, ident_str!("coin").to_owned()),
                    function: ident_str!("balance").to_owned(),
                    ty_args: vec![coin_type.clone()],
                    args: vec![account.to_vec()],
                },
                None,
            )
            .await?;

        let balance = response.inner()[0]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        Ok(vec![AccountBalance {
            asset_type: "coin".to_string(),
            coin_type: Some(coin_type.to_canonical_string()),
            balance,
        }])
    }

    async fn fa_balance(
        self,
        account: AccountAddress,
        fa_addr: AccountAddress,
    ) -> CliTypedResult<Vec<AccountBalance>> {
        let client = self.rest_options.client(&self.profile_options)?;
        let response = client
            .view_bcs_with_json_response(
                &ViewFunction {
                    module: ModuleId::new(
                        AccountAddress::ONE,
                        ident_str!("primary_fungible_store").to_owned(),
                    ),
                    function: ident_str!("balance").to_owned(),
                    ty_args: vec![TypeTag::Struct(Box::new(StructTag {
                        address: AccountAddress::ONE,
                        module: ident_str!("fungible_asset").into(),
                        name: ident_str!("Metadata").into(),
                        type_args: vec![],
                    }))],
                    args: vec![account.to_vec(), fa_addr.to_vec()],
                },
                None,
            )
            .await?;

        let balance = response.inner()[0]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();

        Ok(vec![AccountBalance {
            asset_type: "fa".to_string(),
            coin_type: Some(fa_addr.to_string()),
            balance,
        }])
    }
}
