// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use cached_packages::aptos_stdlib;
use forge::Swarm;

use crate::smoke_test_environment::new_local_swarm_with_aptos;

#[tokio::test]
async fn test_account_creation() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    // created by root account
    let mut accounts = vec![];
    for _ in 0..10 {
        let local_account = info.random_account();
        info.create_user_account(local_account.public_key())
            .await
            .unwrap();
        // TODO(Gas): double check this
        info.mint(local_account.address(), 10_000).await.unwrap();
        accounts.push(local_account);
    }
    // created by user account
    for account in &mut accounts {
        let new_account = info.random_account();
        let txn = account.sign_with_transaction_builder(
            info.transaction_factory()
                .payload(aptos_stdlib::account_create_account(new_account.address())),
        );
        info.client().submit_and_wait(&txn).await.unwrap();
    }
    // create and fund
    for mut account in accounts {
        let new_account = info.random_account();
        let txn = account.sign_with_transaction_builder(
            info.transaction_factory()
                .payload(aptos_stdlib::account_transfer(new_account.address(), 5000)),
        );
        info.client().submit_and_wait(&txn).await.unwrap();
        assert_eq!(info.get_balance(new_account.address()).await.unwrap(), 5000);
    }
}
