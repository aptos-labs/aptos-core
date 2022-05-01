// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use anyhow::bail;
use aptos_config::config::NodeConfig;
use aptos_rest_client::Client as RestClient;
use aptos_sdk::types::LocalAccount;
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use forge::{NetworkContext, NetworkTest, NodeExt, Result, Test};
use tokio::runtime::Runtime;

#[derive(Debug)]
pub struct LaunchFullnode;

impl Test for LaunchFullnode {
    fn name(&self) -> &'static str {
        "smoke-test:launch-fullnode"
    }
}

impl NetworkTest for LaunchFullnode {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let runtime = Runtime::new().unwrap();
        runtime.block_on(self.async_run(ctx))
    }
}

impl LaunchFullnode {
    async fn async_run(&self, ctx: &mut NetworkContext<'_>) -> Result<()> {
        let version = ctx.swarm().versions().max().unwrap();
        let fullnode_peer_id = ctx
            .swarm()
            .add_full_node(&version, NodeConfig::default_for_public_full_node())?;

        let fullnode = ctx.swarm().full_node_mut(fullnode_peer_id).unwrap();
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(10))
            .await?;

        let client = fullnode.rest_client();

        let mut account1 = LocalAccount::generate(ctx.core().rng());
        let account2 = LocalAccount::generate(ctx.core().rng());

        let mut chain_info = ctx.swarm().chain_info().into_aptos_public_info();
        let factory = chain_info.transaction_factory();
        chain_info
            .create_user_account(account1.public_key())
            .await?;
        chain_info.mint(account1.address(), 1000).await?;
        chain_info
            .create_user_account(account2.public_key())
            .await?;

        wait_for_account(&client, account1.address()).await?;

        let txn = account1.sign_with_transaction_builder(factory.payload(
            aptos_stdlib::encode_test_coin_transfer(account2.address(), 10),
        ));

        client.submit_and_wait(&txn).await?;
        let balance = client
            .get_account_balance(account2.address())
            .await?
            .into_inner();

        assert_eq!(balance.get(), 10);

        Ok(())
    }
}

async fn wait_for_account(client: &RestClient, address: AccountAddress) -> Result<()> {
    const DEFAULT_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();
    while start.elapsed() < DEFAULT_WAIT_TIMEOUT {
        if client.get_account(address).await.is_ok() {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    bail!("wait for account(address={}) timeout", address,)
}
