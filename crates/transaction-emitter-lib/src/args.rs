// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, format_err, Result};
use aptos_config::keys::ConfigKey;
use aptos_crypto::{ed25519::Ed25519PrivateKey, encoding_type::EncodingType};
use aptos_sdk::types::chain_id::ChainId;
use aptos_transaction_generator_lib::{args::TransactionTypeArg, AccountType};
use clap::{ArgGroup, Parser};
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};
use url::Url;

const DEFAULT_API_PORT: u16 = 8080;

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct CoinSourceArgs {
    /// Ed25519PrivateKey for minting coins
    #[clap(long, value_parser = ConfigKey::<Ed25519PrivateKey>::from_encoded_string)]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,

    #[clap(long, conflicts_with = "mint_key")]
    pub mint_file: Option<String>,

    /// Ed25519PrivateKey for minting coins
    #[clap(long, value_parser = ConfigKey::<Ed25519PrivateKey>::from_encoded_string, conflicts_with_all = &["mint_key", "mint_file"])]
    pub coin_source_key: Option<ConfigKey<Ed25519PrivateKey>>,

    #[clap(long, conflicts_with_all = &["mint_key", "mint_file", "coin_source_key"])]
    pub coin_source_file: Option<String>,
}

impl CoinSourceArgs {
    pub fn get_private_key(&self) -> Result<(Ed25519PrivateKey, bool)> {
        match (
            &self.mint_key,
            &self.mint_file,
            &self.coin_source_key,
            &self.coin_source_file,
        ) {
            (Some(ref key), None, None, None) => Ok((key.private_key(), true)),
            (None, Some(path), None, None) => Ok((
                EncodingType::BCS
                    .load_key::<Ed25519PrivateKey>("mint key pair", Path::new(path))?,
                true,
            )),
            (None, None, Some(ref key), None) => Ok((key.private_key(), false)),
            (None, None, None, Some(path)) => Ok((
                EncodingType::BCS
                    .load_key::<Ed25519PrivateKey>("mint key pair", Path::new(path))?,
                false,
            )),
            _ => Err(anyhow::anyhow!("Please provide exactly one of mint-key, mint-file, coin-source-key, or coin-source-file")),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct ClusterArgs {
    /// Nodes the cluster should connect to, e.g. `http://node.mysite.com:8080`
    /// If the port is not provided, it is assumed to be 8080.
    #[clap(short, long, required = true, num_args = 1.., value_parser = parse_target)]
    pub targets: Option<Vec<Url>>,

    #[clap(long, conflicts_with = "targets")]
    pub targets_file: Option<String>,

    // If the chain_id is not provided, it is derived from the targets.
    #[clap(long)]
    pub chain_id: Option<ChainId>,

    #[clap(flatten)]
    pub coin_source_args: CoinSourceArgs,

    /// Key to use for ratelimiting purposes with the node API. This value will be used
    /// as `Authorization: Bearer <key>`.
    #[clap(long, env)]
    pub node_api_key: Option<String>,
}

impl ClusterArgs {
    pub fn get_targets(&self) -> Result<Vec<Url>> {
        match (&self.targets, &self.targets_file) {
            (Some(targets), _) => Ok(targets.clone()),
            (None, Some(target_file)) => Self::get_targets_from_file(target_file),
            (_, _) => Err(anyhow::anyhow!("Expected either targets or target_file")),
        }
    }

    fn get_targets_from_file(path: &String) -> Result<Vec<Url>> {
        let reader = BufReader::new(File::open(path)?);
        let mut urls = Vec::new();

        for line in reader.lines() {
            let url_string = &line?;
            urls.push(parse_target(url_string)?);
        }
        Ok(urls)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
#[clap(group(
    ArgGroup::new("mode")
        .required(true)
        .args(&["mempool_backlog", "target_tps"]),
))]
pub struct EmitArgs {
    #[clap(long)]
    /// Number of transactions outstanding in mempool - this is needed to ensure that the emitter
    /// is producing enough load to get the highest TPS in the system. Typically this should be
    /// configured to be ~4x of the max achievable TPS.
    /// 0 if target_tps used.
    pub mempool_backlog: Option<usize>,

    /// Target constant TPS, 0 if mempool_backlog used
    #[clap(long)]
    pub target_tps: Option<usize>,

    #[clap(long, default_value_t = 30)]
    pub txn_expiration_time_secs: u64,

    /// Time to run --emit-tx for in seconds.
    #[clap(long, default_value_t = 60)]
    pub duration: u64,

    #[clap(
        long,
        value_enum,
        default_value = "coin-transfer",
        num_args = 1..,
        ignore_case = true
    )]
    pub transaction_type: Vec<TransactionTypeArg>,

