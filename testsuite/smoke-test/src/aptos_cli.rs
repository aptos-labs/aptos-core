// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::SwarmBuilder;
use crate::test_utils::reconfig;
use aptos::common::types::{GasOptions, DEFAULT_GAS_UNIT_PRICE, DEFAULT_MAX_GAS};
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{bls12381, x25519};
use aptos_genesis::config::HostAndPort;
use aptos_keygen::KeyGen;
use aptos_rest_client::Transaction;
use aptos_types::network_address::DnsName;
use forge::{NodeExt, Swarm};
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn test_account_flow() {
    let (_swarm, cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(2)
        .await;

    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(0).await.unwrap());
    assert_eq!(DEFAULT_FUNDED_COINS, cli.account_balance(1).await.unwrap());

    // Transfer an amount between the accounts
    let transfer_amount = 100;
    let response = cli
        .transfer_coins(
            0,
            1,
            transfer_amount,
            Some(GasOptions {
                gas_unit_price: DEFAULT_GAS_UNIT_PRICE * 2,
                max_gas: DEFAULT_MAX_GAS,
            }),
        )
        .await
        .unwrap();
    let expected_sender_amount =
        DEFAULT_FUNDED_COINS - (response.gas_used * response.gas_unit_price) - transfer_amount;
    let expected_receiver_amount = DEFAULT_FUNDED_COINS + transfer_amount;

    assert_eq!(
        expected_sender_amount,
        cli.wait_for_balance(0, expected_sender_amount)
            .await
            .unwrap()
    );
    assert_eq!(
        expected_receiver_amount,
        cli.wait_for_balance(1, expected_receiver_amount)
            .await
            .unwrap()
    );

    // Wait for faucet amount to be updated
    let expected_sender_amount = expected_sender_amount + DEFAULT_FUNDED_COINS;
    let _ = cli.fund_account(0, None).await.unwrap();
    assert_eq!(
        expected_sender_amount,
        cli.wait_for_balance(0, expected_sender_amount)
            .await
            .unwrap()
    );
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
async fn test_register_and_update_validator() {
    let (mut swarm, mut cli, _faucet) = SwarmBuilder::new_local(1)
        .with_aptos()
        .build_with_cli(0)
        .await;
    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();
    let (validator_cli_index, keys) = init_validator_account(&mut cli, &mut keygen).await;
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
    cli.register_validator_candidate(
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
            genesis_config.epoch_duration_secs = 3600;
            genesis_config.recurring_lockup_duration_secs = 2;
            genesis_config.min_price_per_gas_unit = 0;
        }))
        .build_with_cli(0)
        .await;

    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();
    let (validator_cli_index, keys) = init_validator_account(&mut cli, &mut keygen).await;
    let mut gas_used = 0;

    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    let port = 1234;
    gas_used += get_gas(
        cli.register_validator_candidate(
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

    assert_eq!(
        DEFAULT_FUNDED_COINS - gas_used,
        cli.account_balance(validator_cli_index).await.unwrap()
    );

    let stake_coins = 7;
    gas_used += get_gas(
        cli.add_stake(validator_cli_index, stake_coins)
            .await
            .unwrap(),
    );

    assert_eq!(
        DEFAULT_FUNDED_COINS - stake_coins - gas_used,
        cli.account_balance(validator_cli_index).await.unwrap()
    );

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

    gas_used += get_gas(cli.join_validator_set(validator_cli_index).await.unwrap());

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

    gas_used += get_gas(cli.leave_validator_set(validator_cli_index).await.unwrap());

    assert_validator_set_sizes(&cli, 1, 0, 1).await;

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    assert_validator_set_sizes(&cli, 1, 0, 0).await;

    assert_eq!(
        DEFAULT_FUNDED_COINS - stake_coins - gas_used,
        cli.account_balance(validator_cli_index).await.unwrap()
    );
    let unlock_stake = 3;

    // Unlock stake.
    gas_used += get_gas(
        cli.unlock_stake(validator_cli_index, unlock_stake)
            .await
            .unwrap(),
    );

    // Conservatively wait until the recurring lockup is over.
    tokio::time::sleep(Duration::from_secs(2)).await;

    let withdraw_stake = 2;
    gas_used += get_gas(
        cli.withdraw_stake(validator_cli_index, withdraw_stake)
            .await
            .unwrap(),
    );

    assert_eq!(
        DEFAULT_FUNDED_COINS - stake_coins + withdraw_stake - gas_used,
        cli.account_balance(validator_cli_index).await.unwrap()
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
) -> (usize, ValidatorNodeKeys) {
    let validator_node_keys = ValidatorNodeKeys {
        account_private_key: keygen.generate_ed25519_private_key(),
        network_private_key: keygen.generate_x25519_private_key().unwrap(),
        consensus_private_key: keygen.generate_bls12381_private_key(),
    };
    let validator_cli_index = cli
        .add_cli_account(validator_node_keys.account_private_key.clone())
        .await
        .unwrap();
    assert_eq!(
        DEFAULT_FUNDED_COINS,
        cli.account_balance(validator_cli_index).await.unwrap()
    );
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

fn get_gas(transaction: Transaction) -> u64 {
    *transaction.transaction_info().unwrap().gas_used.inner()
}
