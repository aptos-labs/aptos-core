// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_rest_client::Client;
use diem_sdk::types::account_config::testnet_dd_account_address;
use forge::{PublicUsageContext, PublicUsageTest, Result, Test};

pub struct GetIndex;

impl Test for GetIndex {
    fn name(&self) -> &'static str {
        "api::get-index"
    }
}

impl PublicUsageTest for GetIndex {
    fn run<'t>(&self, ctx: &mut PublicUsageContext<'t>) -> Result<()> {
        let resp = reqwest::blocking::get(ctx.rest_api_url().to_owned())?;
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

impl PublicUsageTest for BasicClient {
    fn run<'t>(&self, ctx: &mut PublicUsageContext<'t>) -> Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(self.async_run(ctx))
    }
}

impl BasicClient {
    async fn async_run(&self, ctx: &mut PublicUsageContext<'_>) -> Result<()> {
        let client = Client::new(reqwest::Url::parse(ctx.rest_api_url()).unwrap());
        client.get_ledger_information().await?;

        let mut account1 = ctx.random_account();
        ctx.create_parent_vasp_account(account1.authentication_key())
            .await?;
        ctx.fund(account1.address(), 10).await?;
        let account2 = ctx.random_account();
        ctx.create_parent_vasp_account(account2.authentication_key())
            .await?;
        ctx.fund(account2.address(), 10).await?;

        let tx = account1.sign_with_transaction_builder(ctx.transaction_factory().peer_to_peer(
            diem_sdk::transaction_builder::Currency::XUS,
            account2.address(),
            1,
        ));
        let pending_txn = client.submit(&tx).await.unwrap().into_inner();

        client.wait_for_transaction(&pending_txn).await.unwrap();

        client
            .get_transaction(pending_txn.hash.into())
            .await
            .unwrap();

        client
            .get_account_resources(testnet_dd_account_address())
            .await
            .unwrap();

        client.get_transactions(None, None).await.unwrap();

        Ok(())
    }
}
