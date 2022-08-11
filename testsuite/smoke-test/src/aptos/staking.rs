// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{account_address::AccountAddress, account_config::CORE_CODE_ADDRESS};
use forge::{AptosPublicInfo, Swarm};

// re-enable after delegation is enabled
#[ignore]
#[tokio::test]
async fn test_staking() {
    let mut swarm = new_local_swarm_with_aptos(1).await;
    let mut info = swarm.aptos_public_info();

    // created by root account
    let mut accounts = vec![];
    for _ in 0..10 {
        let local_account = info.random_account();
        info.create_user_account(local_account.public_key())
            .await
            .unwrap();
        info.mint(local_account.address(), 10000).await.unwrap();
        accounts.push(local_account);
    }
    let validator_addr = AccountAddress::from_hex_literal(
        validator_set(&info).await["active_validators"][0]["addr"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    let txn_factory = info.transaction_factory();
    // let locked_period = 86400 * 2;
    // let current_time = std::time::SystemTime::now()
    //     .duration_since(std::time::UNIX_EPOCH)
    //     .unwrap()
    //     .as_secs()
    //     + locked_period;
    // for account in &mut accounts {
    //     let txn = account.sign_with_transaction_builder(txn_factory.payload(
    //         aptos_stdlib::stake_delegate_stake(validator_addr, 100, current_time),
    //     ));
    //     info.client().submit_and_wait(&txn).await.unwrap();
    // }
    let stake_pool = stake_pool_from_addr(&info, validator_addr).await;
    assert_eq!(
        stake_pool["pending_active"].as_array().unwrap().len(),
        accounts.len()
    );
    // force epoch change to make pending changes take effect
    let txn = info.root_account().sign_with_transaction_builder(
        txn_factory.payload(aptos_stdlib::reconfiguration_force_reconfigure()),
    );
    info.client().submit_and_wait(&txn).await.unwrap();
    let stake_pool = stake_pool_from_addr(&info, validator_addr).await;
    assert_eq!(
        stake_pool["active"].as_array().unwrap().len(),
        accounts.len() + 1
    );
    assert_eq!(
        stake_pool["current_stake"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap(),
        accounts.len() as u64 * 100 + 1
    );
    let voting_power = validator_set(&info).await["active_validators"][0]["voting_power"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert_eq!(voting_power, accounts.len() as u64 * 100 + 1);
}

async fn stake_pool_from_addr(
    info: &AptosPublicInfo<'_>,
    addr: AccountAddress,
) -> serde_json::Value {
    info.client()
        .get_account_resource(addr, "0x1::Stake::StakePool")
        .await
        .unwrap()
        .into_inner()
        .unwrap()
        .data
}

async fn validator_set(info: &AptosPublicInfo<'_>) -> serde_json::Value {
    info.client()
        .get_account_resource(CORE_CODE_ADDRESS, "0x1::Stake::ValidatorSet")
        .await
        .unwrap()
        .into_inner()
        .unwrap()
        .data
}
