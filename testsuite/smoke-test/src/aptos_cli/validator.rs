// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    smoke_test_environment::SwarmBuilder,
    utils::{create_and_fund_account, MAX_CATCH_UP_WAIT_SECS},
};
use aptos_bitvec::BitVec;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{bls12381, ed25519::Ed25519PrivateKey, x25519, ValidCryptoMaterialStringExt};
use aptos_forge::{reconfig, wait_for_all_nodes_to_catchup, LocalSwarm, NodeExt, Swarm, SwarmExt};
use aptos_genesis::config::HostAndPort;
use aptos_keygen::KeyGen;
use aptos_logger::info;
use aptos_rest_client::{Client, State};
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    network_address::DnsName,
    on_chain_config::{
        ConsensusAlgorithmConfig, ConsensusConfigV1, ExecutionConfigV1, LeaderReputationType,
        OnChainConsensusConfig, OnChainExecutionConfig, OnChainRandomnessConfig,
        ProposerAndVoterConfig, ProposerElectionType, TransactionShufflerType, ValidatorSet,
    },
    PeerId,
};
use movement::{
    account::create::DEFAULT_FUNDED_COINS,
    common::types::TransactionSummary,
    node::analyze::{
        analyze_validators::{AnalyzeValidators, EpochStats},
        fetch_metadata::FetchMetadata,
    },
    test::{CliTestFramework, ValidatorPerformance},
};
use std::{
    collections::{HashMap, HashSet},
    convert::TryFrom,
    fmt::Write,
    str::FromStr,
    sync::Arc,
    time::Duration,
};

#[tokio::test]
async fn test_analyze_validators() {
    let (swarm, cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_genesis_stake(Arc::new(|_i, genesis_stake_amount| {
            *genesis_stake_amount = 100000;
        }))
        .build_with_cli(0)
        .await;
    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    tokio::time::sleep(Duration::from_secs(3)).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    cli.analyze_validator_performance(None, None).await.unwrap();
}

#[tokio::test]
async fn test_show_validator_set() {
    let (swarm, cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(1)
        .await;
    let validator_set = cli.show_validator_set().await.unwrap();

    assert_eq!(1, validator_set.active_validators.len());
    assert_eq!(0, validator_set.pending_inactive.len());
    assert_eq!(0, validator_set.pending_active.len());
    assert_eq!(
        validator_set
            .active_validators
            .first()
            .unwrap()
            .account_address(),
        &swarm.validators().next().unwrap().peer_id()
    );
}

/// One effect of updated leader election config, is that new node/node that was down before
/// will be elected much sooner than before.
/// This function checks for a node that is down and not being elected, how long it takes for it
/// to start voting (after being up), and then how long it takes for it to be elected.
async fn check_vote_to_elected(swarm: &mut LocalSwarm) -> (Option<u64>, Option<u64>) {
    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let (address_off, rest_client_off) = swarm
        .validators()
        .nth(1)
        .map(|v| (v.peer_id(), v.rest_client()))
        .unwrap();

    let (_address_after_off, rest_client_after_off) = swarm
        .validators()
        .nth(2)
        .map(|v| (v.peer_id(), v.rest_client()))
        .unwrap();

    rest_client_off
        .set_failpoint("consensus::send::any".to_string(), "100%return".to_string())
        .await
        .unwrap();

    // clear leader reputation history, so we stop electing down node altogether
    for _ in 0..5 {
        reconfig(
            &rest_client,
            &transaction_factory,
            swarm.chain_info().root_account(),
        )
        .await;
    }

    tokio::time::sleep(Duration::from_secs(10)).await;
    rest_client_off
        .set_failpoint("consensus::send::any".to_string(), "off".to_string())
        .await
        .unwrap();

    let epoch = reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await
    .epoch;

    // Turn off a different node, to force votes from recently down node being required for forming consnensus
    rest_client_after_off
        .set_failpoint("consensus::send::any".to_string(), "100%return".to_string())
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(60)).await;

    rest_client_after_off
        .set_failpoint("consensus::send::any".to_string(), "off".to_string())
        .await
        .unwrap();

    let events = FetchMetadata::fetch_new_block_events(&rest_client, Some(epoch as i64), None)
        .await
        .unwrap();
    assert_eq!(events.len(), 1);

    let info = events.first().unwrap();
    let off_index = info
        .validators
        .iter()
        .find(|v| v.address == address_off)
        .unwrap()
        .validator_index;
    let mut first_vote = None;
    let mut first_elected = None;
    for event in info.blocks.iter() {
        let previous_block_votes_bitvec: BitVec =
            event.event.previous_block_votes_bitvec().clone().into();
        if first_vote.is_none() && previous_block_votes_bitvec.is_set(off_index) {
            first_vote = Some(event.event.round());
        }

        if first_elected.is_none() && event.event.proposer() == address_off {
            first_elected = Some(event.event.round());
        }
    }
    (first_vote, first_elected)
}

#[tokio::test]
#[ignore]
async fn test_onchain_config_change() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, conf, _| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 400;
            conf.consensus.quorum_store_poll_time_ms = 100;
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            let inner = match genesis_config.consensus_config.clone() {
                OnChainConsensusConfig::V1(inner) => inner,
                OnChainConsensusConfig::V2(inner) => inner,
                OnChainConsensusConfig::V3 {
                    alg: ConsensusAlgorithmConfig::Jolteon { main, .. },
                    ..
                } => main,
                OnChainConsensusConfig::V4 {
                    alg: ConsensusAlgorithmConfig::Jolteon { main, .. },
                    ..
                } => main,
                _ => panic!("Other branches for OnChainConsensusConfig are not covered"),
            };

            let leader_reputation_type =
                if let ProposerElectionType::LeaderReputation(leader_reputation_type) =
                    inner.proposer_election_type
                {
                    leader_reputation_type
                } else {
                    panic!()
                };
            let proposer_and_voter_config = match &leader_reputation_type {
                LeaderReputationType::ProposerAndVoter(_) => panic!(),
                LeaderReputationType::ProposerAndVoterV2(proposer_and_voter_config) => {
                    proposer_and_voter_config
                },
            };
            let new_consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1 {
                proposer_election_type: ProposerElectionType::LeaderReputation(
                    LeaderReputationType::ProposerAndVoter(ProposerAndVoterConfig {
                        proposer_window_num_validators_multiplier: 20,
                        // reduce max epoch history to speed up the test.
                        use_history_from_previous_epoch_max_count: 2,
                        // make test not flaky, by making unlikely selections extremely unlikely:
                        active_weight: 1000000,
                        ..*proposer_and_voter_config
                    }),
                ),
                ..inner
            });
            genesis_config.consensus_config = new_consensus_config;
        }))
        .with_aptos()
        .build_with_cli(0)
        .await;

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
        OnChainConsensusConfig::V2(inner) => inner,
        _ => unimplemented!(),
    };
    let leader_reputation_type =
        if let ProposerElectionType::LeaderReputation(leader_reputation_type) =
            inner.proposer_election_type
        {
            leader_reputation_type
        } else {
            panic!()
        };
    let proposer_and_voter_config = match &leader_reputation_type {
        LeaderReputationType::ProposerAndVoterV2(_) => panic!(),
        LeaderReputationType::ProposerAndVoter(proposer_and_voter_config) => {
            proposer_and_voter_config
        },
    };
    let new_consensus_config = OnChainConsensusConfig::V1(ConsensusConfigV1 {
        proposer_election_type: ProposerElectionType::LeaderReputation(
            LeaderReputationType::ProposerAndVoterV2(*proposer_and_voter_config),
        ),
        ..inner
    });

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

    // confirm with old configs, validator will need to wait quite a bit from voting to being elected
    let (first_vote_old, first_elected_old) = check_vote_to_elected(&mut swarm).await;
    println!(
        "With old config: {:?} to {:?}",
        first_vote_old, first_elected_old
    );

    println!(
        "Epoch before : {}",
        rest_client
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner()
            .epoch
    );
    cli.run_script(root_cli_index, &update_consensus_config_script)
        .await
        .unwrap();
    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();
    swarm
        .wait_for_all_nodes_to_catchup_to_next(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();
    println!(
        "Epoch after : {}",
        rest_client
            .get_ledger_information()
            .await
            .unwrap()
            .into_inner()
            .epoch
    );

    // confirm with new configs, validator doesn't wait much from voting to being elected
    let (first_vote_new, first_elected_new) = check_vote_to_elected(&mut swarm).await;
    println!(
        "With new config: {:?} to {:?}",
        first_vote_new, first_elected_new
    );

    cli.analyze_validator_performance(Some(0), None)
        .await
        .unwrap();

    // Node that is down, should start voting very fast
    assert!(first_vote_old.unwrap() < 20);
    assert!(first_vote_new.unwrap() < 20);
    // In old config, we expect there to be a lot of rounds before node gets elected as leader
    assert!(first_elected_old.unwrap() > 80);
    // In updated config, we expect it to be elected pretty fast.
    // There is necessary 20 rounds delay due to exclude_round, and then only a few more rounds.
    assert!(first_elected_new.unwrap() < 40);
}

