// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::{convert::TryFrom, path::Path};

use anyhow::{bail, format_err, Result};
use aptos::common::types::EncodingType;
use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::types::chain_id::ChainId;
use clap::{ArgEnum, ArgGroup, Parser};

use serde::{Deserialize, Serialize};
use url::Url;

const DEFAULT_API_PORT: u16 = 8080;

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct MintArgs {
    /// Ed25519PrivateKey for minting coins
    #[clap(long, parse(try_from_str = ConfigKey::from_encoded_string))]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,

    #[clap(long, default_value = "mint.key", conflicts_with = "mint-key")]
    pub mint_file: String,
}

impl MintArgs {
    pub fn get_mint_key(&self) -> Result<Ed25519PrivateKey> {
        let key = match &self.mint_key {
            Some(ref key) => key.private_key(),
            None => EncodingType::BCS
                .load_key::<Ed25519PrivateKey>("mint key pair", Path::new(&self.mint_file))?,
        };
        Ok(key)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
pub struct ClusterArgs {
    /// Nodes the cluster should connect to, e.g. http://node.mysite.com:8080
    /// If the port is not provided, it is assumed to be 8080.
    #[clap(short, long, required = true, min_values = 1, parse(try_from_str = parse_target))]
    pub targets: Vec<Url>,

    /// If set, try to use public peers instead of localhost.
    #[clap(long)]
    pub reuse_accounts: bool,

    #[clap(long, default_value = "TESTING")]
    pub chain_id: ChainId,

    #[clap(flatten)]
    pub mint_args: MintArgs,
}

#[derive(Debug, Clone, Copy, ArgEnum, Deserialize, Parser, Serialize)]
pub enum TransactionType {
    P2P,
    AccountGeneration,
    NftMint,
}

impl Default for TransactionType {
    fn default() -> Self {
        TransactionType::P2P
    }
}

#[derive(Clone, Debug, Default, Deserialize, Parser, Serialize)]
#[clap(group(
    ArgGroup::new("mode")
        .required(true)
        .args(&["mempool-backlog", "target-tps"]),
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

    #[clap(long, default_value = "30")]
    pub txn_expiration_time_secs: u64,

    /// Time to run --emit-tx for in seconds.
    #[clap(long, default_value = "60")]
    pub duration: u64,

    #[clap(long, help = "Percentage of invalid txs", default_value = "0")]
    pub invalid_tx: usize,

    #[clap(
        long,
        arg_enum,
        default_value = "p2p",
        min_values = 1,
        ignore_case = true
    )]
    pub transaction_type: Vec<TransactionType>,

    #[clap(long, min_values = 0)]
    pub transaction_type_weights: Vec<usize>,
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
