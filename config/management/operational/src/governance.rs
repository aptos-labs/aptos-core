// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{auto_validate::AutoValidate, rest_client::RestClient, TransactionContext};
use aptos_global_constants::APTOS_ROOT_KEY;
use aptos_management::{
    config::{Config, ConfigPath},
    error::Error,
    secure_backend::ValidatorBackend,
    transaction::build_raw_transaction,
};
use aptos_transaction_builder::aptos_stdlib as transaction_builder;
use aptos_types::{
    account_address::AccountAddress,
    account_config::aptos_root_address,
    chain_id::ChainId,
    transaction::{authenticator::AuthenticationKey, ScriptFunction, TransactionPayload},
};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
pub struct CreateAccount {
    #[structopt(flatten)]
    config: ConfigPath,
    #[structopt(long)]
    name: String,
    #[structopt(long)]
    path_to_key: PathBuf,
    #[structopt(long, required_unless = "config")]
    json_server: Option<String>,
    #[structopt(long, required_unless("config"))]
    chain_id: Option<ChainId>,
    #[structopt(flatten)]
    validator_backend: ValidatorBackend,
    #[structopt(flatten)]
    auto_validate: AutoValidate,
}

impl CreateAccount {
    async fn execute(
        self,
        script_callback: fn(account_address: AccountAddress, name: Vec<u8>) -> TransactionPayload,
        action: &'static str,
    ) -> Result<(TransactionContext, AccountAddress), Error> {
        let config = self
            .config
            .load()?
            .override_chain_id(self.chain_id)
            .override_json_server(&self.json_server)
            .override_validator_backend(&self.validator_backend.validator_backend)?;

        let key = aptos_management::read_key_from_file(&self.path_to_key)
            .map_err(|e| Error::UnableToReadFile(format!("{:?}", self.path_to_key), e))?;
        let client = RestClient::new(config.json_server.clone());

        let seq_num = client.sequence_number(aptos_root_address()).await?;
        let auth_key = AuthenticationKey::ed25519(&key);
        let account_address = auth_key.derived_address();
        let script =
            script_callback(account_address, self.name.as_bytes().to_vec()).into_script_function();
        let mut transaction_context =
            build_and_submit_aptos_root_transaction(&config, seq_num, script, action).await?;

        // Perform auto validation if required
        transaction_context = self
            .auto_validate
            .execute(config.json_server, transaction_context)
            .await?;

        Ok((transaction_context, account_address))
    }
}

#[derive(Debug, StructOpt)]
pub struct CreateValidator {
    #[structopt(flatten)]
    input: CreateAccount,
}

impl CreateValidator {
    pub async fn execute(self) -> Result<(TransactionContext, AccountAddress), Error> {
        self.input
            .execute(
                transaction_builder::encode_validator_set_script_create_validator_account,
                "create-validator",
            )
            .await
    }
}

#[derive(Debug, StructOpt)]
pub struct CreateValidatorOperator {
    #[structopt(flatten)]
    input: CreateAccount,
}

impl CreateValidatorOperator {
    pub async fn execute(self) -> Result<(TransactionContext, AccountAddress), Error> {
        self.input
            .execute(
                transaction_builder::encode_validator_set_script_create_validator_operator_account,
                "create-validator-operator",
            )
            .await
    }
}

#[derive(Debug, StructOpt)]
struct RootValidatorOperation {
    #[structopt(long, help = "The validator address")]
    account_address: AccountAddress,
    /// JSON-RPC Endpoint (e.g. http://localhost:8080)
    #[structopt(long, required_unless = "config")]
    json_server: Option<String>,
    #[structopt(flatten)]
    validator_config: aptos_management::validator_config::ValidatorConfig,
    #[structopt(flatten)]
    auto_validate: AutoValidate,
}

impl RootValidatorOperation {
    fn config(&self) -> Result<Config, Error> {
        Ok(self
            .validator_config
            .config()?
            .override_json_server(&self.json_server))
    }
}

#[derive(Debug, StructOpt)]
pub struct AddValidator {
    #[structopt(flatten)]
    input: RootValidatorOperation,
}

impl AddValidator {
    pub async fn execute(self) -> Result<TransactionContext, Error> {
        let config = self.input.config()?;
        let client = RestClient::new(config.json_server.clone());

        // Verify that this is a configured validator
        client.validator_config(self.input.account_address).await?;

        let seq_num = client.sequence_number(aptos_root_address()).await?;
        let script = transaction_builder::encode_validator_set_script_add_validator(
            self.input.account_address,
        )
        .into_script_function();
        let mut transaction_context =
            build_and_submit_aptos_root_transaction(&config, seq_num, script, "add-validator")
                .await?;

        // Perform auto validation if required
        transaction_context = self
            .input
            .auto_validate
            .execute(config.json_server, transaction_context)
            .await?;

        Ok(transaction_context)
    }
}

#[derive(Debug, StructOpt)]
pub struct RemoveValidator {
    #[structopt(flatten)]
    input: RootValidatorOperation,
}

impl RemoveValidator {
    pub async fn execute(self) -> Result<TransactionContext, Error> {
        let config = self.input.config()?;
        let client = RestClient::new(config.json_server.clone());

        // Verify that this is a validator within the set
        client
            .validator_set(Some(self.input.account_address))
            .await?;

        let seq_num = client.sequence_number(aptos_root_address()).await?;
        let script = transaction_builder::encode_validator_set_script_remove_validator(
            self.input.account_address,
        )
        .into_script_function();

        let mut transaction_context =
            build_and_submit_aptos_root_transaction(&config, seq_num, script, "remove-validator")
                .await?;

        // Perform auto validation if required
        transaction_context = self
            .input
            .auto_validate
            .execute(config.json_server, transaction_context)
            .await?;

        Ok(transaction_context)
    }
}

async fn build_and_submit_aptos_root_transaction(
    config: &Config,
    seq_num: u64,
    script_function: ScriptFunction,
    action: &'static str,
) -> Result<TransactionContext, Error> {
    let txn = build_raw_transaction(
        config.chain_id,
        aptos_root_address(),
        seq_num,
        script_function,
    );

    let mut storage = config.validator_backend();
    let signed_txn = storage.sign(APTOS_ROOT_KEY, action, txn)?;

    let client = RestClient::new(config.json_server.clone());
    client.submit_transaction(signed_txn).await
}
