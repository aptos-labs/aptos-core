// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::SigningKey;
use aptos_forge::Swarm;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_types::function_info::FunctionInfo;
use move_core_types::account_address::AccountAddress;
use rand::thread_rng;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct SIWSAbstractPublicKey {
    base58_public_key: String,
    domain: String,
}

#[derive(Serialize)]
enum SIWSAbstractSignature {
    RawSignature { signature: Vec<u8> },
}

fn bytes_to_base58(bytes: &[u8]) -> String {
    let base58_alphabet = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    let mut result = Vec::new();
    let mut num = Vec::from(bytes);

    // Handle special case of zero
    if num.is_empty() || num.iter().all(|&x| x == 0) {
        return "1".to_string();
    }

    // Convert to base 58
    while !num.is_empty() && !num.iter().all(|&x| x == 0) {
        let mut remainder = 0u16;
        let mut temp = Vec::new();

        // Perform division on the whole number
        for &digit in &num {
            let current = (remainder << 8) + digit as u16;
            remainder = current % 58;
            let quotient = current / 58;
            if !temp.is_empty() || quotient != 0 {
                temp.push(quotient as u8);
            }
        }

        result.push(base58_alphabet[remainder as usize]);
        num = temp;
    }

    // Add leading zeros
    for &byte in bytes {
        if byte != 0 {
            break;
        }
        result.push(b'1');
    }

    result.reverse();
    String::from_utf8(result).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_domain_aa() {
    let swarm = SwarmBuilder::new_local(1).with_aptos().build().await;
    let mut info = swarm.aptos_public_info();

    let function_info = FunctionInfo::new(
        AccountAddress::ONE,
        "daa_siws".to_string(),
        "authenticate".to_string(),
    );

    let account_key = AccountKey::generate(&mut thread_rng());
    let base58_public_key = bytes_to_base58(&account_key.public_key().to_bytes());
    let domain = "aptos.com";
    let account_identity = bcs::to_bytes(&SIWSAbstractPublicKey {
        base58_public_key: base58_public_key.clone(),
        domain: domain.to_string(),
    })
    .unwrap();

    let account = LocalAccount::new_domain_aa(
        function_info,
        account_identity,
        Arc::new(move |x: &[u8]| {
            let function_name = "0x1::aptos_account::create_account";
            let digest = format!("0x{}", hex::encode(x));
            let message = format!(
                "{} wants you to sign in with your Solana account:\n{}\n\nTo execute transaction {} on Aptos blockchain (local).\n\nNonce: {}",
                domain,
                base58_public_key,
                function_name,
                digest
            );
            let signature_bytes = account_key
                .private_key()
                .sign_arbitrary_message(&message.into_bytes())
                .to_bytes()
                .to_vec();
            let signature = SIWSAbstractSignature::RawSignature {
                signature: signature_bytes.to_vec(),
            };
            bcs::to_bytes(&signature).unwrap()
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
