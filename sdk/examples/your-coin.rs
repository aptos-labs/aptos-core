// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_sdk::{
    coin_client::CoinClient,
    rest_client::{Client, FaucetClient},
    package_publisher::PackagePublisher,
    types::LocalAccount,
};
use aptos_sdk::coin_client::ManagedCoinOptions;
use aptos_sdk::coin_client::TransferOptions;
use once_cell::sync::Lazy;
use std::str::FromStr;
use url::Url;
use std::io;
use std::env;

static NODE_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_NODE_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://fullnode.devnet.aptoslabs.com"),
    )
    .unwrap()
});

static FAUCET_URL: Lazy<Url> = Lazy::new(|| {
    Url::from_str(
        std::env::var("APTOS_FAUCET_URL")
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("https://faucet.devnet.aptoslabs.com"),
    )
    .unwrap()
});

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        panic!("Expecting an argument that points to the moon_coin directory.")
    }

    let rest_client = Client::new(NODE_URL.clone());
    let faucet_client = FaucetClient::new(FAUCET_URL.clone(), NODE_URL.clone()); // <:!:section_1a

    let coin_client = CoinClient::new(&rest_client); // <:!:section_1b
    let package_publisher = PackagePublisher::new(&rest_client); // <:!:section_1b

    // Create two accounts locally, Alice and Bob.
    let mut alice = LocalAccount::generate(&mut rand::rngs::OsRng);
    let mut bob = LocalAccount::generate(&mut rand::rngs::OsRng); // <:!:section_2

    println!("\n=== Addresses ===");
    println!("Alice: {}", alice.address().to_hex_literal());
    println!("Bob: {}", bob.address().to_hex_literal());

    faucet_client
        .fund(alice.address(), 100_000_000)
        .await
        .context("Failed to fund Alice's account")?;
    faucet_client
        .fund(bob.address(), 100_000_000)
        .await
        .context("Failed to fund Bob's account")?; 

    let mut buffer: String = String::with_capacity(0);
    let reader = io::stdin();
    println!("Update the module with Alice's address, compile, and press enter.");
    reader.read_line(&mut buffer)
        .ok()
        .expect("ERRMSG");
    buffer.clear();

    let module_path = &args[1];

    let package_metadata = std::fs::read(module_path.to_owned() + "/build/Examples/package-metadata.bcs").unwrap();
    let module_data = std::fs::read(module_path.to_owned() + "/build/Examples/bytecode_modules/moon_coin.mv").unwrap();

    println!("Publishing MoonCoin package.");

    let txn_hash = package_publisher
        .publish_package(&mut alice, package_metadata, vec![module_data], None)
        .await
        .context("Failed to publish Mooncoin package")?;
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the publish transaction")?;

    let coin_type = alice.address().to_hex_literal() + "::moon_coin::MoonCoin";
    
    let txn_hash = coin_client
        .register_coin(&mut bob, Some(
            ManagedCoinOptions {
                coin_type: coin_type.as_str(),
                ..Default::default()
            }
        ))
        .await
        .context("Failed to register coin")?;
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the register transaction")?;
    
    println!("Bob's initial MoonCoin balance: {}.", coin_client.get_balance(&bob.address(), &coin_type).await?);
    
    println!("Alice mints herself some of the new coin.");
    
    let txn_hash = coin_client
    .register_coin(&mut alice, Some(
        ManagedCoinOptions {
            coin_type: coin_type.as_str(),
            ..Default::default()
        }
    ))
    .await
    .context("Failed to register coin")?;
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the register transaction")?;
    
    let clone_alice_address = alice.address().clone();
    let txn_hash = coin_client
        .mint_coin(&mut alice, clone_alice_address, 100, Some(
            ManagedCoinOptions {
                coin_type: coin_type.as_str(),
                ..Default::default()
            }
        ))
        .await
        .context("Failed to mint coin")?;
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the mint transaction")?;

    println!("Alice transfers the newly minted coins to Bob.");
    
    let txn_hash = coin_client
        .transfer(&mut alice, bob.address(), 100, Some(
            TransferOptions {
                coin_type: coin_type.as_str(),
                ..Default::default()
            }
        ))
        .await
        .context("Failed to transfer coin")?;
    rest_client
        .wait_for_transaction(&txn_hash)
        .await
        .context("Failed when waiting for the transfer transaction")?;

    println!("Bob's updated MoonCoin balance: {}.", coin_client.get_balance(&bob.address(), &coin_type).await?);
    
    Ok(())
}
