// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{
        CliCommand, CliError, CliTypedResult, EntryFunctionArguments, MultisigAccount,
        TransactionOptions, TransactionSummary,
    },
    utils::get_view_json_option_vec_ref,
};
use aptos_cached_packages::aptos_stdlib;
use aptos_rest_client::{
    aptos_api_types::{HexEncodedBytes, ViewRequest, WriteResource, WriteSetChange},
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
use serde_json::json;
use sha2::Digest;
use sha3::Sha3_256;

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
    /// Pass this flag if only storing transaction hash on-chain. Else full payload is stored
    #[clap(long)]
    pub(crate) hash_only: bool,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateTransaction {
    fn command_name(&self) -> &'static str {
        "CreateTransactionMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let multisig_transaction_payload_bytes =
            to_bytes::<MultisigTransactionPayload>(&self.entry_function_args.try_into()?)?;
        let transaction_payload = if self.hash_only {
            aptos_stdlib::multisig_account_create_transaction_with_hash(
                self.multisig_account.multisig_address,
                Sha3_256::digest(&multisig_transaction_payload_bytes).to_vec(),
            )
        } else {
            aptos_stdlib::multisig_account_create_transaction(
                self.multisig_account.multisig_address,
                multisig_transaction_payload_bytes,
            )
        };
        self.txn_options
            .submit_transaction(transaction_payload)
            .await
            .map(|inner| inner.into())
    }
}

/// Check entry function against on-chain transaction proposal payload.
#[derive(Debug, Parser)]
pub struct CheckTransaction {
    #[clap(flatten)]
    pub(crate) multisig_account: MultisigAccount,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
    /// Sequence number of multisig transaction to check
    #[clap(long)]
    pub(crate) sequence_number: u64,
}

#[async_trait]
impl CliCommand<serde_json::Value> for CheckTransaction {
    fn command_name(&self) -> &'static str {
        "CheckTransactionMultisig"
    }

    async fn execute(self) -> CliTypedResult<serde_json::Value> {
        // Get multisig transaction via view function.
        let multisig_transaction = &self
            .txn_options
            .view(ViewRequest {
                function: "0x1::multisig_account::get_transaction".parse()?,
                type_arguments: vec![],
                arguments: vec![
                    serde_json::Value::String(String::from(
                        &self.multisig_account.multisig_address,
                    )),
                    serde_json::Value::String(self.sequence_number.to_string()),
                ],
            })
            .await?[0];
        // Get reference to inner payload option from multisig transaction.
        let multisig_payload_option_ref =
            get_view_json_option_vec_ref(&multisig_transaction["payload"]);
        // Get expected multisig transaction payload bytes from provided entry function.
        let expected_multisig_transaction_payload_bytes =
            to_bytes::<MultisigTransactionPayload>(&self.entry_function_args.try_into()?)?;
        // If full payload stored on-chain, get expected bytes and reference to actual hex option:
        let (expected_bytes, actual_value_hex_option_ref) =
            if !multisig_payload_option_ref.is_empty() {
                (
                    expected_multisig_transaction_payload_bytes,
                    multisig_payload_option_ref,
                )
            // If only payload hash on-chain, get different compare values:
            } else {
                (
                    Sha3_256::digest(&expected_multisig_transaction_payload_bytes).to_vec(),
                    get_view_json_option_vec_ref(&multisig_transaction["payload_hash"]),
                )
            };
        // If expected bytes matches actual hex from view function:
        if expected_bytes.eq(&actual_value_hex_option_ref[0]
            .as_str()
            .unwrap()
            .parse::<HexEncodedBytes>()?
            .inner())
        {
            // Return success message.
            Ok(json!({
                "Status": "Transaction match",
                "Multisig transaction": multisig_transaction
            }))
        } else {
            // If a mismatch between expected bytes and actual hex, error out.
            Err(CliError::UnexpectedError("Payload mismatch".to_string()))
        }
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
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
}

#[async_trait]
impl CliCommand<TransactionSummary> for Execute {
    fn command_name(&self) -> &'static str {
        "ExecuteMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        self.txn_options
            .submit_transaction(TransactionPayload::Multisig(Multisig {
                multisig_address: self.multisig_account.multisig_address,
                transaction_payload: self.entry_function_args.try_into()?,
            }))
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
