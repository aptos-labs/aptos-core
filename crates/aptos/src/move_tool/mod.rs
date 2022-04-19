// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A tool for interacting with Move
//!
//! TODO: Examples
//!

use crate::{
    common::{
        types::{EncodingOptions, MovePackageDir, NodeOptions, PrivateKeyInputOptions},
        utils::to_common_result,
    },
    CliResult, Error,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_rest_client::{Client, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    chain_id::ChainId,
    transaction::{authenticator::AuthenticationKey, ModuleBundle, TransactionPayload},
};
use aptos_vm::natives::aptos_natives;
use clap::{Parser, Subcommand};
use move_cli::package::cli::{run_move_unit_tests, UnitTestResult};
use move_core_types::account_address::AccountAddress;
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig};
use move_unit_test::UnitTestingConfig;
use reqwest::Url;
use std::path::Path;

/// CLI tool for performing Move tasks
///
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    Publish(PublishPackage),
    Test(TestPackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => to_common_result(tool.execute().await),
            MoveTool::Publish(tool) => to_common_result(tool.execute().await),
            MoveTool::Test(tool) => to_common_result(tool.execute().await),
        }
    }
}

/// Compiles a package and returns the [`ModuleId`]s
#[derive(Parser)]
pub struct CompilePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

impl CompilePackage {
    pub async fn execute(&self) -> Result<Vec<String>, Error> {
        let build_config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses.clone(),
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let compiled_package = compile_move(build_config, self.move_options.package_dir.as_path())?;
        let mut ids = Vec::new();
        compiled_package
            .compiled_modules()
            .iter_modules()
            .iter()
            .for_each(|module| ids.push(module.self_id().to_string()));
        Ok(ids)
    }
}

/// Run Move unit tests against a package path
#[derive(Parser)]
pub struct TestPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

impl TestPackage {
    pub async fn execute(&self) -> Result<&'static str, Error> {
        let config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses.clone(),
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let result = run_move_unit_tests(
            self.move_options.package_dir.as_path(),
            config,
            UnitTestingConfig::default_with_bound(Some(100_000)),
            aptos_natives(),
            false,
        )
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;

        // TODO: commit back up to the move repo
        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Ok("Failure"),
        }
    }
}

/// Compiles a Move package dir, and returns the compiled modules.
fn compile_move(build_config: BuildConfig, package_dir: &Path) -> Result<CompiledPackage, Error> {
    // TODO: Add caching
    build_config
        .compile_package(package_dir, &mut Vec::new())
        .map_err(|err| Error::MoveCompiliationError(err.to_string()))
}

/// Publishes the modules in a Move package
#[derive(Parser)]
pub struct PublishPackage {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    move_options: MovePackageDir,
    #[clap(flatten)]
    node_options: NodeOptions,
    /// ChainId for the network
    #[clap(long)]
    chain_id: ChainId,
    /// Maximum gas to be used to publish the package
    ///
    /// Defaults to 1000 gas units
    #[clap(long, default_value_t = 1000)]
    max_gas: u64,
}

impl PublishPackage {
    pub async fn execute(&self) -> Result<aptos_rest_client::Transaction, Error> {
        let build_config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses.clone(),
            generate_abis: false,
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let package = compile_move(build_config, self.move_options.package_dir.as_path())?;
        let compiled_units: Vec<Vec<u8>> = package
            .compiled_units
            .iter()
            .map(|unit_with_source| unit_with_source.unit.serialize())
            .collect();
        let compiled_payload = TransactionPayload::ModuleBundle(ModuleBundle::new(compiled_units));

        // Now that it's compiled, lets send it
        let sender_key = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding)?;

        submit_transaction(
            self.node_options.url.clone(),
            self.chain_id,
            sender_key,
            compiled_payload,
            self.max_gas,
        )
        .await
    }
}

/// Submits a [`TransactionPayload`] as signed by the `sender_key`
async fn submit_transaction(
    url: Url,
    chain_id: ChainId,
    sender_key: Ed25519PrivateKey,
    payload: TransactionPayload,
    max_gas: u64,
) -> Result<Transaction, Error> {
    let client = Client::new(url);

    // Get sender address
    let sender_address = AuthenticationKey::ed25519(&sender_key.public_key()).derived_address();
    let sender_address = AccountAddress::new(*sender_address);

    // Get account to get the sequence number
    let account_response = client
        .get_account(sender_address)
        .await
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;
    let account = account_response.inner();
    let sequence_number = account.sequence_number;

    let transaction_factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(1)
        .with_max_gas_amount(max_gas);
    let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
    let transaction =
        sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
    let response = client
        .submit_and_wait(&transaction)
        .await
        .map_err(|err| Error::UnexpectedError(err.to_string()))?;

    Ok(response.inner().clone())
}
