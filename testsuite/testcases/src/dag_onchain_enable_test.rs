// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{generate_onchain_config_blob, NetworkLoadTest};
use anyhow::Ok;
use velor::test::CliTestFramework;
use velor_forge::{NetworkContextSynchronizer, NetworkTest, NodeExt, SwarmExt, Test};
use velor_sdk::bcs;
use velor_types::{
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::{
        ConsensusAlgorithmConfig, DagConsensusConfigV1, OnChainConsensusConfig, ValidatorTxnConfig,
        DEFAULT_WINDOW_SIZE,
    },
};
use async_trait::async_trait;
use log::info;
use std::{sync::Arc, time::Duration};

const MAX_NODE_LAG_SECS: u64 = 360;

pub struct DagOnChainEnableTest {}

impl Test for DagOnChainEnableTest {
    fn name(&self) -> &'static str {
        "dag reconfig enable test"
    }
}

#[async_trait]
impl NetworkLoadTest for DagOnChainEnableTest {
    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn velor_forge::Swarm>>>,
        _report: &mut velor_forge::TestReport,
        duration: std::time::Duration,
    ) -> anyhow::Result<()> {
        let faucet_endpoint: reqwest::Url = "http://localhost:8081".parse().unwrap();
        let (rest_client, rest_api_endpoint) = {
            let swarm = swarm.read().await;
            let first_validator = swarm.validators().next().unwrap();
            let rest_client = first_validator.rest_client();
            let rest_api_endpoint = first_validator.rest_api_endpoint();
            (rest_client, rest_api_endpoint)
        };
        let mut cli = CliTestFramework::new(
            rest_api_endpoint,
            faucet_endpoint,
            /*num_cli_accounts=*/ 0,
        )
        .await;

        tokio::time::sleep(duration / 3).await;

        let root_cli_index = {
            let root_account = swarm.read().await.chain_info().root_account();
            cli.add_account_with_address_to_cli(
                root_account.private_key().clone(),
                root_account.address(),
            )
        };

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

        assert!(matches!(
            current_consensus_config,
            OnChainConsensusConfig::V4 { .. }
        ));

        // Change to V4
        let new_consensus_config = OnChainConsensusConfig::V4 {
            alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
            vtxn: ValidatorTxnConfig::default_disabled(),
            window_size: DEFAULT_WINDOW_SIZE,
        };

        let update_consensus_config_script = format!(
            r#"
    script {{
        use velor_framework::velor_governance;
        use velor_framework::consensus_config;
        fun main(core_resources: &signer) {{
            let framework_signer = velor_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
            let config_bytes = {};
            consensus_config::set(&framework_signer, config_bytes);
        }}
    }}
    "#,
            generate_onchain_config_blob(&bcs::to_bytes(&new_consensus_config).unwrap())
        );

        cli.run_script_with_default_framework(root_cli_index, &update_consensus_config_script)
            .await?;

        tokio::time::sleep(duration / 3).await;

        let root_cli_index = {
            let root_account = swarm.read().await.chain_info().root_account();
            cli.add_account_with_address_to_cli(
                root_account.private_key().clone(),
                root_account.address(),
            )
        };

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

        assert!(matches!(
            current_consensus_config,
            OnChainConsensusConfig::V4 { .. }
        ));

        // Change to DAG
        let new_consensus_config = OnChainConsensusConfig::V4 {
            alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
            vtxn: ValidatorTxnConfig::default_disabled(),
            window_size: DEFAULT_WINDOW_SIZE,
        };

        let update_consensus_config_script = format!(
            r#"
    script {{
        use velor_framework::velor_governance;
        use velor_framework::consensus_config;
        fun main(core_resources: &signer) {{
            let framework_signer = velor_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
            let config_bytes = {};
            consensus_config::set(&framework_signer, config_bytes);
        }}
    }}
    "#,
            generate_onchain_config_blob(&bcs::to_bytes(&new_consensus_config).unwrap())
        );

        cli.run_script_with_default_framework(root_cli_index, &update_consensus_config_script)
            .await?;

        let initial_consensus_config = current_consensus_config;

        tokio::time::sleep(duration / 3).await;

        let root_cli_index = {
            let root_account = swarm.read().await.chain_info().root_account();
            cli.add_account_with_address_to_cli(
                root_account.private_key().clone(),
                root_account.address(),
            )
        };

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

        assert!(matches!(
            current_consensus_config,
            OnChainConsensusConfig::V4 { .. }
        ));

        // Change back to initial
        let update_consensus_config_script = format!(
            r#"
    script {{
        use velor_framework::velor_governance;
        use velor_framework::consensus_config;
        fun main(core_resources: &signer) {{
            let framework_signer = velor_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
            let config_bytes = {};
            consensus_config::set(&framework_signer, config_bytes);
        }}
    }}
    "#,
            generate_onchain_config_blob(&bcs::to_bytes(&initial_consensus_config).unwrap())
        );

        cli.run_script_with_default_framework(root_cli_index, &update_consensus_config_script)
            .await?;

        // Wait for all nodes to synchronize and stabilize.
        info!("Waiting for the validators to be synchronized.");
        swarm
            .read()
            .await
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
            .await?;

        Ok(())
    }
}

#[async_trait]
impl NetworkTest for DagOnChainEnableTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}
