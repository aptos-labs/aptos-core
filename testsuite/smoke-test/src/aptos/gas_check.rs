// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_gas::{AptosGasParameters, InitialGasSchedule, ToOnChainGasSchedule};
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

        let transfer_txn = account1.sign_with_transaction_builder(
            ctx.aptos_transaction_factory()
                .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 100)),
        );
        // fail due to not enough gas
        let err = ctx
            .client()
            .submit_and_wait(&transfer_txn)
            .await
            .unwrap_err();
        assert!(format!("{:?}", err).contains("INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE"));

        // TODO(Gas): double check this
        ctx.mint(account1.address(), 1_000).await?;
        ctx.mint(account2.address(), 1_000).await?;

        let transfer_too_much = account2.sign_with_transaction_builder(
            // TODO(Gas): double check this
            ctx.aptos_transaction_factory()
                .payload(aptos_stdlib::aptos_coin_transfer(account1.address(), 1_000)),
        );

        let err = ctx
            .client()
            .submit_and_wait(&transfer_too_much)
            .await
            .unwrap_err();
        assert!(format!("{:?}", err).contains("execution failed"));

        // succeed with enough gas
        ctx.client().submit_and_wait(&transfer_txn).await?;

        // update to allow 0 gas unit price
        let mut gas_params = AptosGasParameters::initial();
        gas_params.txn.min_price_per_gas_unit = 0;
        let gas_schedule_blob = bcs::to_bytes(&gas_params.to_on_chain_gas_schedule())
            .expect("failed to serialize gas parameters");

        let txn_factory = ctx.aptos_transaction_factory();

        let update_txn = ctx
            .root_account()
            .sign_with_transaction_builder(txn_factory.payload(
                aptos_stdlib::gas_schedule_set_gas_schedule(gas_schedule_blob),
            ));
        ctx.client().submit_and_wait(&update_txn).await?;

        let zero_gas_txn = account1.sign_with_transaction_builder(
            ctx.aptos_transaction_factory()
                .payload(aptos_stdlib::aptos_coin_transfer(account2.address(), 100))
                .gas_unit_price(0),
        );
        while ctx.client().get_ledger_information().await?.inner().epoch < 2 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        ctx.client().submit_and_wait(&zero_gas_txn).await?;
        Ok(())
    }
}
