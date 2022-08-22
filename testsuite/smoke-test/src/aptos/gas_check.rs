// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule};
use cached_packages::aptos_stdlib;
use forge::Swarm;
use std::time::Duration;

use crate::smoke_test_environment::new_local_swarm_with_aptos;

#[tokio::test]
async fn test_gas_check() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    let mut account1 = info.random_account();
    info.create_user_account(account1.public_key())
        .await
        .unwrap();
    let mut account2 = info.random_account();
    info.create_user_account(account2.public_key())
        .await
        .unwrap();

    let transfer_txn = account1.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 100)),
    );
    // fail due to not enough gas
    let err = info
        .client()
        .submit_and_wait(&transfer_txn)
        .await
        .unwrap_err();
    assert!(format!("{:?}", err).contains("INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE"));

    // TODO(Gas): double check this
    info.mint(account1.address(), 1_000).await.unwrap();
    info.mint(account2.address(), 1_000).await.unwrap();

    let transfer_too_much = account2.sign_with_transaction_builder(
        // TODO(Gas): double check this
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(account1.address(), 1_000)),
    );

    let err = info
        .client()
        .submit_and_wait(&transfer_too_much)
        .await
        .unwrap_err();
    assert!(format!("{:?}", err).contains("execution failed"));

    // succeed with enough gas
    info.client().submit_and_wait(&transfer_txn).await.unwrap();

    // update to allow 0 gas unit price
    let mut gas_params = AptosGasParameters::initial();
    gas_params.txn.min_price_per_gas_unit = 0.into();
    let gas_schedule_blob = bcs::to_bytes(&gas_params.to_on_chain_gas_schedule())
        .expect("failed to serialize gas parameters");

    let txn_factory = info.transaction_factory();

    let update_txn = info
        .root_account()
        .sign_with_transaction_builder(txn_factory.payload(
            aptos_stdlib::gas_schedule_set_gas_schedule(gas_schedule_blob),
        ));
    info.client().submit_and_wait(&update_txn).await.unwrap();

    let zero_gas_txn = account1.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 100))
            .gas_unit_price(0),
    );
    while info
        .client()
        .get_ledger_information()
        .await
        .unwrap()
        .inner()
        .epoch
        < 2
    {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    info.client().submit_and_wait(&zero_gas_txn).await.unwrap();
}
