// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{
        assert_balance, create_and_fund_account, transfer_coins, MAX_CATCH_UP_WAIT_SECS,
        MAX_CONNECTIVITY_WAIT_SECS, MAX_HEALTHY_WAIT_SECS,
    },
};
use velor_config::config::{NodeConfig, OverrideNodeConfig};
use velor_forge::{NodeExt, Swarm, SwarmExt};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Checks txn goes through consensus even if the local validator is not creating proposals.
/// This behavior should be true with both mempool and quorum store.
#[tokio::test]
async fn test_txn_broadcast() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_velor()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        .build()
        .await;
    let transaction_factory = swarm.chain_info().transaction_factory();
    let version = swarm.versions().max().unwrap();
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    let validator = validator_peer_ids[1];
    let vfn = swarm
        .add_validator_fullnode(
            &version,
            OverrideNodeConfig::new_with_default_base(NodeConfig::get_default_vfn_config()),
            validator,
        )
        .unwrap();

    for fullnode in swarm.full_nodes() {
        fullnode
            .wait_until_healthy(Instant::now() + Duration::from_secs(MAX_HEALTHY_WAIT_SECS))
            .await
            .unwrap();
        fullnode
            .wait_for_connectivity(Instant::now() + Duration::from_secs(MAX_CONNECTIVITY_WAIT_SECS))
            .await
            .unwrap();
    }

    // Setup accounts
    let mut account_0 = create_and_fund_account(&mut swarm, 10).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    // set up vfn_client
    let vfn_client = swarm.full_node(vfn).unwrap().rest_client();

    // set up validator_client. proposals not sent from this validator. txn should still go through.
    let validator_client = swarm.validator(validator).unwrap().rest_client();
    validator_client
        .set_failpoint(
            "consensus::send::proposal".to_string(),
            "return".to_string(),
        )
        .await
        .unwrap();

    // send to validator_client
    let txn = transfer_coins(
        &validator_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&validator_client, &account_0, 9).await;
    assert_balance(&validator_client, &account_1, 11).await;
    vfn_client.wait_for_signed_transaction(&txn).await.unwrap();
    assert_balance(&vfn_client, &account_0, 9).await;
    assert_balance(&vfn_client, &account_1, 11).await;

    // send to vfn_client
    transfer_coins(
        &vfn_client,
        &transaction_factory,
        &mut account_0,
        &account_1,
        1,
    )
    .await;

    assert_balance(&validator_client, &account_0, 8).await;
    assert_balance(&validator_client, &account_1, 12).await;
    assert_balance(&vfn_client, &account_0, 8).await;
    assert_balance(&vfn_client, &account_1, 12).await;
}
