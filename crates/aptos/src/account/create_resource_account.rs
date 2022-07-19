// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliTypedResult, TransactionOptions};
use aptos_rest_client::{Transaction, aptos_api_types::WriteSetChange};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
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
            |tx| {
                let mut res: Option<AccountAddress> = None;
                let self_address = self.txn_options.profile_options.account_address().unwrap();
                if let Transaction::UserTransaction(txn) = tx {
                    res = txn
                    .info
                    .changes
                    .iter()
                    .find_map(|change| match change {
                        WriteSetChange::WriteResource { address, data, .. } => {
                            if data.typ.name.as_str() == "Account" && *address.inner().to_hex() != self_address.to_hex() {
                                Some(
                                    *address.inner()
                                )
                            } else {
                                None
                            }
                        }
                        _ => None,
                    });
                }
                format!("resource account key: 0x{}", res.unwrap().to_hex())
            }
        )
    }
}


