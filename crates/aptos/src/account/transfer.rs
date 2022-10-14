// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{
    CliCommand, CliTypedResult, TransactionOptions, TransactionOutput, TransactionSummary,
};
use aptos_rest_client::{
    aptos_api_types::{WriteResource, WriteSetChange},
    Transaction,
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use cached_packages::aptos_stdlib;
use clap::Parser;
use serde::Serialize;
use std::collections::BTreeMap;

// TODO: Add ability to transfer non-APT coins
// TODO: Add ability to not create account by default
/// Transfer APT between accounts
///
#[derive(Debug, Parser)]
pub struct TransferCoins {
    /// Address of account to send APT to
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Amount of Octas (10^-8 APT) to transfer
    #[clap(long)]
    pub(crate) amount: u64,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransferSummary> for TransferCoins {
    fn command_name(&self) -> &'static str {
        "TransferCoins"
    }

    async fn execute(self) -> CliTypedResult<TransferSummary> {
        self.txn_options
            .submit_transaction(aptos_stdlib::aptos_account_transfer(
                self.account,
                self.amount,
            ))
            .await
            .map(TransferSummary::from)
    }
}

const SUPPORTED_COINS: [&str; 1] = ["0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"];

/// A shortened transaction output
#[derive(Clone, Debug, Serialize)]
pub struct TransferSummary {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub balance_changes: BTreeMap<AccountAddress, serde_json::Value>,
    #[serde(flatten)]
    pub transaction_summary: TransactionSummary,
}

impl From<TransactionOutput> for TransferSummary {
    fn from(transaction: TransactionOutput) -> Self {
        let transaction_summary = TransactionSummary::from(&transaction);

        let mut summary = TransferSummary {
            balance_changes: Default::default(),
            transaction_summary,
        };

        if let TransactionOutput::Txn(Transaction::UserTransaction(txn)) = transaction {
            let balance_changes = txn
                .info
                .changes
                .into_iter()
                .filter_map(|change| match change {
                    WriteSetChange::WriteResource(WriteResource { address, data, .. }) => {
                        if SUPPORTED_COINS.contains(&data.typ.to_string().as_str()) {
                            Some((
                                *address.inner(),
                                serde_json::to_value(data.data).unwrap_or_default(),
                            ))
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .collect();
            summary.balance_changes = balance_changes
        }

        summary
    }
}
