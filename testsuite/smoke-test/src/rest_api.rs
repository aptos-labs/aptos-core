// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_config::aptos_root_address;
use forge::{AptosContext, AptosTest, Result, Test};

pub struct GetIndex;

impl Test for GetIndex {
    fn name(&self) -> &'static str {
        "api::get-index"
    }
}

#[async_trait::async_trait]
impl AptosTest for GetIndex {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let resp = reqwest::get(ctx.url().to_owned()).await?;
        assert_eq!(reqwest::StatusCode::OK, resp.status());

        Ok(())
    }
}

pub struct BasicClient;

impl Test for BasicClient {
    fn name(&self) -> &'static str {
        "api::basic-client"
    }
}

#[async_trait::async_trait]
impl AptosTest for BasicClient {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = ctx.client();
        client.get_ledger_information().await?;

        let mut account1 = ctx.create_and_fund_user_account(10_000).await?;
        let account2 = ctx.create_and_fund_user_account(10_000).await?;

        let tx = account1.sign_with_transaction_builder(ctx.transaction_factory().payload(
            aptos_stdlib::encode_test_coin_transfer(account2.address(), 1),
        ));
        let pending_txn = client.submit(&tx).await.unwrap().into_inner();

        client.wait_for_transaction(&pending_txn).await.unwrap();

        client
            .get_transaction(pending_txn.hash.into())
            .await
            .unwrap();

        client
            .get_account_resources(aptos_root_address())
            .await
            .unwrap();

        client.get_transactions(None, None).await.unwrap();

        Ok(())
    }
}
