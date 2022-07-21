// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use consensus::analyze_validators::{AnalyzeLeaderSelection, EpochStats};
use core::time;
use itertools::Itertools;
use std::ops::Add;
use std::thread;

use aptos_sdk::move_types::identifier::Identifier;
use forge::{NodeExt, Swarm};
use reqwest::Url;

use crate::{
    smoke_test_environment::new_local_swarm_with_aptos,
    test_utils::{create_and_fund_account, reconfig, transfer_coins},
};
use aptos_api_types::Transaction;
use aptos_rest_client::{
    aptos_api_types::{self, BlockMetadataTransaction, MoveResource, WriteSetChange},
    Client as RestClient,
};

use aptos_types::account_address::AccountAddress;
use aptos_types::account_config::NewBlockEvent;
use std::collections::HashMap;

pub struct FetchMetadata {}

impl FetchMetadata {
    pub fn metadata_to_new_block_event(metadata: &BlockMetadataTransaction) -> NewBlockEvent {
        NewBlockEvent::new(
            *metadata.epoch.inner(),
            *metadata.round.inner(),
            *metadata.height.inner(),
            metadata.previous_block_votes.clone(),
            *metadata.proposer.inner(),
            metadata
                .failed_proposer_indices
                .iter()
                .map(|i| *i as u64)
                .collect(),
            *metadata.timestamp.inner(),
        )
    }

    pub async fn fetch_new_block_events(
        epoch: u64,
        validator_client: &RestClient,
    ) -> Vec<NewBlockEvent> {
        let transactions = validator_client
            .get_transactions(None, Some(1000))
            .await
            .unwrap()
            .into_inner();

        transactions
            .into_iter()
            .filter(|t| matches!(t, Transaction::BlockMetadataTransaction(_)))
            .map(|t| {
                if let Transaction::BlockMetadataTransaction(metadata) = t {
                    FetchMetadata::metadata_to_new_block_event(&metadata)
                } else {
                    panic!();
                }
            })
            .filter(|e| e.epoch() == epoch)
            .collect()
    }
}

async fn print_transactions(validator_client: &RestClient) {
    let transactions = validator_client
        .get_transactions(None, Some(10))
        .await
        .unwrap()
        .into_inner();
    // println!(
    //     "{:?}",
    //     transactions
    //         .into_iter()
    //         .filter(|t| matches!(t, Transaction::BlockMetadataTransaction(_)))
    //         .map(|t| {
    //             if let Transaction::BlockMetadataTransaction(metadata) = t {
    //                 format!("{} {} {} {:?}\n", metadata.epoch, metadata.round, metadata.proposer, metadata.failed_proposer_indices)
    //             } else {
    //                 t.type_str().to_string()
    //             }
    //         })
    //         .collect::<Vec<String>>()
    // );

    for t in transactions {
        if let Transaction::BlockMetadataTransaction(metadata) = t {
            println!(
                "{} {} {} {:?}",
                metadata.epoch, metadata.round, metadata.proposer, metadata.failed_proposer_indices
            );
        }
    }
}

