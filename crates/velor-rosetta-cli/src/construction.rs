// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, NetworkArgs, UrlArgs};
use velor::common::types::{EncodingOptions, PrivateKeyInputOptions, ProfileOptions};
use velor_logger::info;
use velor_rosetta::types::{Currency, TransactionIdentifier};
use velor_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Construction commands
///
/// At a high level, this provides the full E2E commands provided by the construction API for
/// Rosetta.  This can be used for testing to ensure everything works properly
#[derive(Debug, Subcommand)]
pub enum ConstructionCommand {
    CreateAccount(CreateAccountCommand),
    SetOperator(SetOperatorCommand),
    SetVoter(SetVoterCommand),
    Transfer(TransferCommand),
    CreateStakePool(CreateStakePoolCommand),
}

impl ConstructionCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        use ConstructionCommand::*;
        match self {
            CreateAccount(inner) => format_output(inner.execute().await),
            SetOperator(inner) => format_output(inner.execute().await),
            SetVoter(inner) => format_output(inner.execute().await),
            Transfer(inner) => format_output(inner.execute().await),
            CreateStakePool(inner) => format_output(inner.execute().await),
        }
    }
}

#[derive(Debug, Parser)]
pub struct TransactionArgs {
    /// Number of seconds from now to expire
    ///
    /// If not provided, it will default to 60 seconds
    #[clap(long, default_value_t = 60)]
    expiry_offset_secs: i64,
    /// Sequence number for transaction
    ///
    /// If not provided, the Rosetta server will pull from onchain
    #[clap(long)]
    sequence_number: Option<u64>,
    /// Maximum gas amount for a transaction
    ///
    /// If not provided, the Rosetta server will estimate it
    #[clap(long)]
    max_gas: Option<u64>,
    /// Gas price per unit of gas
    ///
    /// If not provided, the Rosetta server will estimate it
    #[clap(long)]
    gas_price: Option<u64>,
}

impl TransactionArgs {
    /// Calculate expiry time given the offset seconds
    pub fn expiry_time(&self) -> anyhow::Result<u64> {
        let offset = self.expiry_offset_secs;
        if offset > 0 {
            Ok(
                (SystemTime::now().duration_since(UNIX_EPOCH)?
                    + Duration::from_secs(offset as u64))
                .as_secs(),
            )
        } else {
            Ok((SystemTime::now().duration_since(UNIX_EPOCH)?
                - Duration::from_secs((-offset) as u64))
            .as_secs())
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
    #[clap(flatten)]
    txn_args: TransactionArgs,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    sender: Option<AccountAddress>,
    /// The new account (TODO: Maybe we want to take in the public key instead)
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    new_account: AccountAddress,
}

impl CreateAccountCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Create account: {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding, &self.profile_options)?;

        client
            .create_account(
                &network_identifier,
                &private_key,
                self.new_account,
                self.txn_args.expiry_time()?,
                self.txn_args.sequence_number,
                self.txn_args.max_gas,
                self.txn_args.gas_price,
            )
            .await
    }
}

/// Transfer coins via Rosetta
///
/// Only the native coin is allowed for now.  It will create the account if it
/// does not exist
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
    #[clap(flatten)]
    txn_args: TransactionArgs,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    sender: Option<AccountAddress>,
    /// The receiving account
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    receiver: AccountAddress,
    /// The amount of coins to send
    #[clap(long)]
    amount: u64,
    #[clap(long, value_parser = parse_currency)]
    currency: Currency,
}

fn parse_currency(str: &str) -> anyhow::Result<Currency> {
    Ok(serde_json::from_str(str)?)
}

impl TransferCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Transfer {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding, &self.profile_options)?;

        client
            .transfer(
                &network_identifier,
                &private_key,
                self.receiver,
                self.amount,
                self.txn_args.expiry_time()?,
                self.txn_args.sequence_number,
                self.txn_args.max_gas,
                self.txn_args.gas_price,
                self.currency,
            )
            .await
    }
}

/// Set operator
///
///
#[derive(Debug, Parser)]
pub struct SetOperatorCommand {
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
    #[clap(flatten)]
    txn_args: TransactionArgs,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    sender: Option<AccountAddress>,
    /// The old operator of the stake pool
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    old_operator: Option<AccountAddress>,
    /// The new operator of the stake pool
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    new_operator: AccountAddress,
}

impl SetOperatorCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Set operator {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding, &self.profile_options)?;

        client
            .set_operator(
                &network_identifier,
                &private_key,
                self.old_operator,
                self.new_operator,
                self.txn_args.expiry_time()?,
                self.txn_args.sequence_number,
                self.txn_args.max_gas,
                self.txn_args.gas_price,
            )
            .await
    }
}

/// Set voter
///
///
#[derive(Debug, Parser)]
pub struct SetVoterCommand {
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
    #[clap(flatten)]
    txn_args: TransactionArgs,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    sender: Option<AccountAddress>,
    /// The operator of the stake pool
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    operator: Option<AccountAddress>,
    /// The new voter for the stake pool
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    new_voter: AccountAddress,
}

impl SetVoterCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("Set voter {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding, &self.profile_options)?;

        client
            .set_voter(
                &network_identifier,
                &private_key,
                self.operator,
                self.new_voter,
                self.txn_args.expiry_time()?,
                self.txn_args.sequence_number,
                self.txn_args.max_gas,
                self.txn_args.gas_price,
            )
            .await
    }
}

/// Initialize stake amount
///
///
#[derive(Debug, Parser)]
pub struct CreateStakePoolCommand {
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
    #[clap(flatten)]
    txn_args: TransactionArgs,
    /// The sending account, since the private key doesn't always match the
    /// AccountAddress if it rotates
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    sender: Option<AccountAddress>,
    /// Operator
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    operator: Option<AccountAddress>,
    /// Voter
    #[clap(long, value_parser = velor::common::types::load_account_arg)]
    voter: Option<AccountAddress>,
    /// Amount
    #[clap(long)]
    amount: Option<u64>,
    /// Commission percentage
    #[clap(long)]
    commission_percentage: Option<u64>,
}

impl CreateStakePoolCommand {
    pub async fn execute(self) -> anyhow::Result<TransactionIdentifier> {
        info!("CreateStakePool {:?}", self);
        let client = self.url_args.client();
        let network_identifier = self.network_args.network_identifier();
        let private_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding, &self.profile_options)?;

        client
            .create_stake_pool(
                &network_identifier,
                &private_key,
                self.operator,
                self.voter,
                self.amount,
                self.commission_percentage,
                self.txn_args.expiry_time()?,
                self.txn_args.sequence_number,
                self.txn_args.max_gas,
                self.txn_args.gas_price,
            )
            .await
    }
}
