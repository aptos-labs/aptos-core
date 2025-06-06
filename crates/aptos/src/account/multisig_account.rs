// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{
        CliCommand, CliError, CliTypedResult, EntryFunctionArguments, MultisigAccount,
        MultisigAccountWithSequenceNumber, TransactionOptions, TransactionSummary,
    },
    utils::view_json_option_str,
};
use aptos_api_types::ViewFunction;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::HashValue;
use aptos_rest_client::{
    aptos_api_types::{HexEncodedBytes, WriteResource, WriteSetChange},
    Transaction,
};
use aptos_types::{
    account_address::AccountAddress,
    transaction::{Multisig, MultisigTransactionPayload, TransactionPayload},
};
use async_trait::async_trait;
use bcs::to_bytes;
use clap::Parser;
use move_core_types::{ident_str, language_storage::ModuleId};
use serde::Serialize;
use serde_json::json;

/// Create a new multisig account (v2) on-chain.
///
/// This will create a new multisig account and make the sender one of the owners.
#[derive(Debug, Parser)]
pub struct Create {
    /// Addresses of additional owners for the new multisig, beside the transaction sender.
    #[clap(long, num_args = 0.., value_parser = crate::common::types::load_account_arg)]
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
    pub(crate) store_hash_only: bool,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateTransaction {
    fn command_name(&self) -> &'static str {
        "CreateTransactionMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let multisig_transaction_payload_bytes =
            to_bytes::<MultisigTransactionPayload>(&self.entry_function_args.try_into()?)?;
        let transaction_payload = if self.store_hash_only {
            aptos_stdlib::multisig_account_create_transaction_with_hash(
                self.multisig_account.multisig_address,
                HashValue::sha3_256_of(&multisig_transaction_payload_bytes).to_vec(),
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

/// Verify entry function matches on-chain transaction proposal.
#[derive(Debug, Parser)]
pub struct VerifyProposal {
    #[clap(flatten)]
    pub(crate) multisig_account_with_sequence_number: MultisigAccountWithSequenceNumber,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
}

#[async_trait]
impl CliCommand<serde_json::Value> for VerifyProposal {
    fn command_name(&self) -> &'static str {
        "VerifyProposalMultisig"
    }

    async fn execute(self) -> CliTypedResult<serde_json::Value> {
        // Get multisig transaction via view function.
        let multisig_transaction = &self
            .txn_options
            .view(ViewFunction {
                module: ModuleId::new(
                    AccountAddress::ONE,
                    ident_str!("multisig_account").to_owned(),
                ),
                function: ident_str!("get_transaction").to_owned(),
                ty_args: vec![],
                args: vec![
                    bcs::to_bytes(
                        &self
                            .multisig_account_with_sequence_number
                            .multisig_account
                            .multisig_address,
                    )
                    .unwrap(),
                    bcs::to_bytes(&self.multisig_account_with_sequence_number.sequence_number)
                        .unwrap(),
                ],
            })
            .await?[0];
        // Get expected multisig transaction payload hash hex from provided entry function.
        let expected_payload_hash = HashValue::sha3_256_of(
            &to_bytes::<MultisigTransactionPayload>(&self.entry_function_args.try_into()?)?,
        )
        .to_hex_literal();
        // Get on-chain payload hash. If full payload provided on-chain:
        let actual_payload_hash =
            if let Some(actual_payload) = view_json_option_str(&multisig_transaction["payload"])? {
                // Actual payload hash is the hash of the on-chain payload.
                HashValue::sha3_256_of(actual_payload.parse::<HexEncodedBytes>()?.inner())
                    .to_hex_literal()
            // If full payload not provided, get payload hash directly from transaction proposal:
            } else {
                view_json_option_str(&multisig_transaction["payload_hash"])?.ok_or_else(|| {
                    CliError::UnexpectedError(
                        "Neither payload nor payload hash provided on-chain".to_string(),
                    )
                })?
            };
        // Get verification result based on if expected and actual payload hashes match.
        if expected_payload_hash.eq(&actual_payload_hash) {
            Ok(json!({
                "Status": "Transaction match",
                "Multisig transaction": multisig_transaction
            }))
        } else {
            Err(CliError::UnexpectedError(format!(
                "Transaction mismatch: The transaction you provided has a payload hash of \
                {expected_payload_hash}, but the on-chain transaction proposal you specified has \
                a payload hash of {actual_payload_hash}. For more info, see \
                https://aptos.dev/move/move-on-aptos/cli#multisig-governance"
            )))
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
    pub(crate) multisig_account_with_sequence_number: MultisigAccountWithSequenceNumber,
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
                self.multisig_account_with_sequence_number
                    .multisig_account
                    .multisig_address,
                self.multisig_account_with_sequence_number.sequence_number,
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
    pub(crate) multisig_account_with_sequence_number: MultisigAccountWithSequenceNumber,
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
                self.multisig_account_with_sequence_number
                    .multisig_account
                    .multisig_address,
                self.multisig_account_with_sequence_number.sequence_number,
            ))
            .await
            .map(|inner| inner.into())
    }
}

/// Execute a proposed multisig transaction that has a full payload stored on-chain.
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
        // TODO[Orderless]: Change this to payload v2 format
        self.txn_options
            .submit_transaction(TransactionPayload::Multisig(Multisig {
                multisig_address: self.multisig_account.multisig_address,
                transaction_payload: None,
            }))
            .await
            .map(|inner| inner.into())
    }
}

/// Execute a proposed multisig transaction that has only a payload hash stored on-chain.
#[derive(Debug, Parser)]
pub struct ExecuteWithPayload {
    #[clap(flatten)]
    pub(crate) execute: Execute,
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
}

#[async_trait]
impl CliCommand<TransactionSummary> for ExecuteWithPayload {
    fn command_name(&self) -> &'static str {
        "ExecuteWithPayloadMultisig"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        self.execute
            .txn_options
            .submit_transaction(TransactionPayload::Multisig(Multisig {
                multisig_address: self.execute.multisig_account.multisig_address,
                transaction_payload: Some(self.entry_function_args.try_into()?),
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
