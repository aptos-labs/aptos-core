// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliError, ProfileOptions, RestOptions};
use crate::{CliCommand, CliResult, CliTypedResult};
use aptos_rest_client::aptos_api_types::HashValue;
use aptos_rest_client::Response;
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use move_core_types::account_address::AccountAddress;
use serde::Serialize;
use std::future::Future;
/// Experimental CLI used for debugging the API and making direct API calls
///
/// It is locked with feature `api` and isn't currently up to the standards of the
/// rest of the CLI tool.
use std::str::FromStr;
use std::time::{Duration, SystemTime};

/// Tool for using the REST API directly
///
/// This tool is used to create accounts, get information about the
/// account's resources, and transfer resources between accounts.
#[derive(Debug, Subcommand)]
pub enum ApiTool {
    GetAccount(GetAccount),
    GetAccountTransactions(GetAccountTransactions),
    GetTransaction(GetTransaction),
    GetTransactions(GetTransactions),
}

impl ApiTool {
    #[allow(dead_code)]
    pub async fn execute(self) -> CliResult {
        match self {
            ApiTool::GetAccount(inner) => inner.execute_serialized().await,
            ApiTool::GetTransaction(inner) => inner.execute_serialized().await,
            ApiTool::GetAccountTransactions(inner) => inner.execute_serialized().await,
            ApiTool::GetTransactions(inner) => inner.execute_serialized().await,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum ApiOutputFormat {
    Bcs,
    Json,
}

impl Default for ApiOutputFormat {
    fn default() -> Self {
        ApiOutputFormat::Json
    }
}

impl FromStr for ApiOutputFormat {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "bcs" => Ok(ApiOutputFormat::Bcs),
            "json" => Ok(ApiOutputFormat::Json),
            _ => Err(CliError::CommandArgumentError(
                "Invalid type, must be one of [bcs, json]".to_string(),
            )),
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum DisplayFormat {
    Debug,
    Json,
    Yaml,
}

impl Default for DisplayFormat {
    fn default() -> Self {
        DisplayFormat::Json
    }
}

impl FromStr for DisplayFormat {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "debug" => Ok(DisplayFormat::Debug),
            "json" => Ok(DisplayFormat::Json),
            "yaml" => Ok(DisplayFormat::Yaml),
            _ => Err(CliError::CommandArgumentError(
                "Invalid type, must be one of [debug, json]".to_string(),
            )),
        }
    }
}

#[derive(Debug, Parser)]
pub struct GetAccount {
    /// Address of the new account
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) account: Option<AccountAddress>,

    #[clap(flatten)]
    pub api_options: ApiOptions,
    #[clap(flatten)]
    pub(crate) profile: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<()> for GetAccount {
    fn command_name(&self) -> &'static str {
        "GetAccount"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let client = self.rest_options.client(&self.profile)?;
        let account = if let Some(account) = self.account {
            account
        } else {
            self.profile.account_address()?
        };

        self.api_options
            .call_api(client.get_account_bcs(account), client.get_account(account))
            .await?;

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct GetTransaction {
    #[clap(long, group = "identifier")]
    pub hash: Option<HashValue>,
    #[clap(long, group = "identifier")]
    pub version: Option<u64>,

    #[clap(flatten)]
    pub api_options: ApiOptions,
    #[clap(flatten)]
    pub(crate) profile: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<()> for GetTransaction {
    fn command_name(&self) -> &'static str {
        "GetTransaction"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let client = self.rest_options.client(&self.profile)?;

        match (self.version, self.hash) {
            (Some(version), None) => {
                self.api_options
                    .call_api(
                        client.get_transaction_by_version_bcs(version),
                        client.get_transaction_by_version(version),
                    )
                    .await?;
            }
            (None, Some(hash)) => {
                self.api_options
                    .call_api(
                        client.get_transaction_by_hash_bcs(hash.into()),
                        client.get_transaction_by_hash_bcs(hash.into()),
                    )
                    .await?;
            }
            _ => {
                return Err(CliError::CommandArgumentError(
                    "Must provide either `--version` or `--hash` but not both".to_string(),
                ))
            }
        }

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct GetTransactions {
    #[clap(long)]
    pub start: Option<u64>,
    #[clap(long)]
    pub limit: Option<u16>,

    #[clap(flatten)]
    pub api_options: ApiOptions,
    #[clap(flatten)]
    pub(crate) profile: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<()> for GetTransactions {
    fn command_name(&self) -> &'static str {
        "GetTransactions"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let client = self.rest_options.client(&self.profile)?;

        self.api_options
            .call_api(
                client.get_transactions_bcs(self.start, self.limit),
                client.get_transactions(self.start, self.limit),
            )
            .await?;

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct GetAccountTransactions {
    /// Address of the new account
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) account: Option<AccountAddress>,
    #[clap(long)]
    pub start: Option<u64>,
    #[clap(long)]
    pub limit: Option<u16>,

    #[clap(flatten)]
    pub api_options: ApiOptions,
    #[clap(flatten)]
    pub(crate) profile: ProfileOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
}

#[async_trait]
impl CliCommand<()> for GetAccountTransactions {
    fn command_name(&self) -> &'static str {
        "GetAccountTransactions"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let client = self.rest_options.client(&self.profile)?;
        let account = if let Some(account) = self.account {
            account
        } else {
            self.profile.account_address()?
        };

        self.api_options
            .call_api(
                client.get_account_transactions_bcs(account, self.start, self.limit),
                client.get_account_transactions(
                    account,
                    self.start,
                    self.limit.map(|inner| inner as u64),
                ),
            )
            .await?;

        Ok(())
    }
}

#[derive(Debug, Parser)]
pub struct ApiOptions {
    /// API output
    ///
    /// Supported types: [json, bcs]
    #[clap(long, default_value = "json")]
    pub(crate) api_output_format: ApiOutputFormat,
    /// Display output
    ///
    /// Supported types: [yaml, json, debug]
    #[clap(long, default_value = "json")]
    pub(crate) display_format: DisplayFormat,
}

impl ApiOptions {
    /// Calls an API in both BCS and JSON formats accordingly
    async fn call_api<
        B: std::fmt::Debug + Serialize,
        BFut: Future<Output = Result<Response<B>, aptos_rest_client::error::RestError>>,
        SFut: Future<Output = Result<Response<S>, aptos_rest_client::error::RestError>>,
        S: std::fmt::Debug + Serialize,
    >(
        &self,
        bcs: BFut,
        json: SFut,
    ) -> CliTypedResult<()> {
        match self.api_output_format {
            ApiOutputFormat::Bcs => {
                let start_time = SystemTime::now();
                let response = bcs
                    .await
                    .map_err(|err| CliError::ApiError(err.to_string()))?;
                let duration = start_time.elapsed().unwrap();
                output(self.display_format, response, duration)
            }
            ApiOutputFormat::Json => {
                let start_time = SystemTime::now();
                let response = json
                    .await
                    .map_err(|err| CliError::ApiError(err.to_string()))?;
                let duration = start_time.elapsed().unwrap();
                output(self.display_format, response, duration)
            }
        }

        Ok(())
    }
}

/// Output in the different display formats
fn output<T: std::fmt::Debug + Serialize>(
    display_format: DisplayFormat,
    response: Response<T>,
    duration: Duration,
) {
    match display_format {
        DisplayFormat::Debug => {
            eprintln!("Latency: {}ms", duration.as_millis());
            eprintln!("State: {:?}", response.state());
            println!("{:?}", response.inner());
        }
        DisplayFormat::Json => {
            eprintln!("Latency: {}ms", duration.as_millis());
            eprintln!(
                "State: {}",
                serde_json::to_string_pretty(response.state())
                    .unwrap_or_else(|_| "Failed to serialize state to JSON".to_string())
            );
            println!(
                "{}",
                serde_json::to_string_pretty(response.inner())
                    .unwrap_or_else(|_| "Failed to serialize output to JSON".to_string())
            );
        }
        DisplayFormat::Yaml => {
            eprintln!("Latency: {}ms", duration.as_millis());
            eprintln!(
                "State: {}",
                serde_yaml::to_string(response.state())
                    .unwrap_or_else(|_| "Failed to serialize state to YAML".to_string())
            );
            println!(
                "{}",
                serde_yaml::to_string(response.inner())
                    .unwrap_or_else(|_| "Failed to serialize output to YAML".to_string())
            );
        }
    }
}
