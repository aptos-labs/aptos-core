// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{create_and_fund_account, transfer_coins},
};
use aptos_forge::{NodeExt, Swarm};
use std::sync::Arc;

#[ignore]
#[tokio::test]
async fn test_consensus_only_with_txn_emitter() {
    let mut swarm = SwarmBuilder::new_local(1)
        .with_init_config(Arc::new(|_, config, _| {
            config.consensus.enable_prefix_consensus = true;
        }))
        .with_aptos()
        .build()
        .await;

    let client = swarm.validators().next().unwrap().rest_client();
    let transaction_factory = swarm.chain_info().transaction_factory();

    // Create two accounts through the prefix consensus pipeline
    let mut account_0 = create_and_fund_account(&mut swarm, 1000).await;
    let account_1 = create_and_fund_account(&mut swarm, 1000).await;

    // Execute transfers to verify prefix consensus commits user transactions
    for _ in 0..10 {
        transfer_coins(&client, &transaction_factory, &mut account_0, &account_1, 1).await;
    }
}
