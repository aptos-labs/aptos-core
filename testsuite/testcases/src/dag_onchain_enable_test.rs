// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{generate_onchain_config_blob, NetworkLoadTest};
use anyhow::Ok;
use aptos::test::CliTestFramework;
use aptos_forge::{NetworkTest, NodeExt, SwarmExt, Test};
use aptos_logger::info;
use aptos_sdk::bcs;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::{
        ConsensusAlgorithmConfig, DagConsensusConfigV1, OnChainConsensusConfig, ValidatorTxnConfig,
    },
};
use std::time::Duration;
use tokio::runtime::Runtime;

const MAX_NODE_LAG_SECS: u64 = 360;

pub struct DagOnChainEnableTest {}

impl Test for DagOnChainEnableTest {
    fn name(&self) -> &'static str {
        "dag reconfig enable test"
    }
}

impl NetworkLoadTest for DagOnChainEnableTest {
    fn test(
        &self,
        swarm: &mut dyn aptos_forge::Swarm,
        _report: &mut aptos_forge::TestReport,
        duration: std::time::Duration,
    ) -> anyhow::Result<()> {
        let runtime = Runtime::new().unwrap();

        let faucet_endpoint: reqwest::Url = "http://localhost:8081".parse().unwrap();
        let rest_client = swarm.validators().next().unwrap().rest_client();

        let mut cli = runtime.block_on(async {
            CliTestFramework::new(
                swarm.validators().next().unwrap().rest_api_endpoint(),
                faucet_endpoint,
                /*num_cli_accounts=*/ 0,
            )
            .await
        });

        std::thread::sleep(duration / 2);

        runtime.block_on(async {

            let root_cli_index = cli.add_account_with_address_to_cli(
                swarm.chain_info().root_account().private_key().clone(),
                swarm.chain_info().root_account().address(),
            );

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

            assert!(matches!(current_consensus_config, OnChainConsensusConfig::V2(_)));

            // Change to V2
            let new_consensus_config = OnChainConsensusConfig::V3 {
                alg: ConsensusAlgorithmConfig::DAG(DagConsensusConfigV1::default()),
                vtxn: ValidatorTxnConfig::default_disabled(),
            };

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
                generate_onchain_config_blob(&bcs::to_bytes(&new_consensus_config).unwrap())
            );

            cli.run_script_with_default_framework(root_cli_index, &update_consensus_config_script)
                .await
        })?;

        std::thread::sleep(duration / 2);

        // Wait for all nodes to synchronize and stabilize.
        info!("Waiting for the validators to be synchronized.");
        runtime.block_on(async {
            swarm
                .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
                .await
        })?;

        Ok(())
    }
}

impl NetworkTest for DagOnChainEnableTest {
    fn run(&self, ctx: &mut aptos_forge::NetworkContext<'_>) -> anyhow::Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