#[tokio::test]
#[ignore]
// This test is ignored because it is very long running
async fn test_onchain_shuffling_change() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(2)
        .with_aptos()
        .build_with_cli(0)
        .await;

    let root_cli_index = cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );

    let rest_client = swarm.validators().next().unwrap().rest_client();

    let current_execution_config: OnChainExecutionConfig = bcs::from_bytes(
        &rest_client
            .get_account_resource_bcs::<Vec<u8>>(
                CORE_CODE_ADDRESS,
                "0x1::execution_config::ExecutionConfig",
            )
            .await
            .unwrap()
            .into_inner(),
    )
    .unwrap();

    assert_eq!(
        current_execution_config.transaction_shuffler_type(),
        TransactionShufflerType::default_for_genesis(),
    );

    assert_reordering(&mut swarm, true).await;

    let execution_config_with_shuffling = OnChainExecutionConfig::V1(ExecutionConfigV1 {
        transaction_shuffler_type: TransactionShufflerType::NoShuffling,
    });

    let update_execution_config_script = format!(
        r#"
    script {{
        use aptos_framework::aptos_governance;
        use aptos_framework::execution_config;
        fun main(core_resources: &signer) {{
            let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
            let config_bytes = {};
            execution_config::set(&framework_signer, config_bytes);
        }}
    }}
    "#,
        generate_blob(&bcs::to_bytes(&execution_config_with_shuffling).unwrap())
    );

    cli.run_script(root_cli_index, &update_execution_config_script)
        .await
        .unwrap();

    let updated_execution_config: OnChainExecutionConfig = bcs::from_bytes(
        &rest_client
            .get_account_resource_bcs::<Vec<u8>>(
                CORE_CODE_ADDRESS,
                "0x1::execution_config::ExecutionConfig",
            )
            .await
            .unwrap()
            .into_inner(),
    )
    .unwrap();

    assert_eq!(
        updated_execution_config.transaction_shuffler_type(),
        TransactionShufflerType::NoShuffling,
    );

    assert_reordering(&mut swarm, false).await;
}

async fn assert_reordering(swarm: &mut dyn Swarm, expected_reordering: bool) {
    swarm
        .aptos_public_info()
        .sync_root_account_sequence_number()
        .await;
    let transaction_factory = swarm.aptos_public_info().transaction_factory();

    let clients = swarm.get_all_nodes_clients_with_names();

    let dst = create_and_fund_account(swarm, 10000000000).await;

    let mut accounts = vec![];
    let mut txns = vec![];
    for _ in 0..2 {
        let account = create_and_fund_account(swarm, 10000000000).await;

        for _ in 0..5 {
            let txn = account.sign_with_transaction_builder(
                transaction_factory.payload(aptos_stdlib::aptos_coin_transfer(dst.address(), 10)),
            );
            txns.push(txn);
        }
        accounts.push(account);
    }

    let result = clients[0]
        .1
        .submit_batch_bcs(&txns)
        .await
        .unwrap()
        .into_inner();
    info!("result: {:?}", result);

    for txn in &txns {
        clients[0].1.wait_for_signed_transaction(txn).await.unwrap();
    }

    wait_for_all_nodes_to_catchup(&clients, Duration::from_secs(30))
        .await
        .unwrap();

    let committed_order = clients[0]
        .1
        .get_transactions_bcs(None, Some(1000))
        .await
        .unwrap()
        .into_inner();

    info!(
        "dst: {}, senders: {:?}",
        dst.address(),
        accounts.iter().map(|a| a.address()).collect::<Vec<_>>()
    );
    let mut block_txns = vec![];
    for txn in committed_order {
        match txn.transaction {
            aptos_types::transaction::Transaction::UserTransaction(txn) => {
                info!("from {}, seq_num {}", txn.sender(), txn.sequence_number());
                block_txns.push(txn);
            },
            aptos_types::transaction::Transaction::BlockMetadata(b) => {
                info!("block metadata {}", b.round());

                let senders = accounts.iter().map(|a| a.address()).collect::<HashSet<_>>();
                let mut changes = 0;
                for i in 1..block_txns.len() {
                    if block_txns[i - 1].sender() != block_txns[i].sender()
                        && senders.contains(&block_txns[i].sender())
                    {
                        changes += 1;
                    }
                }
                info!("block_txns.len: {}, changes: {}", block_txns.len(), changes);

                if changes > 1 {
                    assert!(expected_reordering, "changes: {}", changes);
                }
                if block_txns.len() >= 4 && changes == 1 {
                    assert!(!expected_reordering, "changes: {}", changes)
                }
                block_txns.clear();
            },
            _ => {},
        }
    }
}

