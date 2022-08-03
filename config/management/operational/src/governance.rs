// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auto_validate::AutoValidate, TransactionContext};
use aptos_management::{config::ConfigPath, error::Error, secure_backend::ValidatorBackend};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct CreateAccount {
    #[structopt(flatten)]
    _config: ConfigPath,
    #[structopt(long)]
    _name: String,
    #[structopt(long)]
    _path_to_key: PathBuf,
    #[structopt(long, required_unless = "config")]
    _json_server: Option<String>,
    #[structopt(long, required_unless("config"))]
    _chain_id: Option<ChainId>,
    #[structopt(flatten)]
    _validator_backend: ValidatorBackend,
    #[structopt(flatten)]
    _auto_validate: AutoValidate,
}

#[derive(Debug, StructOpt)]
pub struct CreateValidator {
    #[structopt(flatten)]
    _input: CreateAccount,
}

impl CreateValidator {
    pub async fn execute(self) -> Result<(TransactionContext, AccountAddress), Error> {
        unimplemented!();
    }
}

#[derive(Debug, StructOpt)]
pub struct CreateValidatorOperator {
    #[structopt(flatten)]
    _input: CreateAccount,
}

impl CreateValidatorOperator {
    pub async fn execute(self) -> Result<(TransactionContext, AccountAddress), Error> {
        unimplemented!();
    }
}

#[derive(Debug, StructOpt)]
struct RootValidatorOperation {
    #[structopt(long, help = "The validator address")]
    _account_address: AccountAddress,
    /// JSON-RPC Endpoint (e.g. http://localhost:8080)
    #[structopt(long, required_unless = "config")]
    _json_server: Option<String>,
    #[structopt(flatten)]
    _validator_config: aptos_management::validator_config::ValidatorConfig,
    #[structopt(flatten)]
    _auto_validate: AutoValidate,
}

#[derive(Debug, StructOpt)]
pub struct AddValidator {
    #[structopt(flatten)]
    _input: RootValidatorOperation,
}

impl AddValidator {
    pub async fn execute(self) -> Result<TransactionContext, Error> {
        unimplemented!();
    }
}

#[derive(Debug, StructOpt)]
pub struct RemoveValidator {
    #[structopt(flatten)]
    _input: RootValidatorOperation,
}

impl RemoveValidator {
    pub async fn execute(self) -> Result<TransactionContext, Error> {
        unimplemented!();
    }
}
