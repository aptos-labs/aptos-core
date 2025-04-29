// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos_cached_packages::aptos_stdlib;
use aptos_forge::Swarm;
use aptos_types::CoinType;

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_account_auto_creation() {
    let swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    let account1 = info
        .create_and_fund_user_account(100_000_000_000)
        .await
        .unwrap();
    let account2 = info.random_account();

    let migrate_txn = account1.sign_with_transaction_builder(info.transaction_factory().payload(
        aptos_stdlib::coin_migrate_to_fungible_store(aptos_types::AptosCoinType::type_tag()),
    ));
    info.client().submit_and_wait(&migrate_txn).await.unwrap();

    let send_fa_txn = account1.sign_with_transaction_builder(info.transaction_factory().payload(
        aptos_stdlib::aptos_account_fungible_transfer_only(account2.address(), 10_000_000_000),
    ));
    info.client().submit_and_wait(&send_fa_txn).await.unwrap();

    // test account creation
    // account2 should be created automatically by sending this transaction.
    let send_back_fa_txn = account2.sign_with_transaction_builder(
        info.transaction_factory()
            .payload(aptos_stdlib::aptos_account_fungible_transfer_only(
                account1.address(),
                1,
            ))
            .gas_unit_price(1),
    );
    info.client()
        .submit_and_wait(&send_back_fa_txn)
        .await
        .unwrap();

    let seq_num = info
        .client()
        .get_account(account2.address())
        .await
        .unwrap()
        .into_inner()
        .sequence_number;

    assert_eq!(seq_num, 1);
}