pub(crate) fn generate_blob(data: &[u8]) -> String {
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


#[tokio::test]
async fn test_large_total_stake() {
    // just barelly below u64::MAX
    const BASE: u64 = 10_000_000_000_000_000_000;
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_genesis_stake(Arc::new(|_, genesis_stake_amount| {
            // make sure we have quorum
            *genesis_stake_amount = BASE;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 4;
            genesis_config.recurring_lockup_duration_secs = 4;
            genesis_config.voting_duration_secs = 3;
            genesis_config.randomness_config_override =
                Some(OnChainRandomnessConfig::default_disabled());
        }))
        .build_with_cli(0)
        .await;

    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();
    let (validator_cli_index, keys) = init_validator_account(&mut cli, &mut keygen, None).await;
    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    cli.initialize_validator(
        validator_cli_index,
        keys.consensus_public_key(),
        keys.consensus_proof_of_possession(),
        HostAndPort {
            host: dns_name("0.0.0.0"),
            port: 1234,
        },
        keys.network_public_key(),
    )
    .await
    .unwrap();

    cli.join_validator_set(validator_cli_index, None)
        .await
        .unwrap();

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    assert_eq!(
        get_validator_state(&cli, validator_cli_index).await,
        ValidatorState::ACTIVE
    );

    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_CATCH_UP_WAIT_SECS))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_nodes_rewards() {
    // with 10% APY, BASE amount gives 100 rewards per second
    const BASE: u64 = 3600u64 * 24 * 365 * 10 * 100;

    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, conf, _| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_time_ms = 100;
            conf.api.failpoints_enabled = true;
        }))
        .with_init_genesis_stake(Arc::new(|i, genesis_stake_amount| {
            // make sure we have quorum
            *genesis_stake_amount = if i < 2 { 10 * BASE } else { BASE };
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 4;
            genesis_config.recurring_lockup_duration_secs = 4;
            genesis_config.voting_duration_secs = 3;
            genesis_config.rewards_apy_percentage = 10;
        }))
        .build_with_cli(0)
        .await;

    let transaction_factory = swarm.chain_info().transaction_factory();

    let mut validators: Vec<_> = swarm.validators().collect();
    validators.sort_by_key(|v| v.name());

    let validator_cli_indices = validators
        .iter()
        .map(|validator| {
            cli.add_account_to_cli(
                validator
                    .account_private_key()
                    .as_ref()
                    .unwrap()
                    .private_key(),
            )
        })
        .collect::<Vec<_>>();
    let rest_clients = validators
        .iter()
        .map(|validator| validator.rest_client())
        .collect::<Vec<_>>();
    let addresses = validators
        .iter()
        .map(|validator| validator.peer_id())
        .collect::<Vec<_>>();

    println!("{:?}", addresses.iter().enumerate().collect::<Vec<_>>());

    async fn get_state_and_validator_set(rest_client: &Client) -> (State, HashMap<PeerId, u64>) {
        let (validator_set, state): (ValidatorSet, State) = rest_client
            .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
            .await
            .unwrap()
            .into_parts();
        let validator_to_voting_power = validator_set
            .active_validators
            .iter()
            .chain(validator_set.pending_inactive.iter())
            .map(|v| (v.account_address, v.consensus_voting_power()))
            .collect::<HashMap<_, _>>();
        (state, validator_to_voting_power)
    }

    println!(
        "{:?}",
        get_state_and_validator_set(&rest_clients[0]).await.1
    );

    rest_clients[2]
        .set_failpoint(
            "consensus::send::broadcast_proposal".to_string(),
            "100%return".to_string(),
        )
        .await
        .unwrap();

    reconfig(
        &rest_clients[0],
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    let (start_2_failures_state, start_2_failures_validator_set) =
        get_state_and_validator_set(&rest_clients[0]).await;
    println!(
        "Node 2 ({}) starts failing: at epoch {} and version {}, set: {:?}",
        addresses[2],
        start_2_failures_state.epoch,
        start_2_failures_state.version,
        start_2_failures_validator_set
    );

    tokio::time::sleep(Duration::from_secs(5)).await;

    reconfig(
        &rest_clients[0],
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    cli.fund_account(validator_cli_indices[3], Some(30000))
        .await
        .unwrap();

    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_clients[3])
        .await
        .unwrap();

    cli.leave_validator_set(validator_cli_indices[3], None)
        .await
        .unwrap();

    reconfig(
        &rest_clients[0],
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    let (start_3_left_state, start_3_left_validator_set) =
        get_state_and_validator_set(&rest_clients[0]).await;
    println!(
        "Node 3 ({}) leaves validator set: at epoch {} and version {}, set: {:?}",
        addresses[3],
        start_3_left_state.epoch,
        start_3_left_state.version,
        start_3_left_validator_set
    );
    let end_2_failures_epoch = start_3_left_state.epoch;
    rest_clients[2]
        .set_failpoint(
            "consensus::send::broadcast_proposal".to_string(),
            "20%return".to_string(),
        )
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_secs(5)).await;
    reconfig(
        &rest_clients[0],
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    tokio::time::sleep(Duration::from_secs(3)).await;

    reconfig(
        &rest_clients[0],
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    let (end_state, end_validator_set) = get_state_and_validator_set(&rest_clients[0]).await;
    println!(
        "END: at epoch {} and version {} set: {:?}",
        end_state.epoch, end_state.version, end_validator_set
    );

    cli.analyze_validator_performance(None, None).await.unwrap();

    let epochs = FetchMetadata::fetch_new_block_events(&rest_clients[0], None, None)
        .await
        .unwrap();

    let mut previous_stats: Option<EpochStats> = None;
    for epoch_info in epochs {
        println!(
            "Processing epoch {} for versions [{}, {}]",
            epoch_info.epoch,
            epoch_info.blocks.first().unwrap().version,
            epoch_info.blocks.last().unwrap().version
        );
        if let Some(previous) = previous_stats {
            let mut estimates = Vec::new();
            for cur_validator in &epoch_info.validators {
                let prev_stats = previous
                    .validator_stats
                    .get(&cur_validator.address)
                    .unwrap();
                if prev_stats.proposal_successes == 0 {
                    assert_eq!(cur_validator.voting_power, prev_stats.voting_power);
                } else {
                    assert!(cur_validator.voting_power > prev_stats.voting_power, "in epoch {} voting power for {} didn't increase with successful proposals (from {} to {})", epoch_info.epoch - 1, cur_validator.address, prev_stats.voting_power, cur_validator.voting_power);
                    let earning = (cur_validator.voting_power - prev_stats.voting_power) as f64
                        / prev_stats.voting_power as f64;
                    let failure_rate = prev_stats.failure_rate() as f64;
                    let epoch_reward_estimate = earning / (1.0 - failure_rate);
                    println!(
                        "{}: {} / {} = {}, prev_voting_power = {}",
                        cur_validator.address,
                        earning,
                        failure_rate,
                        epoch_reward_estimate,
                        prev_stats.voting_power
                    );
                    estimates.push(epoch_reward_estimate);
                }
            }
            if !estimates.is_empty() {
                assert!(
                    estimates.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                        / estimates.iter().copied().fold(f64::INFINITY, f64::min)
                        < 1.1,
                    "in epoch {}, estimated epoch reward rate differs: {:?}",
                    epoch_info.epoch - 1,
                    estimates
                );
            }
        }

        let last_version = epoch_info.blocks.iter().map(|b| b.version).max().unwrap();
        let epoch_stats = AnalyzeValidators::analyze(&epoch_info.blocks, &epoch_info.validators);

        if epoch_info.epoch >= start_3_left_state.epoch {
            assert!(
                !epoch_stats.validator_stats.contains_key(&addresses[3]),
                "Epoch {}, node {} shouldn't be present",
                epoch_info.epoch,
                addresses[3]
            );
        }

        if epoch_info.epoch >= start_2_failures_state.epoch
            && epoch_info.epoch < end_2_failures_epoch
        {
            assert_eq!(
                0,
                epoch_stats
                    .validator_stats
                    .get(&addresses[2])
                    .unwrap()
                    .proposal_successes,
                "Epoch {}, node {} shouldn't have any successful proposals",
                epoch_info.epoch,
                addresses[2]
            );
        }

        let mut epoch_perf = serde_json::from_value::<ValidatorPerformance>(
            rest_clients[0]
                .get_account_resource_at_version(
                    PeerId::ONE,
                    "0x1::stake::ValidatorPerformance",
                    last_version,
                )
                .await
                .unwrap()
                .into_inner()
                .unwrap()
                .data,
        )
        .unwrap();

        println!(
            "ValidatorPerformance for epoch {} at version {}: {:?}",
            epoch_info.epoch, last_version, epoch_perf
        );

        // If epoch change happens with the BlockMetadata block, we don't have the last ValidatorPerformance
        // for that epoch, so we take one before it.
        let target_stats = if epoch_perf
            .validators
            .iter()
            .map(|v| v.successful_proposals() + v.failed_proposals())
            .sum::<u32>()
            == 0
        {
            println!(
                "Don't have latest perf, doing one before, at version {}",
                last_version - 1
            );
            epoch_perf = serde_json::from_value::<ValidatorPerformance>(
                rest_clients[0]
                    .get_account_resource_at_version(
                        PeerId::ONE,
                        "0x1::stake::ValidatorPerformance",
                        last_version - 1,
                    )
                    .await
                    .unwrap()
                    .into_inner()
                    .unwrap()
                    .data,
            )
            .unwrap();
            AnalyzeValidators::analyze(
                &epoch_info.blocks[..epoch_info.blocks.len() - 1],
                &epoch_info.validators,
            )
        } else {
            epoch_stats.clone()
        };

        for info in epoch_info.validators {
            let v_stats = target_stats.validator_stats.get(&info.address).unwrap();
            let v_perf = epoch_perf
                .validators
                .get(info.validator_index as usize)
                .unwrap();
            assert_eq!(
                v_stats.proposal_successes,
                v_perf.successful_proposals(),
                "Epoch {}\n  info  {:?}\n  stats {:?}\n  perf  {:?}",
                epoch_info.epoch,
                info,
                epoch_stats.validator_stats,
                epoch_perf.validators
            );
            assert_eq!(
                v_stats.proposal_failures,
                v_perf.failed_proposals(),
                "Epoch {}\n  info  {:?}\n  stats {:?}\n  perf  {:?}",
                epoch_info.epoch,
                info,
                epoch_stats.validator_stats,
                epoch_perf.validators
            );
        }

        previous_stats = Some(epoch_stats);
    }
}

#[tokio::test]
async fn test_register_and_update_validator() {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();
    let (validator_cli_index, keys) = init_validator_account(&mut cli, &mut keygen, None).await;
    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    assert!(cli
        .show_validator_config(validator_cli_index)
        .await
        .is_err()); // validator not registered yet

    let port = 1234;
    cli.initialize_validator(
        validator_cli_index,
        keys.consensus_public_key(),
        keys.consensus_proof_of_possession(),
        HostAndPort {
            host: dns_name("0.0.0.0"),
            port,
        },
        keys.network_public_key(),
    )
    .await
    .unwrap();

    let validator_config = cli
        .show_validator_config(validator_cli_index)
        .await
        .unwrap();
    assert_eq!(
        validator_config.consensus_public_key,
        keys.consensus_public_key()
            .to_encoded_string()
            .unwrap()
            .as_bytes()
            .to_vec()
    );

    let new_port = 5678;
    let new_network_private_key = keygen.generate_x25519_private_key().unwrap();

    cli.update_validator_network_addresses(
        validator_cli_index,
        None,
        HostAndPort {
            host: dns_name("0.0.0.0"),
            port: new_port,
        },
        new_network_private_key.public_key(),
    )
    .await
    .unwrap();

    let validator_config = cli
        .show_validator_config(validator_cli_index)
        .await
        .unwrap();

    let address_new = validator_config
        .validator_network_addresses()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();
    assert_eq!(
        address_new.find_noise_proto().unwrap(),
        new_network_private_key.public_key()
    );
    assert_eq!(address_new.find_port().unwrap(), new_port);

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    // because we haven't joined the validator set yet, we shouldn't be there
    let validator_set = cli.show_validator_set().await.unwrap();
    assert_eq!(1, validator_set.active_validators.len());
    assert_eq!(0, validator_set.pending_inactive.len());
    assert_eq!(0, validator_set.pending_active.len());
}

#[tokio::test]
async fn test_join_and_leave_validator() {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_i, conf, _| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_time_ms = 100;
        }))
        .with_init_genesis_stake(Arc::new(|_i, genesis_stake_amount| {
            *genesis_stake_amount = 100000;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 5;
            genesis_config.recurring_lockup_duration_secs = 10;
            genesis_config.voting_duration_secs = 5;
            genesis_config.randomness_config_override = Some(OnChainRandomnessConfig::Off);
        }))
        .build_with_cli(0)
        .await;

    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();
    let (validator_cli_index, keys) =
        init_validator_account(&mut cli, &mut keygen, Some(DEFAULT_FUNDED_COINS * 3)).await;
    let mut gas_used = 0;

    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    let port = 1234;
    gas_used += get_gas(
        cli.initialize_validator(
            validator_cli_index,
            keys.consensus_public_key(),
            keys.consensus_proof_of_possession(),
            HostAndPort {
                host: dns_name("0.0.0.0"),
                port,
            },
            keys.network_public_key(),
        )
        .await
        .unwrap(),
    );

    assert_validator_set_sizes(&cli, 1, 0, 0).await;

    cli.assert_account_balance_now(validator_cli_index, (3 * DEFAULT_FUNDED_COINS) - gas_used)
        .await;

    let stake_coins = 7;
    gas_used += get_gas(
        cli.add_stake(validator_cli_index, stake_coins)
            .await
            .unwrap()[0]
            .clone(),
    );

    cli.assert_account_balance_now(
        validator_cli_index,
        (3 * DEFAULT_FUNDED_COINS) - stake_coins - gas_used,
    )
    .await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    assert_validator_set_sizes(&cli, 1, 0, 0).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    assert_validator_set_sizes(&cli, 1, 0, 0).await;

    gas_used += get_gas(
        cli.join_validator_set(validator_cli_index, None)
            .await
            .unwrap(),
    );

    assert_validator_set_sizes(&cli, 1, 1, 0).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    assert_validator_set_sizes(&cli, 2, 0, 0).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    gas_used += get_gas(
        cli.leave_validator_set(validator_cli_index, None)
            .await
            .unwrap(),
    );

    assert_validator_set_sizes(&cli, 1, 0, 1).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    assert_validator_set_sizes(&cli, 1, 0, 0).await;

    cli.assert_account_balance_now(
        validator_cli_index,
        (3 * DEFAULT_FUNDED_COINS) - stake_coins - gas_used,
    )
    .await;

    let unlock_stake = 3;

    // Unlock stake.
    gas_used += get_gas(
        cli.unlock_stake(validator_cli_index, unlock_stake)
            .await
            .unwrap()[0]
            .clone(),
    );

    // Conservatively wait until the recurring lockup is over.
    tokio::time::sleep(Duration::from_secs(10)).await;

    let withdraw_stake = 2;
    gas_used += get_gas(
        cli.withdraw_stake(validator_cli_index, withdraw_stake)
            .await
            .unwrap()
            .remove(0),
    );

    cli.assert_account_balance_now(
        validator_cli_index,
        (3 * DEFAULT_FUNDED_COINS) - stake_coins + withdraw_stake - gas_used,
    )
    .await;
}

