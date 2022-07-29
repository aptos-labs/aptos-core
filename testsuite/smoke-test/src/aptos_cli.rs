// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::smoke_test_environment::new_local_swarm_with_aptos;
use crate::test_utils::reconfig;
use aptos::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use aptos_config::{keys::ConfigKey, utils::get_available_port};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{bls12381, x25519};
use aptos_faucet::FaucetArgs;
use aptos_genesis::config::HostAndPort;
use aptos_keygen::KeyGen;
use aptos_types::{
    account_config::aptos_root_address, chain_id::ChainId, network_address::DnsName,
};
use forge::{LocalSwarm, Node, NodeExt, Swarm};
use std::convert::TryFrom;
use std::path::PathBuf;
use tokio::task::JoinHandle;

pub async fn setup_cli_test(
    num_nodes: usize,
    num_cli_accounts: usize,
) -> (LocalSwarm, CliTestFramework, JoinHandle<()>) {
    let swarm = new_local_swarm_with_aptos(num_nodes).await;
    let chain_id = swarm.chain_id();
    let validator = swarm.validators().next().unwrap();
    let root_key = swarm.root_key();
    let faucet_port = get_available_port();
    let faucet = launch_faucet(
        validator.rest_api_endpoint(),
        root_key,
        chain_id,
        faucet_port,
    );
    let faucet_endpoint: reqwest::Url =
        format!("http://localhost:{}", faucet_port).parse().unwrap();
    // Connect the operator tool to the node's JSON RPC API
    let tool = CliTestFramework::new(
        validator.rest_api_endpoint(),
        faucet_endpoint,
        num_cli_accounts,
    )
    .await;

    (swarm, tool, faucet)
}

pub fn launch_faucet(
    endpoint: reqwest::Url,
    mint_key: Ed25519PrivateKey,
    chain_id: ChainId,
    port: u16,
) -> JoinHandle<()> {
    let faucet = FaucetArgs {
        address: "127.0.0.1".to_string(),
        port,
        server_url: endpoint,
        mint_key_file_path: PathBuf::new(),
        mint_key: Some(ConfigKey::new(mint_key)),
        mint_account_address: Some(aptos_root_address()),
        chain_id,
        maximum_amount: None,
        do_not_delegate: true,
    };
    tokio::spawn(faucet.run())
}

#[tokio::test]
async fn test_account_flow() {
    let (_swarm, cli, _faucet) = setup_cli_test(1, 2).await;

    assert_eq!(
        DEFAULT_FUNDED_COINS,
        cli.wait_for_balance(0, DEFAULT_FUNDED_COINS).await.unwrap()
    );
    assert_eq!(
        DEFAULT_FUNDED_COINS,
        cli.wait_for_balance(1, DEFAULT_FUNDED_COINS).await.unwrap()
    );

    // Transfer an amount between the accounts
    let transfer_amount = 100;
    let response = cli.transfer_coins(0, 1, transfer_amount).await.unwrap();
    let expected_sender_amount =
        DEFAULT_FUNDED_COINS - response.gas_used.unwrap() - transfer_amount;
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
    let _ = cli.fund_account(0).await.unwrap();
    assert_eq!(
        expected_sender_amount,
        cli.wait_for_balance(0, expected_sender_amount)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn test_show_validator_set() {
    let (swarm, cli, _faucet) = setup_cli_test(1, 1).await;
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
    let (mut swarm, mut cli, _faucet) = setup_cli_test(1, 0).await;
    let transaction_factory = swarm.chain_info().transaction_factory();
    let rest_client = swarm.validators().next().unwrap().rest_client();

    let mut keygen = KeyGen::from_os_rng();
    let validator_cli_index = 0;
    let keys = init_validator_account(&mut cli, &mut keygen, validator_cli_index).await;
    // faucet can make our root LocalAccount sequence number get out of sync.
    swarm
        .chain_info()
        .resync_root_account_seq_num(&rest_client)
        .await
        .unwrap();

    assert!(cli.show_validator_config().await.is_err()); // validator not registered yet

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

    let validator_config = cli.show_validator_config().await.unwrap();
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

    let validator_config = cli.show_validator_config().await.unwrap();

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
    validator_cli_index: usize,
) -> ValidatorNodeKeys {
    let validator_node_keys = ValidatorNodeKeys {
        account_private_key: keygen.generate_ed25519_private_key(),
        network_private_key: keygen.generate_x25519_private_key().unwrap(),
        consensus_private_key: keygen.generate_bls12381_private_key(),
    };
    cli.init(&validator_node_keys.account_private_key)
        .await
        .unwrap();

    // Push the private key into the system
    cli.add_private_key(validator_node_keys.account_private_key.clone());

    assert_eq!(
        DEFAULT_FUNDED_COINS,
        cli.account_balance(validator_cli_index).await.unwrap()
    );
    validator_node_keys
}
