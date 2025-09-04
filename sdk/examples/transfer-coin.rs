// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use velor_sdk::{
    coin_client::CoinClient,
    rest_client::{Client, FaucetClient},
    types::LocalAccount,
};
use once_cell::sync::Lazy;
use std::str::FromStr;
use url::Url;

// :!:>section_1c
static NODE_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("VELOR_NODE_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://fullnode.devnet.velorlabs.com"),
    )
    .unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("VELOR_FAUCET_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://faucet.devnet.velorlabs.com"),
    )
    .unwrap()
});
// <:!:section_1c

#[tokio::main]
async fn main() -> Result<()> {
    // :!:>section_1a
    let rest_client = Client::new(NODE_URL.clone());
    let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

    // :!:>section_1b
    let coin_client = CoinClient::new(&rest_client); // <:!:section_1b

    // Create two accounts locally, Alice and Bob.
    // :!:>section_2
    let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
    let bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

    // Print account addresses.
    println!("\n=== Addresses ===");
    println!("Alice: {}", alice.address().to_hex_literal());
    println!("Bob: {}", bob.address().to_hex_literal());

    // Create the accounts on chain, but only fund Alice.
    // :!:>section_3
    faucet_client
        .fund(alice.address(), 100_000_000)
        .await
        .context("Failed to fund Alice's account")?;
    faucet_client
        .create_account(bob.address())
        .await
        .context("Failed to fund Bob's account")?; // <:!:section_3

    // Print initial balances.
    println!("\n=== Initial Balances ===");
    println!(
        "Alice: {:?}",
        coin_client
            .get_account_balance(&alice.address())
            .await
            .context("Failed to get Alice's account balance")?
    );
    println!(
        "Bob: {:?}",
        coin_client
            .get_account_balance(&bob.address())
            .await
            .context("Failed to get Bob's account balance")?
    );

    // Have Alice send Bob some coins.
    let txn_hash = coin_client
        .transfer(&mut alice, bob.address(), 1_000, None)
        .await
        .context("Failed to submit transaction to transfer coins")?;
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the transfer transaction")?;

    // Print intermediate balances.
    println!("\n=== Intermediate Balances ===");
    // :!:>section_4
    println!(
        "Alice: {:?}",
        coin_client
            .get_account_balance(&alice.address())
            .await
            .context("Failed to get Alice's account balance the second time")?
    );
    println!(
        "Bob: {:?}",
        coin_client
            .get_account_balance(&bob.address())
            .await
            .context("Failed to get Bob's account balance the second time")?
    ); // <:!:section_4

    // Have Alice send Bob some more coins.
    // :!:>section_5
    let txn_hash = coin_client
        .transfer(&mut alice, bob.address(), 1_000, None)
        .await
        .context("Failed to submit transaction to transfer coins")?; // <:!:section_5
                                                                     // :!:>section_6
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the transfer transaction")?; // <:!:section_6

    // Print final balances.
    println!("\n=== Final Balances ===");
    println!(
        "Alice: {:?}",
        coin_client
            .get_account_balance(&alice.address())
            .await
            .context("Failed to get Alice's account balance the second time")?
    );
    println!(
        "Bob: {:?}",
        coin_client
            .get_account_balance(&bob.address())
            .await
            .context("Failed to get Bob's account balance the second time")?
    );

    Ok(())
}