#[tokio::test]
async fn test_owner_create_and_delegate_flow() {
    let (swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_i, conf, _| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_time_ms = 100;
        }))
        .with_init_genesis_stake(Arc::new(|_i, genesis_stake_amount| {
            // enough for quorum
            *genesis_stake_amount = 5000000;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 5;
            genesis_config.recurring_lockup_duration_secs = 10;
            genesis_config.voting_duration_secs = 5;
            genesis_config.min_stake = 500000;
            genesis_config.randomness_config_override = Some(OnChainRandomnessConfig::Off);
        }))
        .build_with_cli(0)
        .await;

    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();

    let owner_initial_coins = 20000000;
    let voter_initial_coins = 1000000;
    let operator_initial_coins = 1000000;

    // Owner of the coins receives coins
    let owner_cli_index = cli
        .create_cli_account_from_faucet(
            keygen.generate_ed25519_private_key(),
            Some(owner_initial_coins),
        )
        .await
        .unwrap();
    println!("owner CLI index: {}", owner_cli_index);

    cli.assert_account_balance_now(owner_cli_index, owner_initial_coins)
        .await;

    // Faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    let operator_keys = ValidatorNodeKeys::new(&mut keygen);
    let voter_cli_index = cli
        .create_cli_account(keygen.generate_ed25519_private_key(), owner_cli_index)
        .await
        .unwrap();
    let operator_cli_index = cli
        .create_cli_account(operator_keys.account_private_key.clone(), owner_cli_index)
        .await
        .unwrap();

    // Fetch amount of gas used for the above account creations
    let mut owner_gas =
        owner_initial_coins - cli.account_balance_now(owner_cli_index).await.unwrap();
    println!("owner_gas1: {}", owner_gas);

    // Voter and operator start with no coins
    // Owner needs to send small amount of coins to operator and voter, to create their accounts and so they have enough for gas fees.
    owner_gas += cli
        .transfer_coins(owner_cli_index, voter_cli_index, voter_initial_coins, None)
        .await
        .unwrap()
        .octa_spent();
    owner_gas += cli
        .transfer_coins(
            owner_cli_index,
            operator_cli_index,
            operator_initial_coins,
            None,
        )
        .await
        .unwrap()
        .octa_spent();

    cli.assert_account_balance_now(
        owner_cli_index,
        owner_initial_coins - voter_initial_coins - operator_initial_coins - owner_gas,
    )
    .await;
    cli.assert_account_balance_now(voter_cli_index, voter_initial_coins)
        .await;
    cli.assert_account_balance_now(operator_cli_index, operator_initial_coins)
        .await;

    let stake_amount = 1000000;
    let mut operator_gas = 0;
    owner_gas += get_gas(
        cli.initialize_stake_owner(
            owner_cli_index,
            stake_amount,
            Some(voter_cli_index),
            Some(operator_cli_index),
        )
        .await
        .unwrap(),
    );

    println!("before4");
    cli.assert_account_balance_now(
        owner_cli_index,
        owner_initial_coins
            - voter_initial_coins
            - operator_initial_coins
            - stake_amount
            - owner_gas,
    )
    .await;
    println!("after4");

    assert_validator_set_sizes(&cli, 1, 0, 0).await;
    assert_eq!(
        get_validator_state(&cli, owner_cli_index).await,
        ValidatorState::NONE
    );

    let port = 6543;

    operator_gas += get_gas(
        cli.update_consensus_key(
            operator_cli_index,
            Some(owner_cli_index),
            operator_keys.consensus_public_key(),
            operator_keys.consensus_proof_of_possession(),
            None,
        )
        .await
        .unwrap(),
    );

    operator_gas += get_gas(
        cli.update_validator_network_addresses(
            operator_cli_index,
            Some(owner_cli_index),
            HostAndPort {
                host: dns_name("0.0.0.0"),
                port,
            },
            operator_keys.network_public_key(),
        )
        .await
        .unwrap(),
    );

    println!("before5");
    cli.assert_account_balance_now(operator_cli_index, operator_initial_coins - operator_gas)
        .await;
    println!("after5");

    cli.join_validator_set(operator_cli_index, Some(owner_cli_index))
        .await
        .unwrap();

    let owner_state = get_validator_state(&cli, owner_cli_index).await;
    if owner_state == ValidatorState::JOINING {
        reconfig(
            &rest_client,
            &transaction_factory,
            swarm.chain_info().root_account(),
        )
        .await;

        assert_eq!(
            get_validator_state(&cli, owner_cli_index).await,
            ValidatorState::ACTIVE
        );
    } else {
        assert_eq!(owner_state, ValidatorState::ACTIVE);
    }

    let new_operator_keys = ValidatorNodeKeys::new(&mut keygen);
    let new_voter_cli_index = cli.add_account_to_cli(keygen.generate_ed25519_private_key());
    let new_operator_cli_index = cli
        .create_cli_account(
            new_operator_keys.account_private_key.clone(),
            owner_cli_index,
        )
        .await
        .unwrap();

    cli.set_delegated_voter(owner_cli_index, new_voter_cli_index)
        .await
        .unwrap();
    cli.set_operator(owner_cli_index, new_operator_cli_index)
        .await
        .unwrap();

    cli.transfer_coins(
        owner_cli_index,
        new_operator_cli_index,
        operator_initial_coins,
        None,
    )
    .await
    .unwrap();

    cli.leave_validator_set(new_operator_cli_index, Some(owner_cli_index))
        .await
        .unwrap();

    let owner_state = get_validator_state(&cli, owner_cli_index).await;
    if owner_state == ValidatorState::LEAVING {
        reconfig(
            &rest_client,
            &transaction_factory,
            swarm.chain_info().root_account(),
        )
        .await;

        assert_eq!(
            get_validator_state(&cli, owner_cli_index).await,
            ValidatorState::NONE
        );
    } else {
        assert_eq!(owner_state, ValidatorState::NONE);
    }

    cli.join_validator_set(operator_cli_index, Some(owner_cli_index))
        .await
        .unwrap_err();
    assert_eq!(
        get_validator_state(&cli, owner_cli_index).await,
        ValidatorState::NONE
    );
}

