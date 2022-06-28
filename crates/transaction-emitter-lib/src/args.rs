// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_sdk::types::chain_id::ChainId;
use clap::Parser;
use url::Url;

#[derive(Parser, Debug)]
pub struct ClusterArgs {
    /// Nodes the cluster should connect to, e.g. http://node.mysite.com:8080
    /// If the port is not provided, it is assumed to be 8080.
    #[clap(short, long, required = true, min_values = 1)]
    pub targets: Vec<Url>,

    /// Ed25519PrivateKey for minting coins
    #[clap(long, parse(try_from_str = ConfigKey::from_encoded_string))]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,

    #[clap(long, default_value = "mint.key")]
    pub mint_file: String,

    #[clap(long, default_value = "TESTING")]
    pub chain_id: ChainId,

    /// If set, try to use public peers instead of localhost.
    #[clap(long)]
    pub vasp: bool,
}

#[derive(Parser, Debug)]
pub struct EmitArgs {
    #[clap(long, default_value = "15")]
    pub accounts_per_client: usize,

    #[clap(long)]
    pub workers_per_ac: Option<usize>,

    #[clap(long, default_value = "0")]
    pub wait_millis: u64,

    #[clap(long)]
    pub burst: bool,

    /// This can only be set in conjunction with --burst. By default, when burst
    /// is enabled, we do not ever check the transaction stats. If this is set,
    /// we will check the stats once at the end.
    #[clap(long, requires = "burst")]
    pub check_stats_at_end: bool,

    /// The transaction emitter will submit no more than this many transactions
    /// per second. So if max TPS is 1600 and there are 16 workers, each worker
    /// will submit no more than 100 per second each.
    #[clap(long, default_value = "100000")]
    pub max_tps: u64,

    #[clap(long, default_value = "30")]
    pub txn_expiration_time_secs: u64,

    /// Time to run --emit-tx for in seconds.
    #[clap(long, default_value = "60")]
    pub duration: u64,

    #[clap(long, help = "Percentage of invalid txs", default_value = "0")]
    pub invalid_tx: usize,
}
