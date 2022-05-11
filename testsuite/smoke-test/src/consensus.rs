// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::operational_tooling::launch_swarm_with_op_tool_and_backend;
use aptos_config::config::SecureBackend;
use aptos_secure_storage::{KVStorage, Storage};
use aptos_types::network_address::NetworkAddress;
use forge::NodeExt;
use std::{convert::TryInto, str::FromStr};

#[ignore]
#[tokio::test]
async fn test_consensus_observer_mode_storage_error() {
    let num_nodes = 4;
    let (swarm, op_tool, backend, _) = launch_swarm_with_op_tool_and_backend(num_nodes).await;

    // Kill safety rules storage for validator 1 to ensure it fails on the next epoch change
    let node_config = swarm.validators().nth(1).unwrap().config().clone();
    let safety_rules_storage = match node_config.consensus.safety_rules.backend {
        SecureBackend::OnDiskStorage(config) => SecureBackend::OnDiskStorage(config),
        _ => panic!("On-disk storage is the only backend supported in smoke tests"),
    };
    let mut safety_rules_storage: Storage = (&safety_rules_storage).try_into().unwrap();
    safety_rules_storage.reset_and_clear().unwrap();

    // Force a new epoch by updating validator 0's full node address in the validator config
    let txn_ctx = op_tool
        .set_validator_config(
            None,
            Some(NetworkAddress::from_str("/ip4/10.0.0.16/tcp/80").unwrap()),
            &backend,
            false,
            false,
        )
        .await
        .unwrap();
    assert!(txn_ctx.execution_result.unwrap().success);

    // Rotate validator 0's operator key several different times, each requiring a new transaction
    for _ in 0..5 {
        let (txn_ctx, _) = op_tool.rotate_operator_key(&backend, false).await.unwrap();
        assert!(txn_ctx.execution_result.unwrap().success);
    }

    // Verify validator 1 is still able to stay up to date with validator 0 (despite safety rules failing)
    let client_0 = swarm.validators().next().unwrap().rest_client();
    let sequence_number_0 = client_0
        .get_account(txn_ctx.address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number;
    let client_1 = swarm.validators().nth(1).unwrap().rest_client();
    let sequence_number_1 = client_1
        .get_account(txn_ctx.address)
        .await
        .unwrap()
        .into_inner()
        .sequence_number;
    assert_eq!(sequence_number_0, sequence_number_1);
}

// TODO(https://github.com/aptos-labs/aptos-core/issues/317): add back after support update consensus config in aptos-framework
// #[allow(dead_code)]
// async fn test_onchain_upgrade(new_onfig: OnChainConsensusConfig) {
//     let num_nodes = 4;
//     let (mut swarm, _, _, _) = launch_swarm_with_op_tool_and_backend(num_nodes).await;
//
//     // should work before upgrade.
//     check_create_mint_transfer(&mut swarm).await;
//
//     // send upgrade txn
//     let transaction_factory = swarm.chain_info().transaction_factory();
//     let upgrade_txn = swarm
//         .chain_info()
//         .root_account
//         .sign_with_transaction_builder(
//             transaction_factory.update_aptos_consensus_config(0, bcs::to_bytes(&new_onfig).unwrap()),
//         );
//
//     let client = swarm.validators().next().unwrap().rest_client();
//     client.submit_and_wait(&upgrade_txn).await.unwrap();
//
//     // should work after upgrade.
//     check_create_mint_transfer(&mut swarm).await;
// }
