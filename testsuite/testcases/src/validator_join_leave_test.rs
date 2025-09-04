// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{LoadDestination, NetworkLoadTest};
use velor::{account::create::DEFAULT_FUNDED_COINS, test::CliTestFramework};
use velor_forge::{
    reconfig, NetworkContext, NetworkContextSynchronizer, NetworkTest, NodeExt, Result, Swarm,
    SwarmExt, Test, TestReport, FORGE_KEY_SEED,
};
use velor_keygen::KeyGen;
use velor_sdk::crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use velor_types::{account_address::AccountAddress, transaction::authenticator::AuthenticationKey};
use async_trait::async_trait;
use log::info;
use std::{sync::Arc, time::Duration};

const MAX_NODE_LAG_SECS: u64 = 360;

pub struct ValidatorJoinLeaveTest;

impl Test for ValidatorJoinLeaveTest {
    fn name(&self) -> &'static str {
        "validator join and leave sets"
    }
}

#[async_trait]
impl NetworkLoadTest for ValidatorJoinLeaveTest {
    async fn setup<'a>(&self, _ctx: &mut NetworkContext<'a>) -> Result<LoadDestination> {
        Ok(LoadDestination::FullnodesOtherwiseValidators)
    }

    async fn test(
        &self,
        swarm: Arc<tokio::sync::RwLock<Box<dyn Swarm>>>,
        _report: &mut TestReport,
        duration: Duration,
    ) -> Result<()> {
        // Verify we have at least 7 validators (i.e., 3f+1, where f is 2)
        // so we can lose 2 validators but still make progress.
        let num_validators = { swarm.read().await.validators().count() };
        if num_validators < 7 {
            return Err(anyhow::format_err!(
                "ValidatorSet leaving and rejoining test require at least 7 validators! Given: {:?}.",
                num_validators
            ));
        }

        let faucet_endpoint: reqwest::Url = "http://localhost:8081".parse().unwrap();
        // Connect the operator tool to the node's JSON RPC API
        let transaction_factory = { swarm.read().await.chain_info().transaction_factory() };

        let (rest_client, rest_api_endpoint) = {
            let swarm = swarm.read().await;
            let first_validator = swarm.validators().next().unwrap();
            let rest_client = first_validator.rest_client();
            let rest_api_endpoint = first_validator.rest_api_endpoint();
            (rest_client, rest_api_endpoint)
        };
        let mut cli = CliTestFramework::new(
            rest_api_endpoint,
            faucet_endpoint,
            /*num_cli_accounts=*/ 0,
        )
        .await;

        let mut public_info = { swarm.read().await.chain_info().into_velor_public_info() };

        let mut validator_cli_indices = Vec::new();

        let starting_seed_in_decimal = i64::from_str_radix(FORGE_KEY_SEED, 16)?;

        for i in 0..num_validators {
            // Initialize keyGen to get validator private keys. We uses the same seed in the test
            // driver as in the genesis script so that the validator keys are deterministic.
            let mut seed_slice = [0u8; 32];
            let seed_in_decimal = starting_seed_in_decimal + (i as i64);
            let seed_in_hex_string = format!("{seed_in_decimal:0>64x}");

            hex::decode_to_slice(seed_in_hex_string, &mut seed_slice)?;

            let mut keygen = KeyGen::from_seed(seed_slice);

            let (validator_cli_index, keys) = init_validator_account(&mut cli, &mut keygen).await;

            let auth_key = AuthenticationKey::ed25519(&keys.account_private_key.public_key());
            let validator_account_address = AccountAddress::new(*auth_key.account_address());

            public_info
                .mint(validator_account_address, DEFAULT_FUNDED_COINS)
                .await
                .unwrap();

            let account_balance = public_info.get_balance(validator_account_address).await;
            assert_eq!(account_balance, DEFAULT_FUNDED_COINS);
            validator_cli_indices.push(validator_cli_index);

            assert_eq!(
                get_validator_state(&cli, validator_cli_index).await,
                ValidatorState::ACTIVE
            );
        }

        // Log the test setup
        info!(
            "Running validator join and leave test {:?} with {:?} validators.",
            self.name(),
            num_validators,
        );

        // Wait for all nodes to synchronize and stabilize.
        info!("Waiting for the validators to be synchronized.");
        {
            swarm
                .read()
                .await
                .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
                .await?;
        }

        // Wait for 1/3 of the test duration.
        tokio::time::sleep(duration / 3).await;

        // 1/3 validators leave the validator set.
        info!("Make the last 1/3 validators leave the validator set!");
        for operator_index in validator_cli_indices.iter().rev().take(num_validators / 3) {
            cli.leave_validator_set(*operator_index, None)
                .await
                .unwrap();

            let root_account = swarm.read().await.chain_info().root_account();
            reconfig(&rest_client, &transaction_factory, root_account).await;
        }

        {
            let root_account = swarm.read().await.chain_info().root_account();
            reconfig(&rest_client, &transaction_factory, root_account).await;
        }

        // Wait for 1/3 of the test duration.
        tokio::time::sleep(duration / 3).await;

        // Rejoining validator set.
        info!("Make the last 1/3 validators rejoin the validator set!");
        for operator_index in validator_cli_indices.iter().rev().take(num_validators / 3) {
            cli.join_validator_set(*operator_index, None).await.unwrap();

            let root_account = swarm.read().await.chain_info().root_account();
            reconfig(&rest_client, &transaction_factory, root_account).await;
        }

        {
            let root_account = swarm.read().await.chain_info().root_account();
            reconfig(&rest_client, &transaction_factory, root_account).await;
        }

        // Wait for all nodes to synchronize and stabilize.
        info!("Waiting for the validators to be synchronized.");
        swarm
            .read()
            .await
            .wait_for_all_nodes_to_catchup(Duration::from_secs(MAX_NODE_LAG_SECS))
            .await?;

        Ok(())
    }
}

#[async_trait]
impl NetworkTest for ValidatorJoinLeaveTest {
    async fn run<'a>(&self, ctx: NetworkContextSynchronizer<'a>) -> Result<()> {
        <dyn NetworkLoadTest>::run(self, ctx).await
    }
}

#[derive(Debug, PartialEq, Eq)]
enum ValidatorState {
    ACTIVE,
    JOINING,
    LEAVING,
    NONE,
}

struct ValidatorNodeKeys {
    account_private_key: Ed25519PrivateKey,
}

impl ValidatorNodeKeys {
    pub fn new(keygen: &mut KeyGen) -> Self {
        Self {
            account_private_key: keygen.generate_ed25519_private_key(),
        }
    }
}

async fn init_validator_account(
    cli: &mut CliTestFramework,
    keygen: &mut KeyGen,
) -> (usize, ValidatorNodeKeys) {
    let validator_node_keys = ValidatorNodeKeys::new(keygen);
    let validator_cli_index =
        cli.add_account_to_cli(validator_node_keys.account_private_key.clone());
    (validator_cli_index, validator_node_keys)
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