#[ignore]
#[tokio::test]
async fn test_leader_election() {
    let num_nodes = 8;
    let mut swarm = new_local_swarm_with_aptos(num_nodes).await;

    let mut validator_peer_ids = swarm.validators().map(|v| v.peer_id()).collect::<Vec<_>>();
    validator_peer_ids.sort();
    println!("Swarm started for dir {}", swarm.dir().to_string_lossy());
    println!("Validators {:?}", validator_peer_ids);

    let validator_peer_id = validator_peer_ids[0];
    let validator_client = swarm.validator(validator_peer_id).unwrap().rest_client();

    let transaction_factory = swarm.chain_info().transaction_factory();
    let mut account_0 = create_and_fund_account(&mut swarm, 10000).await;
    let account_1 = create_and_fund_account(&mut swarm, 10).await;

    fn analyze(
        validators: &Vec<AccountAddress>,
        result: Vec<NewBlockEvent>,
        epoch: u64,
    ) -> Option<EpochStats> {
        if !validators.is_empty() {
            let events: Vec<NewBlockEvent> =
                result.into_iter().filter(|e| e.epoch() == epoch).collect();
            println!("Analyzing epoch : {}", epoch);
            let stats = AnalyzeLeaderSelection::analyze(events, &validators);
            AnalyzeLeaderSelection::print_table(&stats, None, false);
            Some(stats)
        } else {
            None
        }
    }

    let validator_clients: Vec<_> = (0..validator_peer_ids.len())
        .map(|i| {
            swarm
                .validator(validator_peer_ids[i])
                .unwrap()
                .rest_client()
        })
        .collect();
    // validator_clients[1].set_failpoint("consensus::process_proposal_msg".to_string(), "100%return".to_string()).await.unwrap();
    // validator_clients[2].set_failpoint("consensus::process_proposal_msg".to_string(), "100%return".to_string()).await.unwrap();
    // validator_clients[3].set_failpoint("consensus::process_vote_msg".to_string(), "100%return".to_string()).await.unwrap();

    validator_clients[1]
        .set_failpoint(
            "consensus::send_proposal".to_string(),
            "100%return".to_string(),
        )
        .await
        .unwrap();
    validator_clients[2]
        .set_failpoint("consensus::send_vote".to_string(), "100%return".to_string())
        .await
        .unwrap();
    validator_clients[3]
        .set_failpoint(
            "consensus::send_proposal".to_string(),
            "100%return".to_string(),
        )
        .await
        .unwrap();
    validator_clients[3]
        .set_failpoint("consensus::send_vote".to_string(), "100%return".to_string())
        .await
        .unwrap();

    for i in 1..8 {
        println!("cycle {}", i);
        print_transactions(&validator_client).await;

        let blocks = FetchMetadata::fetch_new_block_events(2, &validator_client).await;
        analyze(&validator_peer_ids, blocks, 2);

        transfer_coins(
            &validator_client,
            &transaction_factory,
            &mut account_0,
            &account_1,
            10,
        )
        .await;

        thread::sleep(time::Duration::from_secs(10));
    }
    reconfig(
        &validator_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    thread::sleep(time::Duration::from_secs(5));

    print_transactions(&validator_client).await;

    let blocks = FetchMetadata::fetch_new_block_events(2, &validator_client).await;
    analyze(&validator_peer_ids, blocks, 2);
    assert!(false);
}

fn get_validator_addresses(data: &MoveResource, field_name: &str) -> Option<Vec<AccountAddress>> {
    let active_validators_json = data
        .data
        .0
        .get(&Identifier::new(field_name).unwrap())
        .unwrap();
    if let serde_json::Value::Array(active_validators) = active_validators_json {
        let mut validators: Vec<AccountAddress> = vec![];
        for validator in active_validators {
            if let serde_json::Value::Object(value) = validator {
                if let serde_json::Value::String(address) = &value["addr"] {
                    validators.push(AccountAddress::from_hex_literal(&address).unwrap());
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
        Some(validators)
    } else {
        None
    }
}

#[ignore]
#[tokio::test]
async fn test_local_full_node_validator_election() {
    let client = RestClient::new(Url::parse("https://sherryx.aptosdev.com").unwrap()); //"http://127.0.0.1:8080").unwrap());
    let partners: HashMap<AccountAddress, &str> = HashMap::from([
        (
            AccountAddress::from_hex_literal(
                "0x6f409c8b73a18cdeea4546c1df1e93594c07740cc609d93a99ca55c4920beb32",
            )
            .unwrap(),
            "Artifact Staking",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x0242bf2bb0c9d3afeea4d5e15c14608dade0903b383ef0def954e2091343cdc5",
            )
            .unwrap(),
            "metahash",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x938348b7c0d3fa132af43767c46eec9cdf4f726d0bb02e566b61abb11ac62e3a",
            )
            .unwrap(),
            "Pontem",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xff1edbd3e4fd10617261d8f8f9176d77e2093ddada51b248b461a65475e839dd",
            )
            .unwrap(),
            "Hashcell",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xb8587a0616068db6ac7bab5a64ac1b474b5c3ff90a5caf61fe67608eb6749441",
            )
            .unwrap(),
            "BwareLabs",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x1958fae48dae3cf9ed4f92e8308ee213401f2a41ca312980138a06a9b94e541f",
            )
            .unwrap(),
            "NodeReal",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x618aabd73e7eea9511853b3b73ccd479584a40282936cdf3cf82c04d4b085ffb",
            )
            .unwrap(),
            "LiananTech",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xce0a5f973a9b70169db96a7e69ba7167dfa453923022de26119f8ae3cdab8172",
            )
            .unwrap(),
            "Kiln",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xffdabcdd88d12ea0681e2667ca5ec85a573dbbe6d5fa527dfd2bca11eb5a7837",
            )
            .unwrap(),
            "Foundry",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xf5c22558b436a274913e750d91b55f06c52f84bdef6612ecdc221669e0171328",
            )
            .unwrap(),
            "Figment",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x871f63bc252b7428754c4c924d62e25a112790931f7ce82d13418fdae9b73ee3",
            )
            .unwrap(),
            "Corbin Page  / Paymagic",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xc493bb36ac19d800640b32eb2844da802d6530773b01eeaa4886ad25196d7210",
            )
            .unwrap(),
            "Coinbase Cloud",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xa541da2b986ba92d57f20a2de298895273f9601f3eab2cbf4029e0f3b9d45235",
            )
            .unwrap(),
            "Lightspeed",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xecce4a9945e076ad4d47ee6ecc99304d37bad2dbb5d90b45fd3cc044ced8d742",
            )
            .unwrap(),
            "Blockdaemon",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xa6d60a25c85e291feadda8408ff0506bb13fdf7cab962e8cd106b19132e70c4b",
            )
            .unwrap(),
            "Zaptos",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x700816b23b55467f76ce87c0dba228c7c564b61ec173a9f4788ca985c1957b00",
            )
            .unwrap(),
            "Mirny (nodeinfra.com)",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x489d99849903661ee6a608c91339b97c50ec2a4d7105c4c5da86ebe02d1ade55",
            )
            .unwrap(),
            "DoraFactory",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x87d597979680307ebaf079c74feb89582e4950b38b50f80a9b39ee743be790c3",
            )
            .unwrap(),
            "monobot",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x7cc078058cf827d1da8e8a701a38fd3f9b8e5eedd8283e3745e787e6cc307701",
            )
            .unwrap(),
            "RHINO",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xaaf2e1f8fb9a9cc90a66426e35153f1f60100d50ca0262e6982ba2ead977b92f",
            )
            .unwrap(),
            "Google Cloud",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x7165b38326736bef4bf4c215684fb9a399bcc3535f1c0bb10fe90132507e898d",
            )
            .unwrap(),
            "Lavender.Five Nodes",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xd86cad103074343d53d740cfbdd9b433d8d2d06795b974e6631d0121341bcbf8",
            )
            .unwrap(),
            "OKX",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x4ba5c50b70b70a90753d78d712d494d4bd0e945ea8755a82a3104c29a5e9edfe",
            )
            .unwrap(),
            "Blocto",
        ),
        (
            AccountAddress::from_hex_literal(
                "0xb7461f6e37c072ba1cd24382fe0fa78ee4a0af82d730af9938893d3c42d48021",
            )
            .unwrap(),
            "Aptos DevA",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x28c0c0b575af75d0ddac1eb5c17af7a8cf3208a8372415219a7e317264b8f4de",
            )
            .unwrap(),
            "Aptos aws1 (US West)",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x382b611bc8fd0c4efa0d8678d2787d877348b8fbc80a6fe402caf7933a42b1b7",
            )
            .unwrap(),
            "Aptos aws2 (US East)",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x83424ccb8c69982802c35f656a381ea4ee641aa431a8a24d9d1c3134ac697dd9",
            )
            .unwrap(),
            "Aptos gcp1 (Brazil)",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x46161f670671f424cdc7e5c395ce29f78ad36c028346cb5f32a26462226b6c45",
            )
            .unwrap(),
            "Aptos gcp2 (Switzerland)",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x0809575701fbf5bc0c57b5fc266b4da9eb8e7ef7e92af5df74a82b5a1f80466a",
            )
            .unwrap(),
            "Aptos gcp3 (Korea)",
        ),
        (
            AccountAddress::from_hex_literal(
                "0x2f1b57baf278817772bb0f5fe9a391aa6ba66bc6062808297e5cb12ffaf75581",
            )
            .unwrap(),
            "Aptos gcp4 (Singapore)",
        ),
    ]);
    let is_partner: HashMap<AccountAddress, &str> = partners
        .iter()
        .map(|(k, v)| (k.clone(), "partner"))
        .collect();

    let extra = partners;

    let mut start_epoch = 0;
    let mut start = 0;
    let end = client
        .get_transactions(None, Some(1))
        .await
        .unwrap()
        .into_inner()
        .first()
        .unwrap()
        .version()
        .unwrap();
    let batch: u64 = 1000;

    let mut validators: Vec<AccountAddress> = vec![];
    let mut epoch = 0;

    let mut result: Vec<BlockMetadataTransaction> = vec![];

    fn analyze(
        validators: &Vec<AccountAddress>,
        result: &Vec<BlockMetadataTransaction>,
        epoch: u64,
        partners: &HashMap<AccountAddress, &str>,
    ) -> Option<EpochStats> {
        if !validators.is_empty() {
            let events: Vec<NewBlockEvent> = result
                .iter()
                .map(FetchMetadata::metadata_to_new_block_event)
                .filter(|e| e.epoch() == epoch)
                .collect();
            println!("Analyzing epoch : {}", epoch);
            let stats = AnalyzeLeaderSelection::analyze(events, &validators);
            AnalyzeLeaderSelection::print_table(&stats, Some(partners), true);
            Some(stats)
        } else {
            None
        }
    }

    print_transactions(&client).await;

    let mut stats = HashMap::new();

    loop {
        let transactions = client
            .get_transactions(Some(start), Some(batch as u16))
            .await
            .unwrap()
            .into_inner();
        let len = transactions.len();
        for t in transactions {
            if let Transaction::BlockMetadataTransaction(metadata) = t {
                result.push(metadata.clone());
                for change in &metadata.info.changes {
                    if let WriteSetChange::WriteResource { data, .. } = change {
                        if data.typ.name.clone().into_string() == "ValidatorSet" {
                            if epoch >= start_epoch {
                                if let Some(epoch_stats) =
                                    analyze(&validators, &result, epoch, &extra)
                                {
                                    stats.insert(epoch, epoch_stats);
                                }

                                #[derive(Debug, Eq, PartialEq, Hash)]
                                pub enum NodeState {
                                    HEALTHY,
                                    ALIVE,
                                    DOWN,
                                    NOT_PRESENT,
                                }

                                impl NodeState {
                                    pub fn to_char(&self) -> &str {
                                        match self {
                                            Self::HEALTHY => "+",
                                            Self::ALIVE => "~",
                                            Self::DOWN => "-",
                                            Self::NOT_PRESENT => " ",
                                        }
                                    }

                                    pub fn to_order_weight(&self) -> usize {
                                        match self {
                                            Self::HEALTHY => 0,
                                            Self::ALIVE => 100,
                                            Self::DOWN => 10000,
                                            Self::NOT_PRESENT => 1,
                                        }
                                    }
                                }

                                fn to_state(
                                    epoch: u64,
                                    validator: &AccountAddress,
                                    stats: &HashMap<u64, EpochStats>,
                                ) -> NodeState {
                                    stats
                                        .get(&epoch)
                                        .map(|s| {
                                            s.validator_stats
                                                .get(&validator)
                                                .map(|b| {
                                                    if b.is_healthy() {
                                                        NodeState::HEALTHY
                                                    } else {
                                                        if b.is_alive() {
                                                            NodeState::ALIVE
                                                        } else {
                                                            NodeState::DOWN
                                                        }
                                                    }
                                                })
                                                .unwrap_or(NodeState::NOT_PRESENT)
                                        })
                                        .unwrap_or(NodeState::NOT_PRESENT)
                                }

                                let mut sorted_validators = validators.clone();
                                sorted_validators.sort_by_cached_key(|validator| {
                                    (
                                        (start_epoch..(epoch + 1))
                                            .map(|cur_epoch| {
                                                to_state(cur_epoch, &validator, &stats)
                                                    .to_order_weight()
                                            })
                                            .sum::<usize>(),
                                        validator.clone(),
                                    )
                                });

                                for validator in sorted_validators {
                                    print!(
                                        "{} {: <30}:  ",
                                        validator,
                                        extra.get(&validator).unwrap_or(&"")
                                    );
                                    for cur_epoch in start_epoch..(epoch + 1) {
                                        print!(
                                            "{}",
                                            to_state(cur_epoch, &validator, &stats).to_char()
                                        );
                                    }
                                    println!();
                                }
                                println!(
                                    "{: <8} | {: <8} | {: <8} | {: <8}",
                                    "epoch", "healthy", "alive", "down"
                                );
                                for cur_epoch in start_epoch..(epoch + 1) {
                                    let counts = validators
                                        .iter()
                                        .map(|v| to_state(cur_epoch, &v, &stats))
                                        .counts();
                                    println!(
                                        "{: <8} | {: <8} | {: <8} | {: <8}",
                                        cur_epoch,
                                        counts.get(&NodeState::HEALTHY).unwrap_or(&0),
                                        counts.get(&NodeState::ALIVE).unwrap_or(&0),
                                        counts.get(&NodeState::DOWN).unwrap_or(&0)
                                    );
                                }
                            }

                            result = vec![];

                            if let Some(v) = get_validator_addresses(data, "active_validators") {
                                validators = v;
                                validators.sort();
                                epoch = metadata.epoch.0 + 1;
                            } else {
                                validators = vec![];
                            }
                            // No pending at epoch change
                            assert_eq!(
                                Some(vec![]),
                                get_validator_addresses(data, "pending_inactive")
                            );
                            assert_eq!(
                                Some(vec![]),
                                get_validator_addresses(data, "pending_active")
                            );
                        }
                    }
                }
            }
        }

        start += batch;
        if start % 100000 == 0 {
            println!(
                "Fetched {} metadata from {} transactions",
                result.len(),
                start
            );
        }

        if start > end {
            let total_stats = stats
                .clone()
                .into_iter()
                .map(|(_k, v)| v)
                .reduce(|a, b| a + b)
                .unwrap();
            println!(
                "Analyzing totals (full epochs {} to {})",
                start_epoch,
                epoch - 1
            );
            AnalyzeLeaderSelection::print_table(&total_stats, Some(&extra), true);

            if let Some(epoch_stats) = analyze(&validators, &result, epoch, &extra) {
                stats.insert(epoch, epoch_stats);
            }
            break;
        }
    }

    let total_stats = stats
        .into_iter()
        .map(|(_k, v)| v)
        .reduce(|a, b| a + b)
        .unwrap();
    println!("Analyzing totals (current)");
    AnalyzeLeaderSelection::print_table(&total_stats, Some(&extra), true);

    assert!(false);
}
