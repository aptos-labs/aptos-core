// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use anyhow::bail;
use diem_config::config::NodeConfig;
use diem_rest_client::Client as RestClient;
use diem_sdk::{transaction_builder::Currency, types::LocalAccount};
use diem_types::account_address::AccountAddress;
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
        let version = ctx.swarm().versions().max().unwrap();
        let fullnode_peer_id = ctx
            .swarm()
            .add_full_node(&version, NodeConfig::default_for_public_full_node())?;

        let fullnode = ctx.swarm().full_node_mut(fullnode_peer_id).unwrap();
        fullnode.wait_until_healthy(Instant::now() + Duration::from_secs(10))?;
        let client = fullnode.rest_client();

        let factory = ctx.swarm().chain_info().transaction_factory();
        let mut account1 = LocalAccount::generate(ctx.core().rng());
        let account2 = LocalAccount::generate(ctx.core().rng());
        ctx.swarm()
            .chain_info()
            .create_parent_vasp_account(Currency::XUS, account1.authentication_key())?;
        ctx.swarm()
            .chain_info()
            .fund(Currency::XUS, account1.address(), 100)?;
        ctx.swarm()
            .chain_info()
            .create_parent_vasp_account(Currency::XUS, account2.authentication_key())?;

        let runtime = Runtime::new().unwrap();
        runtime.block_on(wait_for_account(&client, account1.address()))?;

        let txn = account1.sign_with_transaction_builder(factory.peer_to_peer(
            Currency::XUS,
            account2.address(),
            10,
        ));

        runtime.block_on(client.submit_and_wait(&txn))?;
        let balances = runtime
            .block_on(client.get_account_balances(account1.address()))?
            .into_inner();

        assert_eq!(
            vec![(90, "XUS".to_string())],
            balances
                .into_iter()
                .map(|b| (b.amount, b.currency_code()))
                .collect::<Vec<(u64, String)>>()
        );

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
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    bail!("wait for account(address={}) timeout", address,)
}
