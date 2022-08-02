// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auto_validate::AutoValidate, TransactionContext};
use aptos_management::{config::ConfigPath, error::Error, secure_backend::ValidatorBackend};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct SetValidatorOperator {
    #[structopt(flatten)]
    _config: ConfigPath,
    #[structopt(long)]
    _name: String,
    #[structopt(long)]
    _account_address: AccountAddress,
    #[structopt(long, required_unless = "config")]
    _json_server: Option<String>,
    #[structopt(long, required_unless("config"))]
    _chain_id: Option<ChainId>,
    #[structopt(flatten)]
    _validator_backend: ValidatorBackend,
    #[structopt(flatten)]
    _auto_validate: AutoValidate,
}

impl SetValidatorOperator {
    pub async fn execute(self) -> Result<TransactionContext, Error> {
        unimplemented!();
    }
}
