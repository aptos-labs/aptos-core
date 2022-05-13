// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use first_transaction::{Account, FaucetClient, FAUCET_URL, TESTNET_URL};
use first_coin::FirstCoinClient;
use hello_blockchain::HelloBlockchainClient;
use std::env;

fn main() -> () {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);

    assert_eq!(
        args.len(),
        2,
        "Expecting an argument that points to the helloblockchain module"
    );

    let client = FirstCoinClient::new(TESTNET_URL.to_string());
    let faucet_client = FaucetClient::new(FAUCET_URL.to_string(), client.rest_client.clone());

    // Create two accounts, Alice and Bob
    let mut alice = Account::new(None);
    let mut bob = Account::new(None);

    println!("\n=== Addresses ===");
    println!("Alice: 0x{}", alice.address());
    println!("Bob: 0x{}", bob.address());

    faucet_client.fund_account(&alice.auth_key(), 10_000_000);
    faucet_client.fund_account(&bob.auth_key(), 10_000_000);

    println!("\nUpdate the module with Alice's address, build, copy to the provided path, and press enter.");
    match std::io::stdin().read_line(&mut String::new()) {
        Ok(_n) => {}
        Err(error) => println!("error: {}", error),
    }

    let module_path = args.get(1).unwrap();
    let module_hex = hex::encode(std::fs::read(module_path).unwrap());

    println!("Publishing MoonCoinType module...");
    let hello_blockchain_client = HelloBlockchainClient::new(TESTNET_URL.to_string());
    let mut tx_hash = hello_blockchain_client.publish_module(&mut alice, &module_hex);
    hello_blockchain_client.rest_client.wait_for_transaction(&tx_hash);

    println!("Alice will initialize the new coin");
    tx_hash = client.initialize_coin(&mut alice);
    client.rest_client.wait_for_transaction(&tx_hash);

    println!("Bob registers the newly created coin so he can receive it from Alice");
    tx_hash = client.register_coin(&mut bob, &alice.address());
    client.rest_client.wait_for_transaction(&tx_hash);
    println!("Bob's initial balance: {}", client.get_balance(&bob.address(), &alice.address()));

    println!("Alice mints Bob some of the new coin");
    tx_hash = client.mint_coin(&mut alice, &bob.address(), 100);
    client.rest_client.wait_for_transaction(&tx_hash);
    println!("Bob's updated balance: {}", client.get_balance(&bob.address(), &alice.address()));
}
