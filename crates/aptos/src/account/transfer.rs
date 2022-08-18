// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliTypedResult, TransactionOptions, TransactionSummary};
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

/// Command to transfer coins between accounts
///
#[derive(Debug, Parser)]
pub struct TransferCoins {
    /// Address of account you want to send coins to
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Amount of coins to transfer
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
            .submit_transaction(aptos_stdlib::aptos_coin_transfer(self.account, self.amount))
            .await
            .map(TransferSummary::from)
    }
}

const SUPPORTED_COINS: [&str; 1] = ["0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>"];

/// A shortened transaction output
#[derive(Clone, Debug, Serialize)]
pub struct TransferSummary {
    pub balance_changes: BTreeMap<AccountAddress, serde_json::Value>,
    #[serde(flatten)]
    pub transaction_summary: TransactionSummary,
}

impl From<Transaction> for TransferSummary {
    fn from(transaction: Transaction) -> Self {
        let transaction_summary = TransactionSummary::from(&transaction);
        let balance_changes = if let Transaction::UserTransaction(txn) = transaction {
            txn.info
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
                .collect()
        } else {
            BTreeMap::new()
        };

        TransferSummary {
            balance_changes,
            transaction_summary,
        }
    }
}