fn dns_name(addr: &str) -> DnsName {
    DnsName::try_from(addr.to_string()).unwrap()
}

pub struct ValidatorNodeKeys {
    account_private_key: Ed25519PrivateKey,
    network_private_key: x25519::PrivateKey,
    consensus_private_key: bls12381::PrivateKey,
}

impl ValidatorNodeKeys {
    pub fn new(keygen: &mut KeyGen) -> Self {
        Self {
            account_private_key: keygen.generate_ed25519_private_key(),
            network_private_key: keygen.generate_x25519_private_key().unwrap(),
            consensus_private_key: keygen.generate_bls12381_private_key(),
        }
    }

    pub fn network_public_key(&self) -> x25519::PublicKey {
        self.network_private_key.public_key()
    }

    pub fn consensus_public_key(&self) -> bls12381::PublicKey {
        bls12381::PublicKey::from(&self.consensus_private_key)
    }

    pub fn consensus_proof_of_possession(&self) -> bls12381::ProofOfPossession {
        bls12381::ProofOfPossession::create(&self.consensus_private_key)
    }
}

pub async fn init_validator_account(
    cli: &mut CliTestFramework,
    keygen: &mut KeyGen,
    amount: Option<u64>,
) -> (usize, ValidatorNodeKeys) {
    let validator_node_keys = ValidatorNodeKeys::new(keygen);
    let validator_cli_index = cli
        .create_cli_account_from_faucet(validator_node_keys.account_private_key.clone(), amount)
        .await
        .unwrap();

    cli.assert_account_balance_now(validator_cli_index, amount.unwrap_or(DEFAULT_FUNDED_COINS))
        .await;
    (validator_cli_index, validator_node_keys)
}

