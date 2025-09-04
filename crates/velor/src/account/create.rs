// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliTypedResult, TransactionOptions, TransactionSummary};
use velor_cached_packages::velor_stdlib;
use velor_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

// 1 APT
pub const DEFAULT_FUNDED_COINS: u64 = 100_000_000;

/// Create a new account on-chain
///
/// An account can be created by transferring coins, or by making an explicit
/// call to create an account.  This will create an account with no coins, and
/// any coins will have to transferred afterwards.
#[derive(Debug, Parser)]
pub struct CreateAccount {
    /// Address of the new account
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) account: AccountAddress,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateAccount {
    fn command_name(&self) -> &'static str {
        "CreateAccount"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let address = self.account;
        self.txn_options
            .submit_transaction(velor_stdlib::velor_account_create_account(address))
            .await
            .map(TransactionSummary::from)
    }
}
