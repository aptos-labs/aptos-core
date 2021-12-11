// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::ensure;
use diem_operational_tool::json_rpc::JsonRpcClientWrapper;
use diem_sdk::{
    client::Client,
    transaction_builder::TransactionFactory,
    types::{
        ledger_info::LedgerInfoWithSignatures,
        on_chain_config::{ConsensusConfigV2, OnChainConsensusConfig},
    },
};
use forge::{execute_and_wait_transactions, NetworkContext, NetworkTest, NodeExt, Result, Test};
use rand::{
    rngs::{OsRng, StdRng},
    seq::IteratorRandom,
    Rng, SeedableRng,
};
use std::time::Instant;
use tokio::runtime::Runtime;

pub struct ReconfigurationTest;

impl Test for ReconfigurationTest {
    fn name(&self) -> &'static str {
        "reconfiguration-test"
    }
}

impl NetworkTest for ReconfigurationTest {
    fn run<'t>(&self, ctx: &mut NetworkContext<'t>) -> Result<()> {
        let mut rng = StdRng::from_seed(OsRng.gen());
        let client = JsonRpcClientWrapper::new(ctx.swarm().chain_info().json_rpc_url);
        let validator_info = client
            .validator_set(None)
            .expect("Unable to fetch validator set");
        let affected_peer_id = *validator_info[0].account_address();
        let validator_config = client
            .validator_config(affected_peer_id)
            .expect("Unable to fetch validator config");
        let affected_pod_name = std::str::from_utf8(&validator_config.human_name)
            .unwrap()
            .to_string();
        let validator_clients = ctx
            .swarm()
            .validators()
            .map(|n| (n.rest_client(), n.async_json_rpc_client()))
            .collect::<Vec<_>>();
        let tx_factory = TransactionFactory::new(ctx.swarm().chain_info().chain_id);
        let mut diem_root_account = ctx.swarm().chain_info().root_account;
        let allowed_nonce = 0;
        let rt = Runtime::new()?;
        let (full_node_client, full_node_jsonrpc_client) =
            validator_clients.iter().choose(&mut rng).unwrap();
        let timer = Instant::now();
        let count = 101;

        rt.block_on(async {
            let mut version = expect_epoch(full_node_jsonrpc_client, 0, 1).await.unwrap();
            {
                println!("Remove and add back {}.", affected_pod_name);
                let validator_name = affected_pod_name.as_bytes().to_vec();
                let remove_txn = diem_root_account.sign_with_transaction_builder(
                    tx_factory.remove_validator_and_reconfigure(
                        allowed_nonce,
                        validator_name.clone(),
                        affected_peer_id,
                    ),
                );
                execute_and_wait_transactions(
                    full_node_client,
                    &mut diem_root_account,
                    vec![remove_txn],
                )
                .await
                .unwrap();
                version = expect_epoch(full_node_jsonrpc_client, version, 2)
                    .await
                    .unwrap();
                let add_txn = diem_root_account.sign_with_transaction_builder(
                    tx_factory.add_validator_and_reconfigure(
                        allowed_nonce,
                        validator_name.clone(),
                        affected_peer_id,
                    ),
                );
                execute_and_wait_transactions(
                    full_node_client,
                    &mut diem_root_account,
                    vec![add_txn],
                )
                .await
                .unwrap();
                version = expect_epoch(full_node_jsonrpc_client, version, 3)
                    .await
                    .unwrap();
            }

            {
                println!("Switch decoupled-execution on and off repetitively.");
                let upgrade_config = OnChainConsensusConfig::V2(ConsensusConfigV2 {
                    two_chain: true,
                    decoupled_execution: true,
                    back_pressure_limit: 10,
                    exclude_round: 20,
                });
                let downgrade_config = OnChainConsensusConfig::default();
                for i in 1..count / 2 {
                    let upgrade_txn = diem_root_account.sign_with_transaction_builder(
                        tx_factory.update_diem_consensus_config(
                            allowed_nonce,
                            bcs::to_bytes(&upgrade_config).unwrap(),
                        ),
                    );
                    execute_and_wait_transactions(
                        full_node_client,
                        &mut diem_root_account,
                        vec![upgrade_txn],
                    )
                    .await
                    .unwrap();
                    version = expect_epoch(full_node_jsonrpc_client, version, (i + 1) * 2)
                        .await
                        .unwrap();
                    let downgrade_txn = diem_root_account.sign_with_transaction_builder(
                        tx_factory.update_diem_consensus_config(
                            allowed_nonce,
                            bcs::to_bytes(&downgrade_config).unwrap(),
                        ),
                    );
                    execute_and_wait_transactions(
                        full_node_client,
                        &mut diem_root_account,
                        vec![downgrade_txn],
                    )
                    .await
                    .unwrap();
                    version = expect_epoch(full_node_jsonrpc_client, version, (i + 1) * 2 + 1)
                        .await
                        .unwrap();
                }
            }

            if count % 2 == 1 {
                let magic_number = 42;
                println!("Bump DiemVersion to {}", magic_number);
                let update_txn = diem_root_account.sign_with_transaction_builder(
                    tx_factory.update_diem_version(allowed_nonce, magic_number),
                );
                execute_and_wait_transactions(
                    full_node_client,
                    &mut diem_root_account,
                    vec![update_txn],
                )
                .await
                .unwrap();
                expect_epoch(full_node_jsonrpc_client, version, count + 1)
                    .await
                    .unwrap();
            }
        });

        let elapsed = timer.elapsed();
        ctx.report.report_text(format!(
            "Reconfiguration: total epoch: {} finished in {} seconds",
            count,
            elapsed.as_secs()
        ));

        Ok(())
    }
}

async fn expect_epoch(
    client: &Client,
    known_version: u64,
    expected_epoch: u64,
) -> anyhow::Result<u64> {
    let state_proof = client.get_state_proof(known_version).await?.into_inner();
    let li: LedgerInfoWithSignatures = bcs::from_bytes(&state_proof.ledger_info_with_signatures)?;
    let epoch = li.ledger_info().next_block_epoch();
    ensure!(
        epoch == expected_epoch,
        "Expect epoch {}, actual {}",
        expected_epoch,
        epoch
    );
    println!("Epoch {} is committed", epoch);
    Ok(li.ledger_info().version())
}