async fn assert_validator_set_sizes(
    cli: &CliTestFramework,
    active: usize,
    joining: usize,
    leaving: usize,
) {
    let validator_set = cli.show_validator_set().await.unwrap();
    assert_eq!(
        active,
        validator_set.active_validators.len(),
        "{:?}",
        validator_set
    );
    assert_eq!(
        joining,
        validator_set.pending_active.len(),
        "{:?}",
        validator_set
    );
    assert_eq!(
        leaving,
        validator_set.pending_inactive.len(),
        "{:?}",
        validator_set
    );
}

#[derive(Debug, PartialEq, Eq)]
enum ValidatorState {
    ACTIVE,
    JOINING,
    LEAVING,
    NONE,
}

async fn get_validator_state(cli: &CliTestFramework, pool_index: usize) -> ValidatorState {
    let validator_set = cli.show_validator_set().await.unwrap();
    let pool_address = cli.account_id(pool_index);

    for (state, list) in [
        (ValidatorState::ACTIVE, &validator_set.active_validators),
        (ValidatorState::JOINING, &validator_set.pending_active),
        (ValidatorState::LEAVING, &validator_set.pending_inactive),
    ] {
        if list.iter().any(|info| info.account_address == pool_address) {
            return state;
        }
    }
    ValidatorState::NONE
}

fn get_gas(transaction: TransactionSummary) -> u64 {
    transaction.gas_used.unwrap() * transaction.gas_unit_price.unwrap()
}

#[tokio::test]
async fn test_multivalidator_staking_reward() {
    // Run the actual test in a thread with larger stack size (8MB instead of default 2MB)
    let handle = std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024) // 8MB stack
        .spawn(|| {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            runtime.block_on(async {
                test_multivalidator_staking_reward_impl().await;
            })
        })
        .unwrap();

    handle.join().unwrap();
}


