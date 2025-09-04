// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use velor_forge::{NetworkContextSynchronizer, NetworkTest, Result, Test};
use async_trait::async_trait;

pub struct ReconfigurationTest;

impl Test for ReconfigurationTest {
    fn name(&self) -> &'static str {
        "reconfiguration-test"
    }
}

#[async_trait]
impl NetworkTest for ReconfigurationTest {
    async fn run<'a>(&self, _ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        Err(anyhow!("Not supported in velor-framework yet"))
    }
    // TODO(https://github.com/velor-chain/velor-core/issues/317): add back after support those transactions in velor-framework
    //     let rt = Runtime::new()?;
    //
    //     let mut rng = StdRng::from_seed(OsRng.gen());
    //     let client = OperationalTool::new(ctx.swarm().chain_info().rest_api().to_owned());
    //     let validator_info = rt
    //         .block_on(client.validator_set(None))
    //         .expect("Unable to fetch validator set");
    //     let affected_peer_id = *validator_info[0].account_address();
    //     let validator_config = rt
    //         .block_on(client.validator_config(affected_peer_id))
    //         .expect("Unable to fetch validator config");
    //     let affected_pod_name = std::str::from_utf8(&validator_config.human_name)
    //         .unwrap()
    //         .to_string();
    //     let validator_clients = ctx
    //         .swarm()
    //         .validators()
    //         .map(|n| n.rest_client())
    //         .collect::<Vec<_>>();
    //     let tx_factory = TransactionFactory::new(ctx.swarm().chain_info().chain_id);
    //     let mut velor_root_account = ctx.swarm().chain_info().root_account;
    //     let allowed_nonce = 0;
    //     let full_node_client = validator_clients.iter().choose(&mut rng).unwrap();
    //     let timer = Instant::now();
    //     let count = 101;
    //
    //     rt.block_on(async {
    //         expect_epoch(full_node_client, 1).await.unwrap();
    //         {
    //             println!("Remove and add back {}.", affected_pod_name);
    //             let validator_name = affected_pod_name.as_bytes().to_vec();
    //             let remove_txn = velor_root_account.sign_with_transaction_builder(
    //                 tx_factory.remove_validator_and_reconfigure(
    //                     allowed_nonce,
    //                     validator_name.clone(),
    //                     affected_peer_id,
    //                 ),
    //             );
    //             execute_and_wait_transactions(
    //                 full_node_client,
    //                 &mut velor_root_account,
    //                 vec![remove_txn],
    //             )
    //             .await
    //             .unwrap();
    //             expect_epoch(full_node_client, 2).await.unwrap();
    //             let add_txn = velor_root_account.sign_with_transaction_builder(
    //                 tx_factory.add_validator_and_reconfigure(
    //                     allowed_nonce,
    //                     validator_name.clone(),
    //                     affected_peer_id,
    //                 ),
    //             );
    //             execute_and_wait_transactions(
    //                 full_node_client,
    //                 &mut velor_root_account,
    //                 vec![add_txn],
    //             )
    //             .await
    //             .unwrap();
    //             expect_epoch(full_node_client, 3).await.unwrap();
    //         }
    //
    //         {
    //             println!("Switch decoupled-execution on and off repetitively.");
    //             let upgrade_config = OnChainConsensusConfig::V2(ConsensusConfigV2 {
    //                 two_chain: true,
    //                 decoupled_execution: true,
    //                 back_pressure_limit: 10,
    //                 exclude_round: 20,
    //             });
    //             let downgrade_config = OnChainConsensusConfig::default();
    //             for i in 1..count / 2 {
    //                 let upgrade_txn = velor_root_account.sign_with_transaction_builder(
    //                     tx_factory.update_velor_consensus_config(
    //                         allowed_nonce,
    //                         bcs::to_bytes(&upgrade_config).unwrap(),
    //                     ),
    //                 );
    //                 execute_and_wait_transactions(
    //                     full_node_client,
    //                     &mut velor_root_account,
    //                     vec![upgrade_txn],
    //                 )
    //                 .await
    //                 .unwrap();
    //                 expect_epoch(full_node_client, (i + 1) * 2).await.unwrap();
    //                 let downgrade_txn = velor_root_account.sign_with_transaction_builder(
    //                     tx_factory.update_velor_consensus_config(
    //                         allowed_nonce,
    //                         bcs::to_bytes(&downgrade_config).unwrap(),
    //                     ),
    //                 );
    //                 execute_and_wait_transactions(
    //                     full_node_client,
    //                     &mut velor_root_account,
    //                     vec![downgrade_txn],
    //                 )
    //                 .await
    //                 .unwrap();
    //                 expect_epoch(full_node_client, (i + 1) * 2 + 1)
    //                     .await
    //                     .unwrap();
    //             }
    //         }
    //
    //         if count % 2 == 1 {
    //             let magic_number = 42;
    //             println!("Bump Version to {}", magic_number);
    //             let update_txn = velor_root_account.sign_with_transaction_builder(
    //                 tx_factory.update_velor_version(allowed_nonce, magic_number),
    //             );
    //             execute_and_wait_transactions(
    //                 full_node_client,
    //                 &mut velor_root_account,
    //                 vec![update_txn],
    //             )
    //             .await
    //             .unwrap();
    //             expect_epoch(full_node_client, count + 1).await.unwrap();
    //         }
    //     });
    //
    //     let elapsed = timer.elapsed();
    //     ctx.report.report_text(format!(
    //         "Reconfiguration: total epoch: {} finished in {} seconds",
    //         count,
    //         elapsed.as_secs()
    //     ));
    //
    //     Ok(())
    // }
}

// async fn expect_epoch(client: &RestClient, expected_epoch: u64) -> anyhow::Result<()> {
//     let config = client.get_epoch_configuration().await?.into_inner();
//     let next_block_epoch = *config.next_block_epoch.inner();
//     ensure!(
//         next_block_epoch == expected_epoch,
//         "Expect next block epoch {}, actual {}",
//         expected_epoch,
//         next_block_epoch
//     );
//     Ok(())
// }
