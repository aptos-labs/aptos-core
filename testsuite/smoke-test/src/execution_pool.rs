// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{update_consensus_config, MAX_CATCH_UP_WAIT_SECS},
};
use aptos::test::CliTestFramework;
use aptos_forge::{LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_rest_client::Client;
use aptos_types::on_chain_config::{
    ConsensusAlgorithmConfig, OnChainConsensusConfig, ValidatorTxnConfig,
};
use move_core_types::language_storage::CORE_CODE_ADDRESS;
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;

/// Checks the value of the `window_size` in the [`OnChainConsensusConfig`](OnChainConsensusConfig)
pub async fn assert_on_chain_consensus_config_window_size(
    swarm: &mut LocalSwarm,
    expected_window_size: Option<u64>,
) {
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

    match current_consensus_config {
        OnChainConsensusConfig::V1(_)
        | OnChainConsensusConfig::V2 { .. }
        | OnChainConsensusConfig::V3 { .. } => {
            panic!("Expected OnChainConsensusConfig::V4, but received a different version")
        },
        OnChainConsensusConfig::V4 { window_size, .. } => {
            assert_eq!(window_size, expected_window_size)
        },
        OnChainConsensusConfig::V5 { window_size, .. } => {
            assert_eq!(window_size, expected_window_size)
        },
    }
}

async fn initialize_swarm_with_window(
    window_size: Option<u64>,
) -> (
    LocalSwarm,
    CliTestFramework,
    JoinHandle<anyhow::Result<()>>,
    usize,
    Client,
) {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, conf, _| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 400;
            conf.consensus.quorum_store_poll_time_ms = 100;
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(move |genesis_config| {
            genesis_config.consensus_config = OnChainConsensusConfig::V4 {
                alg: ConsensusAlgorithmConfig::default_for_genesis(),
                vtxn: ValidatorTxnConfig::default_for_genesis(),
                window_size,
            };
        }))
        .with_aptos()
        .build_with_cli(0)
        .await;

    let root_cli_index = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );

    let rest_client = swarm.validators().next().unwrap().rest_client();

    (swarm, cli, _faucet, root_cli_index, rest_client)
}

#[tokio::test]
async fn test_window_size_onchain_config_change() {
    let window_size = Some(4u64);
    let (mut swarm, cli, _faucet, root_cli_index, ..) =
        initialize_swarm_with_window(window_size).await;

    // Make sure that the current consensus config has a window size of 4
    assert_on_chain_consensus_config_window_size(&mut swarm, window_size).await;

    // Update consensus config with a different window_size
    let window_size = Some(8u64);
    let new_consensus_config = OnChainConsensusConfig::V4 {
        alg: ConsensusAlgorithmConfig::default_for_genesis(),
        vtxn: ValidatorTxnConfig::default_for_genesis(),
        window_size,
    };
    update_consensus_config(&cli, root_cli_index, new_consensus_config).await;

    swarm
        .wait_for_all_nodes_to_catchup_to_next(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();

    // Make sure that the current consensus config has a window size of 8
    assert_on_chain_consensus_config_window_size(&mut swarm, window_size).await;
}