async fn test_multivalidator_staking_reward_impl() {
    // Base stake amount for rewards calculation
    const BASE_STAKE: u64 = 3600u64 * 24 * 365 * 10 * 100; // with 10% APY, this gives 100 rewards per second

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, conf, _| {
            // Configure consensus parameters
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_time_ms = 100;
        }))
        .with_init_genesis_stake(Arc::new(|i, genesis_stake_amount| {
            // Set different stake amounts for each validator
            *genesis_stake_amount = match i {
                0 => 10 * BASE_STAKE,  // Validator 0: 10x base stake
                1 => 8 * BASE_STAKE,   // Validator 1: 8x base stake
                2 => 6 * BASE_STAKE,   // Validator 2: 6x base stake
                3 => 4 * BASE_STAKE,   // Validator 3: 4x base stake
                _ => BASE_STAKE,
            };
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            // Configure epochs and rewards
            genesis_config.epoch_duration_secs = 4;  // Short epochs for testing
            genesis_config.recurring_lockup_duration_secs = 4;
            genesis_config.voting_duration_secs = 3;
            genesis_config.rewards_apy_percentage = 10;  // 10% APY
        }))
        .with_aptos()
        .with_initial_features_override(create_features_with_treasury_rewards())
        .build_with_cli(0)
        .await;

    // Wait for the swarm to be fully ready
    println!("Waiting for swarm to be fully ready...");
    swarm
        .wait_for_all_nodes_to_catchup(Duration::from_secs(30))
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Add root account to CLI for governance operations
    cli.add_account_with_address_to_cli(
        swarm.root_key(),
        swarm.chain_info().root_account().address(),
    );

    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    // Note: The 10% APY is already configured in genesis_config.rewards_apy_percentage above
    // If we need to modify it, we can try using update_rewards_config instead of update_rewards_rate
    // as update_rewards_rate requires the periodical_reward_rate_decrease feature to be disabled
    println!("APR rate already configured at 10% in genesis config");

    println!("Creating treasury account...");
    let treasury_account = create_and_fund_account(&mut swarm,  2*BASE_STAKE).await;
    let treasury_address = treasury_account.address();
    println!("Treasury account created at address: {}", treasury_address);

    let deposit_treasury_txn = treasury_account.sign_with_transaction_builder(
        transaction_factory.payload(
            aptos_cached_packages::aptos_stdlib::governed_gas_pool_deposit_treasury(BASE_STAKE)
        )
    );

    println!("Depositing {} from treasury to governed gas pool...", BASE_STAKE);
    rest_client.submit_and_wait(&deposit_treasury_txn).await
        .expect("Failed to deposit from treasury to governed gas pool");

    println!("Using default staking rewards mechanism with 10% APY from genesis config...");

    // Enable the STAKE_REWARD_USING_TREASURY feature after GGP has sufficient balance
    println!("Enabling stake_reward_using_treasury feature...");
    let enable_feature_script = format!(
        r#"
    script {{
        use aptos_framework::aptos_governance;
        use std::features;
        fun main(core_resources: &signer) {{
            let framework_signer = aptos_governance::get_signer_testnet_only(core_resources, @0000000000000000000000000000000000000000000000000000000000000001);
            features::change_feature_flags_for_next_epoch(&framework_signer, vector[224], vector[]);
        }}
    }}
    "#
    );
    
    cli.run_script(0, &enable_feature_script)
        .await
        .expect("Failed to enable stake_reward_using_treasury feature");
    
    // Resync after running script
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    // Collect validator information
    let mut validators: Vec<_> = swarm.validators().collect();
    validators.sort_by_key(|v| v.name());

    let validator_addresses: Vec<_> = validators
        .iter()
        .map(|validator| validator.peer_id())
        .collect();

    // Check governed gas pool balance
    // The governed gas pool should contain the treasury deposit plus collected gas fees
    let gas_pool_balance: u64 = rest_client
        .view(
            &aptos_rest_client::aptos_api_types::ViewRequest {
                function: aptos_rest_client::aptos_api_types::EntryFunctionId::from_str("0x1::governed_gas_pool::get_balance").unwrap(),
                type_arguments: vec![aptos_rest_client::aptos_api_types::MoveType::from_str("0x1::aptos_coin::AptosCoin").unwrap()],
                arguments: vec![],
            },
            None,
        )
        .await
        .expect("GGP balance view request failed")
        .inner()
        .first()
        .unwrap()
        .as_str()
        .unwrap_or("0")
        .parse()
        .unwrap();
    // assert gas being deposited to ggp
    assert!(gas_pool_balance > BASE_STAKE, "ggp balance: {}, treasury amount: {}", gas_pool_balance, BASE_STAKE);

    // Calculate expected reward rate based on actual epoch count with 10% APY and 4-second epochs
    const SECONDS_PER_YEAR: u64 = 365 * 24 * 60 * 60; // 31,536,000
    const EPOCH_DURATION_SECS: u64 = 4;  // Must match genesis_config.epoch_duration_secs
    const APY_PERCENTAGE: u64 = 10;
    const REWARDS_RATE_DENOMINATOR: f64 = 1_000_000_000.0;
    
    let epochs_per_year = SECONDS_PER_YEAR as f64 / EPOCH_DURATION_SECS as f64;
    let rewards_rate_numerator = (APY_PERCENTAGE as f64 * REWARDS_RATE_DENOMINATOR / 100.0) / epochs_per_year;
    let per_epoch_rate = rewards_rate_numerator / REWARDS_RATE_DENOMINATOR;
    
    println!("Per-epoch reward rate: {:.10}", per_epoch_rate);

     // Get initial validator state
    let (initial_state, initial_validator_set) = get_validator_set_and_state(&rest_client).await;
    println!(
        "Initial state - Epoch: {}, Version: {}",
        initial_state.epoch, initial_state.version
    );
    println!("Initial validator voting power: {:?}", initial_validator_set);

    // Store initial voting power for comparison
    let initial_voting_powers = initial_validator_set.clone();

    // Track previous epoch state for reward verification
    let mut previous_epoch_set = initial_validator_set.clone();
    let mut final_epoch_info = None;

    // Run through 2 epochs
    for epoch_num in 1..=2 {
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Trigger epoch change
        let epoch_result = reconfig(
            &rest_client,
            &transaction_factory,
            swarm.chain_info().root_account(),
        )
        .await;
        
        // Get state after epoch
        let (epoch_state, epoch_validator_set) = get_validator_set_and_state(&rest_client).await;
        println!(
            "\n=== After Round {} (Blockchain Epoch: {}, Version: {}) ===",
            epoch_num, epoch_state.epoch, epoch_state.version
        );
        
        // Track rewards for this epoch
        let mut epoch_rewards = HashMap::new();

        // Verify rewards were distributed in this epoch with detailed logging
        println!("\nPer-Validator Rewards for Round {}:", epoch_num);
        println!("{:<70} | {:>20} | {:>20} | {:>20} | {:>12}", 
                 "Validator Address", "Previous Power", "Current Power", "Reward Earned", "Reward Rate");
        println!("{}", "-".repeat(160));
        
        for (address, current_power) in &epoch_validator_set {
            let previous_power = previous_epoch_set.get(address).unwrap();
            let reward_earned = if current_power > previous_power {
                current_power - previous_power
            } else {
                0
            };
            
            epoch_rewards.insert(*address, reward_earned);
            
            let epoch_reward_rate = reward_earned as f64 / *previous_power as f64;
            
            println!("{:<70} | {:>20} | {:>20} | {:>20} | {:>12.10}",
                     address.to_string(),
                     previous_power,
                     current_power,
                     reward_earned,
                     epoch_reward_rate);
            
            if reward_earned == 0 {
                println!("  ⚠️  WARNING: Validator {} earned NO rewards in epoch {}", address, epoch_num);
            }
        }
        
        // Update previous epoch state for next iteration
        previous_epoch_set = epoch_validator_set.clone();
        
        // Store final epoch info
        if epoch_num == 2 {
            final_epoch_info = Some((epoch_result, epoch_state, epoch_validator_set));
        }
    }

    // Extract final epoch data
    let (final_epoch_result, final_state, final_validator_set) = final_epoch_info.unwrap();
  
    // Calculate actual epoch count and expected rate
    let actual_epochs = final_epoch_result.epoch - initial_state.epoch;
    let expected_rate = per_epoch_rate * (actual_epochs as f64);
    
    println!(
        "Actual epochs passed: {}, Expected reward rate: {:.10}",
        actual_epochs, expected_rate
    );
  
    // Verify proportional reward distribution across actual epochs
    // Calculate total rewards and rates in a single pass
    let mut reward_rates: Vec<(PeerId, f64)> = Vec::new();
    let mut total_rewards_distributed = 0u64;

    for address in &validator_addresses {
        let initial_power = initial_voting_powers.get(address).unwrap();
        let final_power = final_validator_set.get(address).unwrap();
        let total_rewards = final_power - initial_power;
        total_rewards_distributed += total_rewards;  // Accumulate total as we go
        
        let reward_rate = total_rewards as f64 / *initial_power as f64;
        reward_rates.push((*address, reward_rate));

        println!(
            "Validator {} - Initial stake: {}, Final stake: {}, Total rewards: {}, Reward rate: {:.10}",
            address, initial_power, final_power, total_rewards, reward_rate
        );
        
        // Assert that the actual reward rate is close to the expected rate (within 50% tolerance)
        // We use a generous tolerance because:
        // 1. Validators may have different success rates in proposals
        // 2. There may be rounding in the on-chain calculations
        // 3. The test runs with short epochs which can introduce timing variations
        let rate_deviation = ((reward_rate - expected_rate) / expected_rate).abs();
        assert!(
            rate_deviation < 0.5,
            "Validator {} reward rate {:.10} is too far from expected {:.10} for {} epochs (deviation: {:.2}%)",
            address,
            reward_rate,
            expected_rate,
            actual_epochs,
            rate_deviation * 100.0
        );
    }

    let avg_rate: f64 = reward_rates.iter().map(|(_, r)| r).sum::<f64>() / reward_rates.len() as f64;
    for (address, rate) in &reward_rates {
        let deviation = ((rate - avg_rate) / avg_rate).abs();
        assert!(
            deviation < 0.5,
            "Validator {} reward rate {:.10} deviates too much from average {:.10} (deviation: {:.2}%)",
            address,
            rate,
            avg_rate,
            deviation * 100.0
        );
    }

    println!("All validators received proportional rewards correctly!");
    println!("Average reward rate: {:.6}", avg_rate);
    println!("Total rewards distributed across all validators: {}", total_rewards_distributed);

    // Use the CLI to analyze validator performance
    cli.analyze_validator_performance(Some(initial_state.epoch as i64), Some(final_epoch_result.epoch as i64))
        .await
        .unwrap();

    // Verify we ran through at least 2 epochs (may be more due to initialization)
    assert!(
        final_epoch_result.epoch - initial_state.epoch >= 2,
        "Should have progressed through at least 2 epochs, actual: {}",
        final_epoch_result.epoch - initial_state.epoch
    );

    // Fetch and verify WithdrawStakingRewardEvent amounts match the stake rewards
    println!("Fetching WithdrawStakingRewardEvent events...");
    let events = rest_client
        .get_account_events_bcs(
            CORE_CODE_ADDRESS,
            "0x1::governed_gas_pool::GovernedGasPoolExtension",
            "withdraw_staking_reward_events",
            None,
            None,
        )
        .await
        .expect("Failed to fetch withdraw staking reward events");

    // Deserialize events and sum up the amounts
    #[derive(Debug, serde::Deserialize)]
    struct WithdrawStakingRewardEvent {
        amount: u64,
    }

    let mut total_event_amount = 0u64;
    let mut events_in_range = 0;
    for event_with_version in events.inner() {
        let version = event_with_version.transaction_version;
        
        // Only count events that occurred between initial and final versions
        if version >= initial_state.version && version <= final_state.version {
            let event = event_with_version.event.v1()
                .expect("Failed to get v1 event");
            let withdraw_event: WithdrawStakingRewardEvent = bcs::from_bytes(event.event_data())
                .expect("Failed to deserialize WithdrawStakingRewardEvent");
            total_event_amount += withdraw_event.amount;
            events_in_range += 1;
            println!(
                "WithdrawStakingRewardEvent at version {}: amount = {}",
                version,
                withdraw_event.amount
            );
        }
    }

    println!(
        "Found {} WithdrawStakingRewardEvent(s) in version range [{}, {}]",
        events_in_range,
        initial_state.version,
        final_state.version
    );

    println!(
        "Total rewards from events: {}, Total rewards distributed: {}",
        total_event_amount, total_rewards_distributed
    );

    // Assert that the event amounts equal the stake rewards
    assert_eq!(
        total_event_amount,
        total_rewards_distributed,
        "WithdrawStakingRewardEvent total amount ({}) should equal total stake rewards distributed ({})",
        total_event_amount,
        total_rewards_distributed
    );

    println!("Successfully verified that WithdrawStakingRewardEvent amounts match stake rewards!");

    swarm.wait_for_all_nodes_to_catchup(Duration::from_secs(30))
    .await
    .unwrap();
}

