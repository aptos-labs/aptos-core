// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};

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
        let account2 = ctx.random_account();
        ctx.create_user_account(account2.public_key()).await?;

        let transfer_txn = account1.sign_with_transaction_builder(
            ctx.transaction_factory()
                .payload(aptos_stdlib::encode_transfer_script_function(
                    account2.address(),
                    100,
                ))
                .gas_unit_price(1)
                .max_gas_amount(1000),
        );
        // not enough gas
        ctx.client()
            .submit_and_wait(&transfer_txn)
            .await
            .unwrap_err();

        ctx.mint(account1.address(), 1000).await?;

        ctx.client().submit_and_wait(&transfer_txn).await?;
        Ok(())
    }
}
