// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use first_transaction::{Account, FaucetClient, FAUCET_URL, TESTNET_URL};
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

    let client = HelloBlockchainClient::new(TESTNET_URL.to_string());
    let faucet_client = FaucetClient::new(FAUCET_URL.to_string(), client.rest_client.clone());

    // Create two accounts, Alice and Bob
    let mut alice = Account::new(None);
    let mut bob = Account::new(None);

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

    println!("\nUpdate the module with Alice's address, build, copy to the provided path, and press enter.");
    match std::io::stdin().read_line(&mut String::new()) {
        Ok(_n) => {}
        Err(error) => println!("error: {}", error),
    }

    let module_path = args.get(1).unwrap();
    let module_hex = hex::encode(std::fs::read(module_path).unwrap());

    println!("\n=== Testing Alice ===");
    println!("Publishing...");
    let mut tx_hash = client.publish_module(&mut alice, &module_hex);
    client.rest_client.wait_for_transaction(&tx_hash);
    println!(
        "Initial value: {:?}",
        client.get_message(&alice.address(), &alice.address())
    );
    println!("Setting the message to \"Hello, Blockchain\"");
    tx_hash = client.set_message(&alice.address(), &mut alice, &"Hello, Blockchain");
    client.rest_client.wait_for_transaction(&tx_hash);
    println!(
        "New value: {:?}",
        client.get_message(&alice.address(), &alice.address())
    );

    println!("\n=== Testing Bob ===");
    println!(
        "Initial value: {:?}",
        client.get_message(&alice.address(), &bob.address())
    );
    println!(
        "Initial value: {:?}",
        client.get_message(&alice.address(), &bob.address())
    );
    println!("Setting the message to \"Hello, Blockchain\"");
    tx_hash = client.set_message(&alice.address(), &mut bob, &"Hello, Blockchain");
    client.rest_client.wait_for_transaction(&tx_hash);
    println!(
        "New value: {:?}",
        client.get_message(&alice.address(), &bob.address())
    );
}
