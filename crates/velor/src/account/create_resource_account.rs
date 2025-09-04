// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::derive_resource_account::ResourceAccountSeed,
    common::types::{CliCommand, CliTypedResult, TransactionOptions, TransactionSummary},
};
use velor_cached_packages::velor_stdlib::resource_account_create_resource_account;
use velor_rest_client::{
    velor_api_types::{WriteResource, WriteSetChange},
    Transaction,
};
use velor_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use async_trait::async_trait;
use clap::Parser;
use serde::Serialize;
use std::str::FromStr;

/// Create a resource account on-chain
///
/// This will create a resource account which can be used as an autonomous account
/// not controlled directly by one account.
#[derive(Debug, Parser)]
pub struct CreateResourceAccount {
    /// Optional Resource Account authentication key.
    #[clap(long, value_parser = AuthenticationKey::from_str)]
    pub(crate) authentication_key: Option<AuthenticationKey>,

    #[clap(flatten)]
    pub(crate) seed_args: ResourceAccountSeed,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

/// A shortened create resource account output
#[derive(Clone, Debug, Serialize)]
pub struct CreateResourceAccountSummary {
    pub resource_account: Option<AccountAddress>,
    #[serde(flatten)]
    pub transaction_summary: TransactionSummary,
}

impl From<Transaction> for CreateResourceAccountSummary {
    fn from(transaction: Transaction) -> Self {
        let transaction_summary = TransactionSummary::from(&transaction);

        let mut summary = CreateResourceAccountSummary {
            transaction_summary,
            resource_account: None,
        };

        if let Transaction::UserTransaction(txn) = transaction {
            summary.resource_account = txn.info.changes.iter().find_map(|change| match change {
                WriteSetChange::WriteResource(WriteResource { address, data, .. }) => {
                    if data.typ.name.as_str() == "Account"
                        && *address.inner().to_hex() != *txn.request.sender.inner().to_hex()
                    {
                        Some(*address.inner())
                    } else {
                        None
                    }
                },
                _ => None,
            });
        }

        summary
    }
}

#[async_trait]
impl CliCommand<CreateResourceAccountSummary> for CreateResourceAccount {
    fn command_name(&self) -> &'static str {
        "CreateResourceAccount"
    }

    async fn execute(self) -> CliTypedResult<CreateResourceAccountSummary> {
        let authentication_key: Vec<u8> = if let Some(key) = self.authentication_key {
            bcs::to_bytes(&key)?
        } else {
            vec![]
        };
        self.txn_options
            .submit_transaction(resource_account_create_resource_account(
                self.seed_args.seed()?,
                authentication_key,
            ))
            .await
            .map(CreateResourceAccountSummary::from)
    }
}