// Helper function to get validator set and state
async fn get_validator_set_and_state(rest_client: &Client) -> (State, HashMap<PeerId, u64>) {
    let (validator_set, state): (ValidatorSet, State) = rest_client
        .get_account_resource_bcs(CORE_CODE_ADDRESS, "0x1::stake::ValidatorSet")
        .await
        .unwrap()
        .into_parts();

    let validator_to_voting_power = validator_set
        .active_validators
        .iter()
        .map(|v| (v.account_address, v.consensus_voting_power()))
        .collect::<HashMap<_, _>>();

    (state, validator_to_voting_power)
}

/// Creates a Features object with specific features disabled for testing
/// Disables FA migration features (treasury rewards will be enabled via governance after GGP has balance)
/// Disables:
/// - PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS
/// - NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE
/// - OPERATIONS_DEFAULT_TO_FA_APT_STORE
/// - DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE
/// - ORDERLESS_TRANSACTIONS
/// - CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION
/// - DISTRIBUTE_TRANSACTION_FEE
pub fn create_features_with_treasury_rewards() -> aptos_types::on_chain_config::Features {
    use aptos_types::on_chain_config::{Features, FeatureFlag};

    let mut features = Features::default();

    // Disable the features related to fungible asset migration
    features.disable(FeatureFlag::PRIMARY_APT_FUNGIBLE_STORE_AT_USER_ADDRESS);
    features.disable(FeatureFlag::NEW_ACCOUNTS_DEFAULT_TO_FA_APT_STORE);
    features.disable(FeatureFlag::OPERATIONS_DEFAULT_TO_FA_APT_STORE);
    features.disable(FeatureFlag::DEFAULT_TO_CONCURRENT_FUNGIBLE_BALANCE);
    features.disable(FeatureFlag::ORDERLESS_TRANSACTIONS);
    features.disable(FeatureFlag::CALCULATE_TRANSACTION_FEE_FOR_DISTRIBUTION);
    features.disable(FeatureFlag::DISTRIBUTE_TRANSACTION_FEE);
    features.disable(FeatureFlag::ACCOUNT_ABSTRACTION);
    features.disable(FeatureFlag::TRANSACTION_PAYLOAD_V2);
    features.disable(FeatureFlag::DERIVABLE_ACCOUNT_ABSTRACTION);
        
    features
}
