// SPDX-License-Identifier: Apache-2.0

use forge::{AptosContext, AptosTest, Result, Test};

pub struct AccountCreation;

impl Test for AccountCreation {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::account-creation"
    }
}

#[async_trait::async_trait]
impl AptosTest for AccountCreation {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        for _ in 0..10 {
            let local_account = ctx.random_account();
            ctx.create_user_account(local_account.authentication_key())
                .await?;
        }
        Ok(())
    }
}
