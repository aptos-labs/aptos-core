// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    aptos_cli::validator::generate_blob, smoke_test_environment::SwarmBuilder,
    txn_emitter::generate_traffic,
};
use aptos_forge::{NodeExt, Swarm, SwarmExt, TransactionType};
use aptos_types::{
    on_chain_config::{ConsensusConfigV1, OnChainConsensusConfig},
    PeerId,
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::{sync::Arc, time::Duration};

const MAX_WAIT_SECS: u64 = 60;

async fn generate_traffic_and_assert_committed(swarm: &mut dyn Swarm, nodes: &[PeerId]) {
    let rest_client = swarm.validator(nodes[0]).unwrap().rest_client();

    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    let txn_stat = generate_traffic(swarm, nodes, Duration::from_secs(20), 1, vec![vec![
        (
            TransactionType::CoinTransfer {
                invalid_transaction_ratio: 0,
                sender_use_account_pool: false,
            },
            70,
        ),
        (
            TransactionType::AccountGeneration {
                add_created_accounts_to_pool: true,
                max_account_working_set: 1_000_000,
                creation_balance: 1_000_000,
            },
            20,
        ),
    ]])
    .await
    .unwrap();
    println!("{:?}", txn_stat.rate());
    // assert some much smaller number than expected, so it doesn't fail under contention
    assert!(txn_stat.submitted > 30);
    assert!(txn_stat.committed > 30);
}

// TODO: remove when quorum store becomes the in-code default
#[tokio::test]
async fn test_onchain_config_quorum_store_enabled() {
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

    generate_traffic_and_assert_committed(&mut swarm, &validator_peer_ids).await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();

    let root_cli_index = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );

    let rest_client = swarm.validators().next().unwrap().rest_client();
    let current_consensus_config: OnChainConsensusConfig = bcs::from_bytes(
        &rest_client
            .get_account_resource_bcs::<Vec<u8>>(
                CORE_CODE_ADDRESS,
                "0x1::consensus_config::ConsensusConfig",
            )
            .await
            .unwrap()
            .into_inner(),
    )
    .unwrap();

    let inner = match current_consensus_config {
        OnChainConsensusConfig::V1(inner) => inner,
        OnChainConsensusConfig::V2(_) => panic!("Unexpected V2 config"),
    };

    // Change to V2
    let new_consensus_config = OnChainConsensusConfig::V2(ConsensusConfigV1 { ..inner });

    let update_consensus_config_script = format!(
        r#"
    script {{
        use aptos_framework::aptos_governance;
        use aptos_framework::consensus_config;
        fun main(core_resources: &signer) {{
            let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
            let config_bytes = {};
            consensus_config::set(&framework_signer, config_bytes);
        }}
    }}
    "#,
        generate_blob(&bcs::to_bytes(&new_consensus_config).unwrap())
    );
    cli.run_script(root_cli_index, &update_consensus_config_script)
        .await
        .unwrap();

    generate_traffic_and_assert_committed(&mut swarm, &validator_peer_ids).await;
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

    generate_traffic_and_assert_committed(&mut swarm, &[validator_peer_ids[2]]).await;

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_WAIT_SECS))
        .await
        .unwrap();
}
