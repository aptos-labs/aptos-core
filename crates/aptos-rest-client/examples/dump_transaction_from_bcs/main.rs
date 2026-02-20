// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{Context, Result};
use aptos_api_types::TransactionData;
use aptos_crypto::HashValue;
use aptos_rest_client::Client;
use clap::Parser;
use reqwest::Url;

const DEVNET_API_URL: &str = "https://fullnode.devnet.aptoslabs.com/v1";

#[derive(Debug, Parser)]
#[clap(author, version, about = "Fetch a transaction by hash via BCS and print it")]
pub struct Args {
    /// Transaction hash (with or without 0x prefix)
    #[clap(long)]
    txn_hash: String,

    /// API URL (defaults to devnet)
    #[clap(long, default_value = DEVNET_API_URL)]
    api_url: Url,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let hash = HashValue::from_hex(&args.txn_hash)
        .or_else(|_| HashValue::from_hex(&args.txn_hash))
        .context("Invalid transaction hash")?;

    let client = Client::new(args.api_url);

    let response = client
        .get_transaction_by_hash_bcs(hash)
        .await
        .context("Failed to fetch transaction by hash (BCS)")?;

    match response.inner() {
        TransactionData::OnChain(txn) => {
            println!("Transaction (version {}):", txn.version);
            println!("{:#?}", txn);
        },
        TransactionData::Pending(signed_txn) => {
            println!("Transaction is still pending:");
            println!("{:#?}", signed_txn);
        },
    }

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
