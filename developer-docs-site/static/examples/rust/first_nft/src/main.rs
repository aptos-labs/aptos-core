// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use first_nft::NftClient;
use first_transaction::{Account, FaucetClient, FAUCET_URL, TESTNET_URL};

fn main() {
    let client = NftClient::new(TESTNET_URL);
    let faucet_client = FaucetClient::new(FAUCET_URL.to_string(), client.rest_client.clone());

    let mut alice = Account::new(None);
    let alice_address = alice.address();
    let mut bob = Account::new(None);
    let collection_name = "Alice's";
    let token_name = "Alice's first token";
    println!("\n=== Addresses ===");
    println!("Alice: 0x{}", alice.address());
    println!("Bob: 0x{}", bob.address());

    faucet_client.fund_account(&alice.auth_key(), 10_000_000);
    faucet_client.fund_account(&bob.auth_key(), 10_000_000);

    println!("\n=== Initial Balances ===");
    println!(
        "Alice: {:?}",
        client.rest_client.account_balance(&alice.address())
    );
    println!(
        "Bob: {:?}",
        client.rest_client.account_balance(&bob.address())
    );
    println!("\n=== Creating Collection and Token ===");
    client.create_collection(
        &mut alice,
        collection_name,
        "Alice's simple collection",
        "https://aptos.dev",
    );

    client.create_token(
        &mut alice,
        collection_name,
        token_name,
        "Alice's simple token",
        1,
        "https://aptos.dev/img/nyan.jpeg",
    );

    println!(
        "Alice's collection: {}",
        client.get_collection(&alice.address(), collection_name)
    );

    println!(
        "Alice's token balance: {}",
        client.get_token_balance(
            &alice.address(),
            &alice.address(),
            collection_name,
            token_name
        )
    );

    println!(
        "Alice's token data: {}",
        client.get_token_data(&alice.address(), collection_name, token_name)
    );

    println!("\n=== Transferring the token to Bob ===");
    client.offer_token(
        &mut alice,
        &bob.address(),
        alice_address.as_str(),
        collection_name,
        token_name,
        1,
    );

    client.claim_token(
        &mut bob,
        &alice.address(),
        &alice.address(),
        collection_name,
        token_name,
    );

    println!(
        "Alice's token balance: {}",
        client.get_token_balance(
            &alice.address(),
            &alice.address(),
            collection_name,
            token_name
        )
    );

    println!(
        "Bob's token balance: {}",
        client.get_token_balance(
            &bob.address(),
            &alice.address(),
            collection_name,
            token_name
        )
    );
}
