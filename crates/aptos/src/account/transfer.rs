// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliTypedResult, EncodingOptions, ProfileOptions, WriteTransactionOptions},
    utils::submit_transaction,
};
use aptos_rest_client::{aptos_api_types::WriteSetChange, Transaction};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use cached_framework_packages::aptos_stdlib;
use clap::Parser;
use serde::Serialize;
use std::collections::BTreeMap;

/// Command to transfer coins between accounts
///
#[derive(Debug, Parser)]
pub struct TransferCoins {
    #[clap(flatten)]
    write_options: WriteTransactionOptions,

    #[clap(flatten)]
    encoding_options: EncodingOptions,

    #[clap(flatten)]
    profile_options: ProfileOptions,

    /// Address of account you want to send coins to
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    account: AccountAddress,

    /// Amount of coins to transfer
    #[clap(long)]
    amount: u64,
}

#[async_trait]
impl CliCommand<TransferSummary> for TransferCoins {
    fn command_name(&self) -> &'static str {
        "TransferCoins"
    }

    async fn execute(self) -> CliTypedResult<TransferSummary> {
        let sender_key = self.write_options.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        submit_transaction(
            self.write_options
                .rest_options
                .url(&self.profile_options.profile)?,
            self.write_options
                .chain_id(&self.profile_options.profile)
                .await?,
            sender_key,
            aptos_stdlib::encode_test_coin_transfer(self.account, self.amount),
            self.write_options.max_gas,
        )
        .await
        .map(TransferSummary::from)
    }
}

const SUPPORTED_COINS: [&str; 1] = ["0x1::TestCoin::Balance"];

/// A shortened transaction output
#[derive(Clone, Debug, Default, Serialize)]
pub struct TransferSummary {
    gas_used: Option<u64>,
    balance_changes: BTreeMap<AccountAddress, serde_json::Value>,
    sender: Option<AccountAddress>,
    success: bool,
    version: Option<u64>,
    vm_status: String,
}

impl From<Transaction> for TransferSummary {
    fn from(transaction: Transaction) -> Self {
        let mut summary = TransferSummary {
            success: transaction.success(),
            version: transaction.version(),
            vm_status: transaction.vm_status(),
            ..Default::default()
        };

        if let Transaction::UserTransaction(txn) = transaction {
            summary.sender = Some(*txn.request.sender.inner());
            summary.gas_used = Some(txn.info.gas_used.0);
            summary.version = Some(txn.info.version.0);
            summary.balance_changes = txn
                .info
                .changes
                .iter()
                .filter_map(|change| match change {
                    WriteSetChange::WriteResource { address, data, .. } => {
                        if SUPPORTED_COINS.contains(&data.typ.to_string().as_str()) {
                            Some((
                                *address.inner(),
                                serde_json::to_value(data.data.clone()).unwrap_or_default(),
                            ))
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
                .collect();
        }

        summary
    }
}
