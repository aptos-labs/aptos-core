// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A tool for interacting with Move
//!
//! TODO: Examples
//!

use crate::common::types::{CliConfig, EncodingOptions, NodeOptions, PrivateKeyInputOptions};
use crate::{
    common::{types::MovePackageDir, utils::to_common_result},
    CliResult, Error,
};
use aptos_crypto::PrivateKey;
use aptos_rest_client::Client;
use aptos_sdk::transaction_builder::TransactionFactory;
use aptos_sdk::types::LocalAccount;
use aptos_types::chain_id::ChainId;
use aptos_types::transaction::authenticator::AuthenticationKey;
use aptos_types::transaction::{ModuleBundle, TransactionPayload};
use aptos_vm::natives::aptos_natives;
use clap::{Parser, Subcommand};
use move_cli::package::cli::{run_move_unit_tests, UnitTestResult};
use move_core_types::account_address::AccountAddress;
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig};
use move_unit_test::UnitTestingConfig;
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
    #[clap(long)]
    chain_id: ChainId,
    #[clap(long, default = 1000)]
    max_gas: u64,
}

impl PublishPackage {
    pub async fn execute(&self) -> Result<aptos_rest_client::Transaction, Error> {
        let build_config = BuildConfig {
            generate_abis: false,
            generate_docs: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let package = compile_move(build_config, self.move_options.package_dir.as_path())?;

        // Now that it's compiled, lets send it
        let client = Client::new(self.node_options.url.clone());
        let sender_key = if let Some(private_key) = self
            .private_key_options
            .extract_private_key(self.encoding_options.encoding)?
        {
            private_key
        } else {
            let config = CliConfig::load()?;
            config.private_key.unwrap()
        };
        let sender_address = AuthenticationKey::ed25519(&sender_key.public_key()).derived_address();
        let sender_address = AccountAddress::new(*sender_address);

        let account_response = client
            .get_account(sender_address)
            .await
            .map_err(|err| Error::UnexpectedError(err.to_string()))?;
        let account = account_response.inner();

        let transaction_factory = TransactionFactory::new(self.chain_id)
            .with_gas_unit_price(1)
            .with_max_gas_amount(self.max_gas);
        let sender_account =
            &mut LocalAccount::new(sender_address, sender_key, account.sequence_number);

        let compiled_units: Vec<Vec<u8>> = package
            .compiled_units
            .iter()
            .map(|unit_with_source| unit_with_source.unit.serialize())
            .collect();

        let payload = TransactionPayload::ModuleBundle(ModuleBundle::new(compiled_units));
        let transaction =
            sender_account.sign_with_transaction_builder(transaction_factory.payload(payload));
        let response = client
            .submit_and_wait(&transaction)
            .await
            .map_err(|err| Error::UnexpectedError(err.to_string()))?;

        Ok(response.inner().clone())
    }
}
