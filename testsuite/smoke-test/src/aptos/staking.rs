// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use aptos_types::{account_address::AccountAddress, account_config::aptos_root_address};
use forge::{AptosContext, AptosTest, Result, Test};

pub struct Staking;

impl Test for Staking {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::staking"
    }
}

#[async_trait::async_trait]
impl AptosTest for Staking {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        // created by root account
        let mut accounts = vec![];
        for _ in 0..10 {
            let local_account = ctx.random_account();
            ctx.create_user_account(local_account.public_key()).await?;
            ctx.mint(local_account.address(), 10000).await?;
            accounts.push(local_account);
        }
        let validator_addr = AccountAddress::from_hex_literal(
            validator_set(ctx).await["active_validators"][0]["addr"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
        let txn_factory = ctx.aptos_transaction_factory();
        // let locked_period = 86400 * 2;
        // let current_time = std::time::SystemTime::now()
        //     .duration_since(std::time::UNIX_EPOCH)
        //     .unwrap()
        //     .as_secs()
        //     + locked_period;
        // for account in &mut accounts {
        //     let txn = account.sign_with_transaction_builder(txn_factory.payload(
        //         aptos_stdlib::encode_stake_delegate_stake(validator_addr, 100, current_time),
        //     ));
        //     ctx.client().submit_and_wait(&txn).await?;
        // }
        let stake_pool = stake_pool_from_addr(ctx, validator_addr).await;
        assert_eq!(
            stake_pool["pending_active"].as_array().unwrap().len(),
            accounts.len()
        );
        // force epoch change to make pending changes take effect
        let txn = ctx.root_account().sign_with_transaction_builder(
            txn_factory.payload(aptos_stdlib::encode_reconfiguration_force_reconfigure()),
        );
        ctx.client().submit_and_wait(&txn).await?;
        let stake_pool = stake_pool_from_addr(ctx, validator_addr).await;
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
        let voting_power = validator_set(ctx).await["active_validators"][0]["voting_power"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        assert_eq!(voting_power, accounts.len() as u64 * 100 + 1);
        Ok(())
    }
}

async fn stake_pool_from_addr(ctx: &AptosContext<'_>, addr: AccountAddress) -> serde_json::Value {
    ctx.client()
        .get_account_resource(addr, "0x1::Stake::StakePool")
        .await
        .unwrap()
        .into_inner()
        .unwrap()
        .data
}

async fn validator_set(ctx: &AptosContext<'_>) -> serde_json::Value {
    ctx.client()
        .get_account_resource(aptos_root_address(), "0x1::Stake::ValidatorSet")
        .await
        .unwrap()
        .into_inner()
        .unwrap()
        .data
}
