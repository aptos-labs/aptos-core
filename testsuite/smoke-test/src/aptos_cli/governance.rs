// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::aptos_cli::validator::ValidatorNodeKeys;
use crate::smoke_test_environment::SwarmBuilder;
use crate::test_utils::reconfig;
use aptos::governance::ScriptHash;
use aptos_genesis::config::HostAndPort;
use aptos_keygen::KeyGen;
use aptos_rest_client::aptos_api_types::{Address, IdentifierWrapper, MoveStructTag, MoveType};
use aptos_rest_client::Client as RestClient;
use aptos_rest_client::Transaction;
use aptos_types::account_config::CORE_CODE_ADDRESS;
use aptos_types::network_address::DnsName;
use forge::{NodeExt, Swarm};
use std::convert::TryFrom;
use std::str::FromStr;
use std::sync::Arc;

const PACKAGE_NAME: &str = "AwesomePackage";
const SCRIPT_NAME: &str = "main";
const MIN_STAKE: u64 = 500000;

#[tokio::test]
async fn test_proposal() {
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
            // lockup duration need to be large enough to vote
            genesis_config.recurring_lockup_duration_secs = 100;
            // for early proposal execution
            genesis_config.voting_duration_secs = 1;
            genesis_config.min_stake = MIN_STAKE;
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

    cli.transfer_coins(owner_cli_index, voter_cli_index, voter_initial_coins, None)
        .await
        .unwrap();
    cli.transfer_coins(
        owner_cli_index,
        operator_cli_index,
        operator_initial_coins,
        None,
    )
    .await
    .unwrap();

    let stake_amount = 1000000;
    cli.initialize_stake_owner(
        owner_cli_index,
        stake_amount,
        Some(voter_cli_index),
        Some(operator_cli_index),
    )
    .await
    .unwrap();

    let port = 6543;

    cli.update_consensus_key(
        operator_cli_index,
        Some(owner_cli_index),
        operator_keys.consensus_public_key(),
        operator_keys.consensus_proof_of_possession(),
    )
    .await
    .unwrap();

    cli.update_validator_network_addresses(
        operator_cli_index,
        Some(owner_cli_index),
        HostAndPort {
            host: DnsName::try_from("0.0.0.0".to_string()).unwrap(),
            port,
        },
        operator_keys.network_public_key(),
    )
    .await
    .unwrap();

    cli.join_validator_set(operator_cli_index, Some(owner_cli_index))
        .await
        .unwrap();

    reconfig(
        &rest_client,
        &transaction_factory,
        swarm.chain_info().root_account(),
    )
    .await;

    // Init package should not fail, otherwise panic.
    cli.init_move_dir();
    cli.init_proposal_package(PACKAGE_NAME, SCRIPT_NAME).await;
    let hash = match cli.prepare_proposal().await {
        Ok(ScriptHash { hash, bytecode }) => {
            assert!(!hash.is_empty());
            assert!(!bytecode.is_empty());
            hash
        }
        Err(err) => panic!("Error preparing proposal: {:?}", err),
    };

    // Now we are ready to propose
    let transaction = cli
        .submit_proposal(
            voter_cli_index,
            cli.account_id(owner_cli_index).to_hex_literal(),
            hash.clone(),
        )
        .await
        .unwrap();

    let events = match transaction {
        Transaction::UserTransaction(ref txn) => txn
            .events
            .iter()
            .filter(|event| {
                event.typ
                    == MoveType::Struct(MoveStructTag::new(
                        Address::from_str("0x1").unwrap(),
                        IdentifierWrapper::from_str("aptos_governance").unwrap(),
                        IdentifierWrapper::from_str("CreateProposalEvent").unwrap(),
                        vec![],
                    ))
            })
            .collect::<Vec<_>>(),
        _ => panic!("Encountered unexpected transaction type"),
    };

    assert_eq!(events.len(), 1);

    let event = *events.get(0).unwrap();

    let proposal_id = event.data["proposal_id"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap();

    // Now lets vote
    let transaction = cli
        .submit_vote(
            voter_cli_index,
            cli.account_id(owner_cli_index).to_hex_literal(),
            proposal_id,
        )
        .await
        .unwrap();

    match transaction {
        Transaction::UserTransaction(ref txn) => assert!(txn.info.success),
        _ => panic!("Encountered unexpected transaction type"),
    };

    assert_eq!(min_stake(&rest_client).await, MIN_STAKE);

    let transaction = cli
        .execute_proposal(
            voter_cli_index,
            format!(
                "{}/build/{}/bytecode_scripts/{}.mv",
                cli.move_dir().display(),
                PACKAGE_NAME,
                SCRIPT_NAME
            )
            .as_str(),
            proposal_id,
        )
        .await
        .unwrap();

    match transaction {
        Transaction::UserTransaction(ref txn) => assert!(txn.info.success),
        _ => panic!("Encountered unexpected transaction type"),
    };

    assert_ne!(min_stake(&rest_client).await, MIN_STAKE);
}

async fn min_stake(client: &RestClient) -> u64 {
    let stake_config = client
        .get_account_resource(CORE_CODE_ADDRESS, "0x1::staking_config::StakingConfig")
        .await
        .unwrap()
        .into_inner()
        .unwrap()
        .data;

    stake_config["minimum_stake"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap()
}
