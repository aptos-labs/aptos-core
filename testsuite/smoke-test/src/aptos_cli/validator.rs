// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use aptos::common::types::TransactionSummary;
use aptos::node::analyze::analyze_validators::{AnalyzeValidators, EpochStats};
use aptos::node::analyze::fetch_metadata::FetchMetadata;
use aptos::test::ValidatorPerformance;
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{bls12381, x25519};
use aptos_genesis::config::HostAndPort;
use aptos_keygen::KeyGen;
use aptos_rest_client::{Client, State};
use aptos_types::account_config::CORE_CODE_ADDRESS;
use aptos_types::network_address::DnsName;
use aptos_types::on_chain_config::ValidatorSet;
use aptos_types::PeerId;
use forge::{reconfig, NodeExt, Swarm, SwarmExt};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_analyze_validators() {
    let (mut swarm, cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_i, _conf, genesis_stake_amount| {
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

#[tokio::test]
async fn test_large_total_stake() {
    // just barelly below u64::MAX
    const BASE: u64 = 10_000_000_000_000_000_000;
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|_, _, genesis_stake_amount| {
            // make sure we have quorum
            *genesis_stake_amount = BASE;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 4;
            genesis_config.recurring_lockup_duration_secs = 4;
            genesis_config.voting_duration_secs = 3;
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
        .wait_for_all_nodes_to_catchup(Duration::from_secs(20))
        .await
        .unwrap();
}

#[tokio::test]
async fn test_nodes_rewards() {
    // with 10% APY, BASE amount gives 100 rewards per second
    const BASE: u64 = 3600u64 * 24 * 365 * 10 * 100;

    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(4)
        .with_init_config(Arc::new(|i, conf, genesis_stake_amount| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_count = 4;
            conf.api.failpoints_enabled = true;

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
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
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
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_i, conf, genesis_stake_amount| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_count = 4;
            *genesis_stake_amount = 100000;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 5;
            genesis_config.recurring_lockup_duration_secs = 10;
            genesis_config.voting_duration_secs = 5;
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
            .unwrap(),
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
            .unwrap(),
    );

    // Conservatively wait until the recurring lockup is over.
    tokio::time::sleep(Duration::from_secs(10)).await;

    let withdraw_stake = 2;
    gas_used += get_gas(
        cli.withdraw_stake(validator_cli_index, withdraw_stake)
            .await
            .unwrap(),
    );

    cli.assert_account_balance_now(
        validator_cli_index,
        (3 * DEFAULT_FUNDED_COINS) - stake_coins + withdraw_stake - gas_used,
    )
    .await;
}

#[tokio::test]
async fn test_owner_create_and_delegate_flow() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .with_init_config(Arc::new(|_i, conf, genesis_stake_amount| {
            // reduce timeout, as we will have dead node during rounds
            conf.consensus.round_initial_timeout_ms = 200;
            conf.consensus.quorum_store_poll_count = 4;
            // enough for quorum
            *genesis_stake_amount = 5000000;
        }))
        .with_init_genesis_config(Arc::new(|genesis_config| {
            genesis_config.allow_new_validators = true;
            genesis_config.epoch_duration_secs = 5;
            genesis_config.recurring_lockup_duration_secs = 10;
            genesis_config.voting_duration_secs = 5;
            genesis_config.min_stake = 500000
        }))
        .build_with_cli(0)
        .await;

    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();

    let owner_initial_coins = 1100000;
    let voter_initial_coins = 10000;
    let operator_initial_coins = 10000;

    // Owner of the coins receives coins
    let owner_cli_index = cli
        .create_cli_account_from_faucet(
            keygen.generate_ed25519_private_key(),
            Some(owner_initial_coins),
        )
        .await
        .unwrap();

    cli.assert_account_balance_now(owner_cli_index, owner_initial_coins)
        .await;

    // faucet can make our root LocalAccount sequence number get out of sync.
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

    // Voter and operator start with no coins
    // Owner needs to send small amount of coins to operator and voter, to create their accounts and so they have enough for gas fees.
    owner_gas += cli
        .transfer_coins(owner_cli_index, voter_cli_index, voter_initial_coins, None)
        .await
        .unwrap()
        .gas_used;
    owner_gas += cli
        .transfer_coins(
            owner_cli_index,
            operator_cli_index,
            operator_initial_coins,
            None,
        )
        .await
        .unwrap()
        .gas_used;

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

    cli.assert_account_balance_now(
        owner_cli_index,
        owner_initial_coins
            - voter_initial_coins
            - operator_initial_coins
            - stake_amount
            - owner_gas,
    )
    .await;

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

    cli.assert_account_balance_now(operator_cli_index, operator_initial_coins - operator_gas)
        .await;

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

struct ValidatorNodeKeys {
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

async fn init_validator_account(
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
