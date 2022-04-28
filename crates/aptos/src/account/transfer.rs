// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{
        account_address_from_public_key, CliCommand, CliError, CliTypedResult, EncodingOptions,
        ProfileOptions, WriteTransactionOptions,
    },
    utils::get_sequence_number,
};
use aptos_crypto::PrivateKey;
use aptos_rest_client::Transaction;
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use cached_framework_packages::aptos_stdlib;
use clap::Parser;

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
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    account: AccountAddress,

    /// Amount of coins to transfer
    #[clap(long)]
    amount: u64,
}

#[async_trait]
impl CliCommand<Transaction> for TransferCoins {
    fn command_name(&self) -> &'static str {
        "TransferCoins"
    }

    async fn execute(self) -> CliTypedResult<Transaction> {
        let client = aptos_rest_client::Client::new(reqwest::Url::clone(
            &self
                .write_options
                .rest_options
                .url(&self.profile_options.profile)?,
        ));
        let transaction_factory = TransactionFactory::new(
            self.write_options
                .chain_id(&self.profile_options.profile)
                .await?,
        )
        .with_gas_unit_price(1)
        .with_max_gas_amount(self.write_options.max_gas);

        let sender_key = self.write_options.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;
        let sender_public_key = sender_key.public_key();
        let sender_address = account_address_from_public_key(&sender_public_key);
        let sequence_number = get_sequence_number(&client, sender_address).await?;

        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction =
            sender_account.sign_with_transaction_builder(transaction_factory.payload(
                aptos_stdlib::encode_transfer_script_function(self.account, self.amount),
            ));
        client
            .submit_and_wait(&transaction)
            .await
            .map(|response| response.into_inner())
            .map_err(|err| CliError::ApiError(err.to_string()))
    }
}
