// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliTypedResult, TransactionOptions, TransactionOutput, TransactionSummary,
};
use aptos_rest_client::{
    aptos_api_types::{WriteResource, WriteSetChange},
    Transaction,
};
use aptos_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use async_trait::async_trait;
use cached_packages::aptos_stdlib::resource_account_create_resource_account;
use clap::Parser;
use serde::Serialize;
use std::str::FromStr;

/// Create a resource account on-chain
///
/// This will create a resource account which can be used as an autonomous account
/// not controlled directly by one account.
#[derive(Debug, Parser)]
pub struct CreateResourceAccount {
    /// Resource account seed
    ///
    /// Seed used in generation of the AccountId of the resource account
    /// The seed will be converted to bytes using `BCS`
    #[clap(long)]
    pub(crate) seed: String,

    /// Optional Resource Account authentication key.
    #[clap(long, parse(try_from_str = AuthenticationKey::from_str))]
    pub(crate) authentication_key: Option<AuthenticationKey>,

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

impl From<TransactionOutput> for CreateResourceAccountSummary {
    fn from(transaction: TransactionOutput) -> Self {
        let transaction_summary = TransactionSummary::from(&transaction);

        let mut summary = CreateResourceAccountSummary {
            transaction_summary,
            resource_account: None,
        };

        if let TransactionOutput::Txn(Transaction::UserTransaction(txn)) = transaction {
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
                bcs::to_bytes(&self.seed)?,
                authentication_key,
            ))
            .await
            .map(CreateResourceAccountSummary::from)
    }
}
