// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_faucet::{delegate_account, Service};
use aptos_rest_client::Client;
use aptos_sdk::types::{AccountKey, LocalAccount};
use aptos_transaction_builder::aptos_stdlib;
use forge::{AptosContext, AptosTest, Result, Test};
use std::sync::Arc;

pub struct FaucetDelegation;

impl Test for FaucetDelegation {
    fn name(&self) -> &'static str {
        "smoke-test::aptos::faucet-delegation"
    }
}

#[async_trait::async_trait]
impl AptosTest for FaucetDelegation {
    async fn run<'t>(&self, ctx: &mut AptosContext<'t>) -> Result<()> {
        let client = Client::new(reqwest::Url::parse(ctx.url()).unwrap());
        client.get_ledger_information().await?;

        let root_clone = LocalAccount::new(
            ctx.root_account().address(),
            AccountKey::from_private_key(ctx.root_account().private_key().clone()),
            0,
        );

        let service = Arc::new(Service::new(
            ctx.url().to_string(),
            ctx.chain_id(),
            root_clone,
            None,
        ));

        let new_service =
            delegate_account(service.clone(), ctx.url().to_string(), ctx.chain_id(), None).await;

        let old_root_address = ctx.root_account().address();
        let new_account_address = new_service.faucet_account.lock().await.address();
        assert_ne!(
            old_root_address, new_account_address,
            "account was not delegated!"
        );

        let starting_balance = ctx.get_balance(new_account_address).await.unwrap();

        // ensure new account can mint
        let tx = new_service
            .faucet_account
            .lock()
            .await
            .sign_with_transaction_builder(ctx.aptos_transaction_factory().payload(
                aptos_stdlib::encode_mint_script_function(new_account_address, 1_000_000),
            ));

        let pending_txn = client.submit(&tx).await?.into_inner();
        client.wait_for_transaction(&pending_txn).await?;

        let ending_balance = ctx.get_balance(new_account_address).await.unwrap();

        assert!(
            ending_balance > starting_balance,
            "ending balance of {} was not greater than starting balance: {}",
            ending_balance,
            starting_balance
        );

        Ok(())
    }
}
