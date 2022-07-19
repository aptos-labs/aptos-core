// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliTypedResult, TransactionOptions};
use aptos_transaction_builder::aptos_stdlib;
use async_trait::async_trait;
use clap::Parser;

/// Command to create a resource account
///
#[derive(Debug, Parser)]
pub struct CreateResourceAccount {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,

    /// Resource account seed.
    #[clap(long)]
    pub(crate) seed: String,

    // /// Resource authentication key.
    // #[clap(long, requires=false)]
    // pub(crate) authentication_key: Vec<u8>,
}

#[async_trait]
impl CliCommand<String> for CreateResourceAccount {
    fn command_name(&self) -> &'static str {
        "Create Resource Account"
    }

    async fn execute(self) -> CliTypedResult<String> {
        self.txn_options
        .submit_transaction(aptos_stdlib::encode_create_resource_account(
            &self.seed,
            None
        ))
        .await
        .map(
            |_| format!("weiwu: {}", self.seed)
        )
    }
}


