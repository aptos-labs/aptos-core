// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to create a new account on-chain
//!
//! TODO: Examples
//!

use crate::common::{
    types::{
        account_address_from_public_key, CliError, CliTypedResult, EncodingOptions,
        ExtractPublicKey, FaucetOptions, PublicKeyInputOptions, WriteTransactionOptions,
    },
    utils::send_transaction,
};
use aptos_rest_client::FaucetClient;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use clap::Parser;

/// Command to create a new account on-chain
///
#[derive(Debug, Parser)]
pub struct CreateAccount {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    write_options: WriteTransactionOptions,
    #[clap(flatten)]
    public_key_options: PublicKeyInputOptions,
    /// Flag for using faucet to create the account
    #[clap(long)]
    use_faucet: bool,
    #[clap(flatten)]
    faucet_options: FaucetOptions,
}

impl CreateAccount {
    pub async fn execute(self) -> CliTypedResult<String> {
        let public_key_to_create = self
            .public_key_options
            .extract_public_key(self.encoding_options.encoding)?;
        let address = account_address_from_public_key(&public_key_to_create);
        if self.use_faucet {
            self.create_account_with_faucet(address).await
        } else {
            self.create_account_with_key(address).await
        }
        .map(|_| format!("Account Created at {}", address))
    }

    /// Creates an account and funds it from the faucet
    async fn create_account_with_faucet(self, address: AccountAddress) -> CliTypedResult<()> {
        let faucet_client = FaucetClient::new(
            self.faucet_options.faucet_url.to_string(),
            self.write_options.rest_options.url.to_string(),
        );
        faucet_client
            .fund(address, self.faucet_options.num_coins)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))
    }

    /// Creates an account and does not fund it
    async fn create_account_with_key(self, address: AccountAddress) -> CliTypedResult<()> {
        let payload = aptos_stdlib::encode_create_account_script_function(address);
        send_transaction(self.encoding_options, self.write_options, payload)
            .await
            .map(|_| ())
    }
}
