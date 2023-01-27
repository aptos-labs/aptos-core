// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::NetworkLoadTest;
use aptos::test::CliTestFramework;
use aptos_forge::{NetworkContext, NetworkTest, NodeExt, Result, Swarm, SwarmExt, Test};
use aptos_sdk::{bcs, types::account_config::CORE_CODE_ADDRESS};
use aptos_types::on_chain_config::{ConsensusConfigV1, OnChainConsensusConfig};
use std::{fmt::Write, time::Duration};
use tokio::runtime::Runtime;

pub struct EnableQuorumStoreTest;

impl EnableQuorumStoreTest {
    fn generate_blob(data: &[u8]) -> String {
        let mut buf = String::new();

        write!(buf, "vector[").unwrap();
        for (i, b) in data.iter().enumerate() {
            if i % 20 == 0 {
                if i > 0 {
                    writeln!(buf).unwrap();
                }
            } else {
                write!(buf, " ").unwrap();
            }
            write!(buf, "{}u8,", b).unwrap();
        }
        write!(buf, "]").unwrap();
        buf
    }
}

impl Test for EnableQuorumStoreTest {
    fn name(&self) -> &'static str {
        "enable quorum store test"
    }
}

impl NetworkLoadTest for EnableQuorumStoreTest {
    fn test(&self, swarm: &mut dyn Swarm, duration: Duration) -> Result<()> {
        let faucet_endpoint: reqwest::Url = "http://localhost:8081".parse().unwrap();
        // Connect the operator tool to the node's JSON RPC API
        let rest_client = swarm.validators().next().unwrap().rest_client();
        let runtime = Runtime::new().unwrap();
        let mut cli = runtime.block_on(async {
            CliTestFramework::new(
                swarm.validators().next().unwrap().rest_api_endpoint(),
                faucet_endpoint,
                /*num_cli_accounts=*/ 0,
            )
            .await
        });
        let root_cli_index = cli.add_account_with_address_to_cli(
            swarm.root_key(),
            swarm.chain_info().root_account().address(),
        );

        runtime.block_on(async {
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
                Self::generate_blob(&bcs::to_bytes(&new_consensus_config).unwrap())
            );
            cli.run_script(root_cli_index, &update_consensus_config_script)
                .await
                .unwrap();

        });
        Ok(())
    }
}

impl NetworkTest for EnableQuorumStoreTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx)
    }
}
