// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};

pub struct MintTransfer;

impl Test for MintTransfer {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::mint-transfer"
    }
}

#[async_trait::async_trait]
impl AptosTest for MintTransfer {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let mut account1 = ctx.random_account();
        ctx.create_user_account(account1.public_key()).await?;
        let account2 = ctx.random_account();
        ctx.create_user_account(account2.public_key()).await?;

        ctx.mint(account1.address(), 1000).await?;

        let transfer_txn =
            account1.sign_with_transaction_builder(ctx.transaction_factory().payload(
                aptos_stdlib::encode_transfer_script_function(account2.address(), 400),
            ));
        ctx.client().submit_and_wait(&transfer_txn).await?;

        Ok(())
    }
}