    #[clap(long, value_enum, default_value = "local", ignore_case = true)]
    pub account_type: AccountType,

    /// Number of copies of the modules that will be published,
    /// under separate accounts, creating independent contracts,
    /// removing contention.
    /// For example for NFT minting, setting to 1 will be equivalent
    /// to minting from single collection,
    /// setting to 20 means minting from 20 collections in parallel.
    #[clap(long)]
    pub module_working_set_size: Option<usize>,

    /// Whether to use burner accounts for the sender.
    /// For example when transaction can only be done once per account.
    /// (pool needs to be populated by account-creation transactions)
    #[clap(long)]
    pub sender_use_account_pool: Option<bool>,

    #[clap(long, num_args = 0..)]
    pub transaction_weights: Vec<usize>,

    #[clap(long, num_args = 0..)]
    pub transaction_phases: Vec<usize>,

    #[clap(long)]
    pub gas_price: Option<u64>,

    #[clap(long)]
    pub max_gas_per_txn: Option<u64>,

    #[clap(long)]
    pub init_max_gas_per_txn: Option<u64>,

    #[clap(long)]
    pub init_gas_price_multiplier: Option<u64>,

    #[clap(long)]
    pub expected_max_txns: Option<u64>,

    #[clap(long)]
    pub expected_gas_per_txn: Option<u64>,

    #[clap(long)]
    pub expected_gas_per_transfer: Option<u64>,

    #[clap(long)]
    pub expected_gas_per_account_create: Option<u64>,

    #[clap(long, conflicts_with = "num_accounts")]
    pub max_transactions_per_account: Option<usize>,

    #[clap(long, conflicts_with = "max_transactions_per_account")]
    pub num_accounts: Option<usize>,

    #[clap(long, default_value = "false")]
    /// Skip minting account during initialization
    pub skip_minting_accounts: bool,

    #[clap(long)]
    pub latency_polling_interval_s: Option<f32>,

    // In cases you want to run txn emitter from multiple machines,
    // and want to make sure that initialization succeeds
    // (account minting and txn-specific initialization), before the
    // loadtest puts significant load, you can add a delay here.
    //
    // This also enables few other changes needed to run txn emitter
    // from multiple machines simultaneously:
    // - retrying minting phase before this delay expires
    // - having a single transaction on root/source account
    //   (to reduce contention and issues with sequence numbers across multiple txn emitters).
    //   basically creating a new source account (to then create seed accounts from).
    #[clap(long)]
    pub coordination_delay_between_instances: Option<u64>,

    #[clap(long)]
    pub account_minter_seed: Option<String>,

    #[clap(long)]
    pub coins_per_account_override: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct CreateAccountsArgs {
    /// Number of accounts to create
    #[clap(long)]
    pub count: usize,

    /// The amount of gas needed to create an account
    #[clap(long)]
    pub max_gas_per_txn: u64,

    /// Optional seed, which is compatible with emit-tx. If no seed is provided, a random seed is
    /// used and printed.
    #[clap(long)]
    pub account_minter_seed: Option<String>,
}

fn parse_target(target: &str) -> Result<Url> {
    let mut url = Url::try_from(target).map_err(|e| {
        format_err!(
            "Failed to parse listen address, try adding a scheme, e.g. http://: {:?}",
            e
        )
    })?;
    if url.scheme().is_empty() {
        bail!("Scheme must not be empty, try prefixing URL with http://");
    }
    if url.port_or_known_default().is_none() {
        url.set_port(Some(DEFAULT_API_PORT)).map_err(|_| {
            anyhow::anyhow!(
                "Failed to set port to default value, make sure you have set a scheme like http://"
            )
        })?;
    }
    Ok(url)
}
