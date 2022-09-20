// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_keygen::KeyGen;
use aptos_rest_client::{
    aptos_api_types::{EntryFunctionPayload, TransactionPayload},
    Transaction,
};
use aptos_sdk::{
    crypto::{PrivateKey, SigningKey},
    types::transaction::{authenticator::AuthenticationKey, SignedTransaction},
};
use cached_packages::aptos_stdlib;
use forge::Swarm;

use crate::smoke_test_environment::new_local_swarm_with_aptos;

// TODO: debug me and re-enable the test!
#[ignore]
#[tokio::test]
async fn test_external_transaction_signer() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    // generate key pair
    let mut key_gen = KeyGen::from_os_rng();
    let private_key = key_gen.generate_ed25519_private_key();
    let public_key = private_key.public_key();

    // create transfer parameters
    let sender_auth_key = AuthenticationKey::ed25519(&public_key);
    let sender_address = sender_auth_key.derived_address();
    info.create_user_account(&public_key).await.unwrap();
    // TODO(Gas): double check if this is correct
    info.mint(sender_address, 10_000_000).await.unwrap();

    let receiver = info.random_account();
    info.create_user_account(receiver.public_key())
        .await
        .unwrap();
    // TODO(Gas): double check if this is correct
    info.mint(receiver.address(), 1_000_000).await.unwrap();

    let amount = 1_000_000;
    let test_gas_unit_price = 1;
    // TODO(Gas): double check if this is correct
    let test_max_gas_amount = 1_000_000;

    // prepare transfer transaction
    let test_sequence_number = info
        .client()
        .get_account(sender_address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number;

    let unsigned_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(
            receiver.address(),
            amount,
        ))
        .sender(sender_address)
        .sequence_number(test_sequence_number)
        .max_gas_amount(test_max_gas_amount)
        .gas_unit_price(test_gas_unit_price)
        .build();

    assert_eq!(unsigned_txn.sender(), sender_address);

    // sign the transaction with the private key
    let signature = private_key.sign(&unsigned_txn).unwrap();

    // submit the transaction
    let txn = SignedTransaction::new(unsigned_txn.clone(), public_key, signature);
    info.client().submit_and_wait(&txn).await.unwrap();

    // query the transaction and check it contains the same values as requested
    let txn = info
        .client()
        .get_account_transactions(sender_address, Some(test_sequence_number), Some(1))
        .await
        .unwrap()
        .into_inner()
        .into_iter()
        .next()
        .unwrap();

    match txn {
        Transaction::UserTransaction(user_txn) => {
            assert_eq!(*user_txn.request.sender.inner(), sender_address);
            assert_eq!(user_txn.request.sequence_number.0, test_sequence_number);
            assert_eq!(user_txn.request.gas_unit_price.0, test_gas_unit_price);
            assert_eq!(user_txn.request.max_gas_amount.0, test_max_gas_amount);

            if let TransactionPayload::EntryFunctionPayload(EntryFunctionPayload {
                function: _,
                type_arguments: _,
                arguments,
            }) = user_txn.request.payload
            {
                assert_eq!(
                    arguments
                        .into_iter()
                        .map(|arg| arg.as_str().unwrap().to_owned())
                        .collect::<Vec<String>>(),
                    vec![receiver.address().to_hex_literal(), amount.to_string(),]
                );
            } else {
                panic!("unexpected transaction playload")
            }
        }
        _ => panic!("Query should get user transaction"),
    }
}
