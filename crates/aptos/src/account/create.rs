// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliTypedResult, TransactionOptions};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

// TODO(Gas): double check if this is correct
pub const DEFAULT_FUNDED_COINS: u64 = 10_000;

/// Command to create a new account on-chain
///
#[derive(Debug, Parser)]
pub struct CreateAccount {
    /// Address of the new account
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<String> for CreateAccount {
    fn command_name(&self) -> &'static str {
        "CreateAccount"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let address = self.account;
        self.txn_options
            .submit_transaction(aptos_stdlib::account_create_account(address))
            .await
            .map(|_| format!("Account Created at {}", address))
    }
}
