// SPDX-License-Identifier: Apache-2.0

use diem_transaction_builder::aptos_stdlib;
use diem_types::transaction::authenticator::AuthenticationKeyPreimage;
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
        // created by root account
        let mut accounts = vec![];
        for _ in 0..10 {
            let local_account = ctx.random_account();
            ctx.create_user_account(local_account.public_key()).await?;
            accounts.push(local_account);
        }
        // created by user account
        for mut account in accounts {
            let new_account = ctx.random_account();
            let preimage = AuthenticationKeyPreimage::ed25519(new_account.public_key());
            let txn = account.sign_with_transaction_builder(ctx.transaction_factory().payload(
                aptos_stdlib::encode_create_account_script_function(
                    new_account.address(),
                    preimage.into_vec(),
                ),
            ));
            ctx.client().submit_and_wait(&txn).await?;
        }
        Ok(())
    }
}
