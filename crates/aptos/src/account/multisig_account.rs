// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliTypedResult, EntryFunctionArguments, MultisigAccount, TransactionOptions,
    TransactionSummary,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_rest_client::{
    aptos_api_types::{WriteResource, WriteSetChange},
    Transaction,
};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Multisig, MultisigTransactionPayload, TransactionPayload},
};
use async_trait::async_trait;
use bcs::to_bytes;
use clap::Parser;
use serde::Serialize;

/// Create a new multisig account (v2) on-chain.
///
/// This will create a new multisig account and make the sender one of the owners.
#[derive(Debug, Parser)]
pub struct Create {
    /// Addresses of additional owners for the new multisig, beside the transaction sender.
    #[clap(long, multiple_values = true, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) additional_owners: Vec<AccountAddress>,
    /// The number of signatures (approvals or rejections) required to execute or remove a proposed
    /// transaction.
    #[clap(long)]
    pub(crate) num_signatures_required: u64,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

/// A shortened create multisig account output
#[derive(Clone, Debug, Serialize)]
pub struct CreateSummary {
    #[serde(flatten)]
    pub multisig_account: Option<MultisigAccount>,
    #[serde(flatten)]
    pub transaction_summary: TransactionSummary,
}

impl From<Transaction> for CreateSummary {
    fn from(transaction: Transaction) -> Self {
        let transaction_summary = TransactionSummary::from(&transaction);

        let mut summary = CreateSummary {
            transaction_summary,
            multisig_account: None,
        };

        if let Transaction::UserTransaction(txn) = transaction {
            summary.multisig_account = txn.info.changes.iter().find_map(|change| match change {
                WriteSetChange::WriteResource(WriteResource { address, data, .. }) => {
                    if data.typ.name.as_str() == "Account"
                        && *address.inner().to_hex() != *txn.request.sender.inner().to_hex()
                    {
                        Some(MultisigAccount {
                            multisig_address: *address.inner(),
                        })
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
impl CliCommand<CreateSummary> for Create {
    fn command_name(&self) -> &'static str {
        "CreateMultisig"
    }

    async fn execute(self) -> CliTypedResult<CreateSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::multisig_account_create_with_owners(
                self.additional_owners,
                self.num_signatures_required,
                // TODO: Support passing in custom metadata.
                vec![],
                vec![],
            ))
            .await
            .map(CreateSummary::from)
    }
}

/// Propose a new multisig transaction.
///
/// As one of the owners of the multisig, propose a new transaction. This also implicitly approves
/// the created transaction so it has one approval initially. In order for the transaction to be
/// executed, it needs as many approvals as the number of signatures required.
#[derive(Debug, Parser)]
pub struct CreateTransaction {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateTransaction {
    fn command_name(&self) -> &'static str {
        "CreateTransactionMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let payload = MultisigTransactionPayload::EntryFunction(
            self.entry_function_args.create_entry_function_payload()?,
        );
        self.txn_options
            .submit_transaction(aptos_stdlib::multisig_account_create_transaction(
                self.multisig_account.multisig_address,
                to_bytes(&payload)?,
            ))
            .await
            .map(|inner| inner.into())
    }
}

/// Approve a multisig transaction.
///
/// As one of the owners of the multisig, approve a transaction proposed for the multisig.
/// With enough approvals (as many as the number of signatures required), the transaction can be
/// executed (See Execute).
#[derive(Debug, Parser)]
pub struct Approve {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    /// The sequence number of the multisig transaction to approve. The sequence number increments
    /// for every new multisig transaction.
    #[clap(long)]
    pub(crate) sequence_number: u64,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for Approve {
    fn command_name(&self) -> &'static str {
        "ApproveMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::multisig_account_approve_transaction(
                self.multisig_account.multisig_address,
                self.sequence_number,
            ))
            .await
            .map(|inner| inner.into())
    }
}

/// Reject a multisig transaction.
///
/// As one of the owners of the multisig, reject a transaction proposed for the multisig.
/// With enough rejections (as many as the number of signatures required), the transaction can be
/// completely removed (See ExecuteReject).
#[derive(Debug, Parser)]
pub struct Reject {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    /// The sequence number of the multisig transaction to reject. The sequence number increments
    /// for every new multisig transaction.
    #[clap(long)]
    pub(crate) sequence_number: u64,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for Reject {
    fn command_name(&self) -> &'static str {
        "RejectMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::multisig_account_reject_transaction(
                self.multisig_account.multisig_address,
                self.sequence_number,
            ))
            .await
            .map(|inner| inner.into())
    }
}

/// Execute a proposed multisig transaction.
///
/// The transaction to be executed needs to have as many approvals as the number of signatures
/// required.
#[derive(Debug, Parser)]
pub struct Execute {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for Execute {
    fn command_name(&self) -> &'static str {
        "ExecuteMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let payload = TransactionPayload::Multisig(Multisig {
            multisig_address: self.multisig_account.multisig_address,
            // TODO: Support passing an explicit payload
            transaction_payload: None,
        });
        self.txn_options
            .submit_transaction(payload)
            .await
            .map(|inner| inner.into())
    }
}

/// Remove a proposed multisig transaction.
///
/// The transaction to be removed needs to have as many rejections as the number of signatures
/// required.
#[derive(Debug, Parser)]
pub struct ExecuteReject {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for ExecuteReject {
    fn command_name(&self) -> &'static str {
        "ExecuteRejectMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::multisig_account_execute_rejected_transaction(
                self.multisig_account.multisig_address,
            ))
            .await
            .map(|inner| inner.into())
    }
}
