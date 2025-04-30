// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus::helpers::generate_traffic_and_assert_committed,
    smoke_test_environment::SwarmBuilder, txn_emitter::generate_traffic,
    utils::update_consensus_config,
};
use aptos_consensus::QUORUM_STORE_DB_NAME;
use aptos_forge::{
    args::TransactionTypeArg, reconfig, wait_for_all_nodes_to_catchup, NodeExt,
    ReplayProtectionType, Swarm, SwarmExt, TransactionType,
};
use aptos_logger::info;
use aptos_rest_client::Client;
use aptos_types::on_chain_config::{ConsensusConfigV1, OnChainConsensusConfig};
use std::{fs, sync::Arc, time::Duration};

const MAX_WAIT_SECS: u64 = 60;

// TODO: remove when quorum store becomes the in-code default
#[tokio::test]
async fn test_onchain_config_quorum_store_enabled_and_disabled() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_aptos()
        // Start with V1
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.consensus_config =
                OnChainConsensusConfig::V1(ConsensusConfigV1::default())
        }))
        .build_with_cli(0)
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    generate_traffic_and_assert_committed(&mut swarm, &validator_peer_ids, Duration::from_secs(5))
        .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    for _ in 0..5 {
        let root_cli_index = cli.add_account_with_address_to_cli(
            swarm.root_key(),
            swarm.chain_info().root_account().address(),
        );
        let rest_client = swarm.validators().next().unwrap().rest_client();

        let current_consensus_config =
            crate::utils::get_current_consensus_config(&rest_client).await;
        let inner = match current_consensus_config {
            OnChainConsensusConfig::V1(inner) => inner,
            OnChainConsensusConfig::V2(_) => panic!("Unexpected V2 config"),
            _ => unimplemented!(),
        };
        // Change to V2
        let new_consensus_config = OnChainConsensusConfig::V2(ConsensusConfigV1 { ..inner });
        update_consensus_config(&cli, root_cli_index, new_consensus_config).await;

        generate_traffic_and_assert_committed(
            &mut swarm,
            &validator_peer_ids,
            Duration::from_secs(5),
        )
        .await;

        swarm
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();

        let current_consensus_config =
            crate::utils::get_current_consensus_config(&rest_client).await;
        let inner = match current_consensus_config {
            OnChainConsensusConfig::V1(_) => panic!("Unexpected V1 config"),
            OnChainConsensusConfig::V2(inner) => inner,
            _ => unimplemented!(),
        };

        // Disaster rollback to V1
        let new_consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1 { ..inner });
        update_consensus_config(&cli, root_cli_index, new_consensus_config).await;

        generate_traffic_and_assert_committed(
            &mut swarm,
            &validator_peer_ids,
            Duration::from_secs(5),
        )
        .await;

        swarm
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
            .await
            .unwrap();
    }
}

/// Checks progress even if half the nodes are not actually writing batches to the DB.
/// Shows that remote reading of batches is working.
/// Note this is more than expected (f) byzantine behavior.
#[tokio::test]
async fn test_remote_batch_reads() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        // TODO: remove when quorum store becomes the in-code default
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.consensus_config =
                OnChainConsensusConfig::V2(ConsensusConfigV1::default())
        }))
        .build()
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    for peer_id in validator_peer_ids.iter().take(2) {
        let validator_client = swarm.validator(*peer_id).unwrap().rest_client();
        validator_client
            .set_failpoint("quorum_store::save".to_string(), "return".to_string())
            .await
            .unwrap();
    }

    generate_traffic_and_assert_committed(
        &mut swarm,
        &[validator_peer_ids[2]],
        Duration::from_secs(20),
    )
    .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
}

