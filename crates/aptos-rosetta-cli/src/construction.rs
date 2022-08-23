// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, NetworkArgs, UrlArgs};
use aptos::common::types::{EncodingOptions, PrivateKeyInputOptions, ProfileOptions};
use aptos_logger::info;
use aptos_rosetta::types::TransactionIdentifier;
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Construction commands
///
/// At a high level, this provides the full E2E commands provided by the construction API for
/// Rosetta.  This can be used for testing to ensure everything works properly
#[derive(Debug, Subcommand)]
pub enum ConstructionCommand {
    CreateAccount(CreateAccountCommand),
    Transfer(TransferCommand),
}

impl ConstructionCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        use ConstructionCommand::*;
        match self {
            CreateAccount(inner) => format_output(inner.execute().await),
            Transfer(inner) => format_output(inner.execute().await),
        }
    }
}

/// Creates an account using Rosetta, no funds will be transferred
///
/// EncodingOptions are here so we can allow using the BCS encoded mint key
#[derive(Debug, Parser)]
pub struct CreateAccountCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    sender: Option<AccountAddress>,
    /// The new account (TODO: Maybe we want to take in the public key instead)
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    new_account: AccountAddress,
}

impl CreateAccountCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Create account: {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        client
            .create_account(
                &network_identifier,
                &private_key,
                self.new_account,
                expiry_time()?,
                None,
            )
            .await
    }
}

/// Transfer coins via Rosetta
///
/// Only the native coin is allowed for now
#[derive(Debug, Parser)]
pub struct TransferCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    #[clap(flatten)]
    url_args: UrlArgs,
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    sender: Option<AccountAddress>,
    /// The receiving account
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    receiver: AccountAddress,
    /// The amount of coins to send
    #[clap(long)]
    amount: u64,
}

impl TransferCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Transfer {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        client
            .transfer(
                &network_identifier,
                &private_key,
                self.receiver,
                self.amount,
                expiry_time()?,
                None,
            )
            .await
    }
}

fn expiry_time() -> anyhow::Result<u64> {
    Ok((SystemTime::now().duration_since(UNIX_EPOCH)? + Duration::from_secs(60)).as_secs())
}
