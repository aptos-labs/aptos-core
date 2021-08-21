// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use std::time::{Duration, Instant};

use diem_config::config::NodeConfig;
use diem_sdk::{client::views::AmountView, transaction_builder::Currency, types::LocalAccount};
use forge::{NetworkContext, NetworkTest, NodeExt, Result, Test};

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
        let client = fullnode.json_rpc_client();

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

        loop {
            if client
                .get_account(account1.address())?
                .into_inner()
                .is_some()
            {
                println!("hello");
                break;
            }
        }

        let txn = account1.sign_with_transaction_builder(factory.peer_to_peer(
            Currency::XUS,
            account2.address(),
            10,
        ));

        client.submit(&txn)?;
        client.wait_for_signed_transaction(&txn, None, None)?;

        assert_eq!(
            vec![AmountView {
                amount: 90,
                currency: "XUS".to_string()
            }],
            client
                .get_account(account1.address())?
                .into_inner()
                .unwrap()
                .balances
        );

        Ok(())
    }
}