async fn test_batch_id_on_restart(do_wipe_db: bool) {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        // TODO: remove when quorum store becomes the in-code default
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.consensus_config =
                OnChainConsensusConfig::V2(ConsensusConfigV1::default())
        }))
        .build()
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let node_to_restart = validator_peer_ids[0];

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    generate_traffic_and_assert_committed(&mut swarm, &[node_to_restart], Duration::from_secs(20))
        .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    info!("restart node 0, db intact");
    swarm
        .validator_mut(node_to_restart)
        .unwrap()
        .restart()
        .await
        .unwrap();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    generate_traffic_and_assert_committed(&mut swarm, &[node_to_restart], Duration::from_secs(20))
        .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    info!("stop node 0");
    swarm.validator_mut(node_to_restart).unwrap().stop();
    if do_wipe_db {
        let node0_config = swarm.validator(node_to_restart).unwrap().config().clone();
        let db_dir = node0_config.storage.dir();
        let quorum_store_db_dir = db_dir.join(QUORUM_STORE_DB_NAME);
        info!(
            "wipe only quorum store db: {}",
            quorum_store_db_dir.display()
        );
        fs::remove_dir_all(quorum_store_db_dir).unwrap();
    } else {
        info!("don't do anything to quorum store db");
    }
    info!("start node 0");
    swarm
        .validator_mut(node_to_restart)
        .unwrap()
        .start()
        .unwrap();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    info!("generate traffic");
    generate_traffic_and_assert_committed(&mut swarm, &[node_to_restart], Duration::from_secs(20))
        .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
}

/// Checks that a validator can still get signatures on batches on restart when the db is intact.
#[tokio::test]
async fn test_batch_id_on_restart_same_db() {
    test_batch_id_on_restart(false).await;
}

/// Checks that a validator can still get signatures on batches even if its db is reset (e.g.,
/// the disk failed, or the validator had to be moved to another node).
#[tokio::test]
async fn test_batch_id_on_restart_wiped_db() {
    test_batch_id_on_restart(true).await;
}

#[tokio::test]
async fn test_swarm_with_bad_non_qs_node() {
    let mut swarm = SwarmBuilder::new_local(4)
        .with_aptos()
        .with_init_config(Arc::new(|_, conf, _| {
            conf.api.failpoints_enabled = true;
        }))
        // TODO: remove when quorum store becomes the in-code default
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.consensus_config =
                OnChainConsensusConfig::V2(ConsensusConfigV1::default())
        }))
        .build()
        .await;
    let validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    let dishonest_peer_id = validator_peer_ids[0];
    let honest_peers: Vec<(String, Client)> = swarm
        .validators()
        .skip(1)
        .map(|node| (node.name().to_string(), node.rest_client()))
        .collect();
    let transaction_factory = swarm.chain_info().transaction_factory();

    let rest_client = swarm
        .validator(validator_peer_ids[1])
        .unwrap()
        .rest_client();

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    generate_traffic_and_assert_committed(
        &mut swarm,
        &validator_peer_ids[1..],
        Duration::from_secs(20),
    )
    .await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    let non_qs_validator_client = swarm.validator(dishonest_peer_id).unwrap().rest_client();
    non_qs_validator_client
        .set_failpoint(
            "consensus::start_new_epoch::disable_qs".to_string(),
            "return".to_string(),
        )
        .await
        .unwrap();

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    wait_for_all_nodes_to_catchup(&honest_peers, Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    info!("generate traffic");
    let tx_stat = tokio::time::timeout(
        Duration::from_secs(60),
        generate_traffic(
            &mut swarm,
            &[dishonest_peer_id],
            Duration::from_secs(20),
            1,
            vec![vec![
                (
                    TransactionTypeArg::CoinTransfer.materialize_default(),
                    ReplayProtectionType::SequenceNumber,
                    70,
                ),
                (
                    TransactionTypeArg::AccountGeneration.materialize_default(),
                    ReplayProtectionType::SequenceNumber,
                    20,
                ),
            ]],
        ),
    )
    .await;
    assert!(tx_stat.is_err() || tx_stat.is_ok_and(|result| result.is_err()));

    generate_traffic_and_assert_committed(
        &mut swarm,
        &validator_peer_ids[1..],
        Duration::from_secs(20),
    )
    .await;

    wait_for_all_nodes_to_catchup(&honest_peers, Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
}
