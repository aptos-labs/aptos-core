// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos_forge::Swarm;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::SigningKey;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::function_info::FunctionInfo;
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use serde::Serialize;
use std::sync::Arc;
use blake2_rfc::blake2b::blake2b;

#[derive(Serialize)]
struct SuiAbstractPublicKey {
    // The Sui account address, in hex string format with "0x" prefix
    sui_account_address: Vec<u8>,
    // The domain, in utf8 bytes
    domain: Vec<u8>,
}

#[derive(Serialize)]
enum SuiAbstractSignature {
    MessageV1 {
        /// The signature of the message in raw bytes
        signature: Vec<u8>,
    },
}

#[derive(Serialize)]
enum Scope {
  PersonalMessage = 3,
}

#[derive(Serialize)]
enum Version {
    V0 = 0,
}

#[derive(Serialize)]
enum AppId {
    Sui = 0,
}

#[derive(Serialize)]
struct Intent {
    scope: u8,
    version: u8,
    app_id: u8,
}

#[derive(Serialize)]
struct IntentMessage<'a> {
    intent: &'a Intent,
    value: &'a [u8],
}

fn derive_account_address_from_public_key(public_key_bytes: Vec<u8>, scheme_flag: u8) -> Vec<u8> {
    // Create auth key by prepending scheme flag
    let mut auth_key = vec![scheme_flag];
    auth_key.extend(&public_key_bytes);

    // Compute blake2b hash
    let hash = blake2b(32, &[], &auth_key).as_bytes().to_vec();

    // Take all 32 bytes
    let sui_address: Vec<u8> = hash.to_vec();

    // Convert the address bytes to a hex string with "0x" prefix
    let mut sui_account_address = b"0x".to_vec();

    // Convert each byte to hex and append to result
    for byte in sui_address {
        // Convert byte to hex chars
        let high_nibble = if (byte >> 4) < 10 {
            (byte >> 4) + 0x30
        } else {
            (byte >> 4) - 10 + 0x61
        };

        let low_nibble = if (byte & 0xf) < 10 {
            (byte & 0xf) + 0x30
        } else {
            (byte & 0xf) - 10 + 0x61
        };

        sui_account_address.extend_from_slice(&[high_nibble, low_nibble]);
    }

    sui_account_address
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_sui_derivable_account() {
    let swarm = SwarmBuilder::new_local(1).with_aptos().build().await;
    let mut info = swarm.aptos_public_info();

    let function_info = FunctionInfo::new(
        AccountAddress::ONE,
        "sui_derivable_account".to_string(),
        "authenticate".to_string(),
    );

    let account_key = AccountKey::generate(&mut thread_rng());
    let account_public_key = account_key.public_key().to_bytes();

    let sui_account_address = derive_account_address_from_public_key(
      account_public_key.to_vec(),
      0x00, // 0x00 for ED25519
    );

    let domain = "localhost:3001";
    let account_identity: Vec<u8> = bcs::to_bytes(&SuiAbstractPublicKey {
        sui_account_address: sui_account_address.clone(),
        domain: domain.as_bytes().to_vec(),
    })
    .unwrap();

    let account = LocalAccount::new_domain_aa(
        function_info,
        account_identity,
        Arc::new({move |x: &[u8]| {
            let function_name = "0x1::aptos_account::create_account";
            let digest = format!("0x{}", hex::encode(x));

            let message = format!(
                "{} wants you to sign in with your Sui account:\n{}\n\nPlease confirm you explicitly initiated this request from {}. You are approving to execute transaction {} on Aptos blockchain (local).\n\nNonce: {}",
                domain,
                String::from_utf8(sui_account_address.clone()).unwrap(),
                domain,
                function_name,
                digest
            );
            println!("Raw message: {:?}", message);

            // Create Intent message
            let intent = Intent {
                scope: Scope::PersonalMessage as u8,
                version: Version::V0 as u8,
                app_id: AppId::Sui as u8,
            };
            let intent_message = IntentMessage {
                intent: &intent,
                value: message.as_bytes(),
            };

            // Serialize and hash the message
            let bcs_bytes = bcs::to_bytes(&intent_message).unwrap();
            let hash = blake2b(32, &[], &bcs_bytes).as_bytes().to_vec();

            let signature = account_key
                .private_key()
                .sign_arbitrary_message(&hash)
                .to_bytes()
                .to_vec();

            // Create the full signature bytes: [scheme_flag (1 byte) || signature (64 bytes) || public_key (32 bytes)]
            let mut sui_signature = vec![0x00];  // ED25519 scheme
            sui_signature.extend(signature);
            sui_signature.extend(account_public_key.to_vec()); // Skip "0x00" prefix to get raw public key

            let signature = SuiAbstractSignature::MessageV1 {
                signature: sui_signature,
            };
            bcs::to_bytes(&signature).unwrap()
        }
    }),
        0,
    );

    // test some transaction
    let create_txn = account.sign_aa_transaction_with_transaction_builder(
        vec![],
        Some(&info.root_account()),
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_account_create_account(
                AccountAddress::random(),
            )),
    );
    info.client()
        .submit_and_wait(&create_txn)
        .await
        .unwrap_or_else(|_| panic!("aa: {:?}", create_txn));
}
