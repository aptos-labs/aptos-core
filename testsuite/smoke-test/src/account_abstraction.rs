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
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_domain_aa() {
    let swarm = SwarmBuilder::new_local(1).with_aptos().build().await;
    let mut info = swarm.aptos_public_info();

    // This method is in genesis now:
    //
    // let register_txn = info.root_account().sign_with_transaction_builder(
    //     info.transaction_factory()
    //         .entry_function(EntryFunction::new(
    //             ModuleId::new(
    //                 AccountAddress::ONE,
    //                 Identifier::new("account_abstraction").unwrap(),
    //             ),
    //             Identifier::new("register_derivable_authentication_function_test_network_only")
    //                 .unwrap(),
    //             vec![],
    //             vec![
    //                 bcs::to_bytes(&AccountAddress::ONE).unwrap(),
    //                 bcs::to_bytes(&"derivable_account_abstraction_ed25519_hex").unwrap(),
    //                 bcs::to_bytes(&"authenticate").unwrap(),
    //             ],
    //         )),
    // );
    // info.client().submit_and_wait(&register_txn).await.unwrap();

    let function_info = FunctionInfo::new(
        AccountAddress::ONE,
        "derivable_account_abstraction_ed25519_hex".to_string(),
        "authenticate".to_string(),
    );

    let account_key = AccountKey::generate(&mut thread_rng());

    let account = LocalAccount::new_domain_aa(
        function_info,
        account_key.public_key().to_bytes().to_vec(),
        Arc::new(move |x: &[u8]| {
            let x_hex = format!("0x{}", hex::encode(x)).into_bytes();
            account_key
                .private_key()
                .sign_arbitrary_message(&x_hex)
                .to_bytes()
                .to_vec()
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