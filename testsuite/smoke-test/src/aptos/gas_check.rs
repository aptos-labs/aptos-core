// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::move_types::gas_schedule::{GasAlgebra, GasConstants};
use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};
use std::time::Duration;

pub struct GasCheck;

impl Test for GasCheck {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::gas-check"
    }
}

#[async_trait::async_trait]
impl AptosTest for GasCheck {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let mut account1 = ctx.random_account();
        ctx.create_user_account(account1.public_key()).await?;
        let mut account2 = ctx.random_account();
        ctx.create_user_account(account2.public_key()).await?;

        let transfer_txn =
            account1.sign_with_transaction_builder(ctx.aptos_transaction_factory().payload(
                aptos_stdlib::encode_test_coin_transfer(account2.address(), 100),
            ));
        // fail due to not enough gas
        let err = ctx
            .client()
            .submit_and_wait(&transfer_txn)
            .await
            .unwrap_err();
        assert!(format!("{:?}", err).contains("INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE"));

        ctx.mint(account1.address(), 1000).await?;
        ctx.mint(account2.address(), 1000).await?;

        let transfer_too_much =
            account2.sign_with_transaction_builder(ctx.aptos_transaction_factory().payload(
                aptos_stdlib::encode_test_coin_transfer(account1.address(), 1000),
            ));

        let err = ctx
            .client()
            .submit_and_wait(&transfer_too_much)
            .await
            .unwrap_err();
        assert!(format!("{:?}", err).contains("execution failed"));

        // succeed with enough gas
        ctx.client().submit_and_wait(&transfer_txn).await?;

        // update to allow 0 gas unit price
        let gas_constant = GasConstants::default();
        let txn_factory = ctx.aptos_transaction_factory();

        let update_txn = ctx
            .root_account()
            .sign_with_transaction_builder(txn_factory.payload(
                aptos_stdlib::encode_vm_config_set_gas_constants(
                    gas_constant.global_memory_per_byte_cost.get(),
                    gas_constant.global_memory_per_byte_write_cost.get(),
                    gas_constant.min_transaction_gas_units.get(),
                    gas_constant.large_transaction_cutoff.get(),
                    gas_constant.intrinsic_gas_per_byte.get(),
                    gas_constant.maximum_number_of_gas_units.get(),
                    0, // updated value
                    gas_constant.max_price_per_gas_unit.get(),
                    gas_constant.max_transaction_size_in_bytes,
                    gas_constant.gas_unit_scaling_factor,
                    gas_constant.default_account_size.get(),
                ),
            ));
        ctx.client().submit_and_wait(&update_txn).await?;

        let zero_gas_txn = account1.sign_with_transaction_builder(
            ctx.aptos_transaction_factory()
                .payload(aptos_stdlib::encode_test_coin_transfer(
                    account2.address(),
                    100,
                ))
                .gas_unit_price(0),
        );
        while ctx.client().get_ledger_information().await?.inner().epoch < 2 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        ctx.client().submit_and_wait(&zero_gas_txn).await?;
        Ok(())
    }
}
