// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliTypedResult, TransactionOptions};
use aptos_rest_client::{
    aptos_api_types::{WriteResource, WriteSetChange},
    Transaction,
};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;
use serde::Serialize;

/// Command to create a resource account
///
#[derive(Debug, Parser)]
pub struct CreateResourceAccount {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,

    /// Resource account seed.
    #[clap(long)]
    pub(crate) seed: String,

    /// Optional Resource Account authentication key.
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) authentication_key: Option<AccountAddress>,
}

/// A shortened create resource account output
#[derive(Clone, Debug, Default, Serialize)]
pub struct CreateResourceAccountSummary {
    pub gas_used: Option<u64>,
    pub sender: Option<AccountAddress>,
    pub resource_account: Option<AccountAddress>,
    pub hash: Option<String>,
    pub success: bool,
    pub version: Option<u64>,
    pub vm_status: String,
}

impl From<Transaction> for CreateResourceAccountSummary {
    fn from(transaction: Transaction) -> Self {
        let mut summary = CreateResourceAccountSummary {
            success: transaction.success(),
            version: transaction.version(),
            vm_status: transaction.vm_status(),
            ..Default::default()
        };

        if let Transaction::UserTransaction(txn) = transaction {
            summary.sender = Some(*txn.request.sender.inner());
            summary.gas_used = Some(txn.info.gas_used.0);
            summary.version = Some(txn.info.version.0);
            summary.hash = Some(txn.info.hash.to_string());
            summary.resource_account = txn.info.changes.iter().find_map(|change| match change {
                WriteSetChange::WriteResource(WriteResource { address, data, .. }) => {
                    if data.typ.name.as_str() == "Account"
                        && *address.inner().to_hex() != *txn.request.sender.inner().to_hex()
                    {
                        Some(*address.inner())
                    } else {
                        None
                    }
                }
                _ => None,
            });
        }

        summary
    }
}

#[async_trait]
impl CliCommand<CreateResourceAccountSummary> for CreateResourceAccount {
    fn command_name(&self) -> &'static str {
        "Create Resource Account"
    }

    async fn execute(self) -> CliTypedResult<CreateResourceAccountSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::encode_create_resource_account(
                &self.seed,
                self.authentication_key,
            ))
            .await
            .map(CreateResourceAccountSummary::from)
    }
}
