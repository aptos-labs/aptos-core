// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos::common::types::{account_address_from_public_key, EncodingOptions, EncodingType, MovePackageDir, NodeOptions, PrivateKeyInputOptions, PublicKeyInputOptions};
use aptos::op::key::GenerateKey;
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_crypto::{PrivateKey, ValidCryptoMaterialStringExt};
use aptos_sdk::move_types::account_address::AccountAddress;
use forge::Node;
use crate::smoke_test_environment::new_local_swarm_with_aptos;
use crate::test_utils::create_and_fund_account;

/// Test a full E2E test of the aptos CLI  I'm jamming this all into one
#[tokio::test]
async fn test_aptos_cli() {
    let mut swarm = new_local_swarm_with_aptos(num_nodes).await;

    let (alice_private_key, alice_public_key, alice_account) = new_account();
    let (bob_private_key, bob_public_key, bob_account) = new_account();


    let rest_endpoint = swarm.validators().next().unwrap().rest_api_endpoint();
    let alice_public_key_options = public_key_input_options(&alice_public_key);
    let encoding_options = encoding_options();
    let node_options = node_options(rest_endpoint.clone());

    let alice = create_and_fund_account(&mut swarm, 100).await;
    let alice_private_key_options = private_key_input_options(alice.private_key());
    let move_package_dir = MovePackageDir {
        package_dir: "aptos-move/move-examples".parse().unwrap(),
        output_dir: None
    };

    let publish = aptos::move_tool::PublishPackage {
        encoding_options,
        private_key_options: alice_private_key_options,
        move_options: move_package_dir,
        node_options,
        chain_id: swarm.chain_id(),
        max_gas: 1000
    };

    let txn = publish.execute().await.unwrap();
    println!("{:?}", txn);
}

fn new_account() -> (Ed25519PrivateKey, Ed25519PublicKey, AccountAddress) {
    let private_key = GenerateKey::generate_ed25519_in_memory();
    let public_key = private_key.public_key();
    let account = account_address_from_public_key(&public_key);

    (private_key, public_key, account)
}

fn node_options(rest_endpoint: reqwest::Url) -> NodeOptions {
    NodeOptions {
        url: rest_endpoint
    }
}

fn encoding_options() -> EncodingOptions {
    EncodingOptions {
        encoding: EncodingType::Hex
    }
}

fn private_key_input_options(private_key: &Ed25519PrivateKey) -> PrivateKeyInputOptions {
    PrivateKeyInputOptions {
        private_key: Some(private_key.to_encoded_string().unwrap()),
        private_key_file: None,
    }
}

fn public_key_input_options(public_key: &Ed25519PublicKey) -> PublicKeyInputOptions {
    PublicKeyInputOptions{
        public_key: Some(public_key.to_encoded_string().unwrap()),
        public_key_file: None
    }
}