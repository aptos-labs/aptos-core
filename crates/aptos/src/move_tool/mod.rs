// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A tool for interacting with Move
//!
//! TODO: Examples
//!

use crate::{
    common::{
        types::{
            load_account_arg, CliError, CliTypedResult, EncodingOptions, MovePackageDir,
            ProfileOptions, WriteTransactionOptions,
        },
        utils::to_common_result,
    },
    CliResult,
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_rest_client::{aptos_api_types::MoveType, Client, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_types::{
    chain_id::ChainId,
    transaction::{
        authenticator::AuthenticationKey, ModuleBundle, ScriptFunction, TransactionPayload,
    },
};
use aptos_vm::natives::aptos_natives;
use clap::{Parser, Subcommand};
use move_cli::package::cli::{run_move_unit_tests, UnitTestResult};
use move_core_types::{
    account_address::AccountAddress,
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use move_package::{compilation::compiled_package::CompiledPackage, BuildConfig};
use move_unit_test::UnitTestingConfig;
use reqwest::Url;
use std::{convert::TryFrom, path::Path, str::FromStr};

/// CLI tool for performing Move tasks
///
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    Publish(PublishPackage),
    Run(RunFunction),
    Test(TestPackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => {
                to_common_result("CompilePackage", tool.execute().await).await
            }
            MoveTool::Publish(tool) => {
                to_common_result("PublishPackage", tool.execute().await).await
            }
            MoveTool::Run(tool) => to_common_result("RunFunction", tool.execute().await).await,
            MoveTool::Test(tool) => to_common_result("TestPackage", tool.execute().await).await,
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
    pub async fn execute(self) -> CliTypedResult<Vec<String>> {
        let build_config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
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
    pub async fn execute(self) -> CliTypedResult<&'static str> {
        let config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
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
        .map_err(|err| CliError::MoveTestError(err.to_string()))?;

        // TODO: commit back up to the move repo
        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Ok("Failure"),
        }
    }
}

/// Compiles a Move package dir, and returns the compiled modules.
fn compile_move(build_config: BuildConfig, package_dir: &Path) -> CliTypedResult<CompiledPackage> {
    // TODO: Add caching
    build_config
        .compile_package(package_dir, &mut Vec::new())
        .map_err(|err| CliError::MoveCompilationError(err.to_string()))
}

/// Publishes the modules in a Move package
#[derive(Parser)]
pub struct PublishPackage {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    move_options: MovePackageDir,
    #[clap(flatten)]
    write_options: WriteTransactionOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
}

impl PublishPackage {
    pub async fn execute(self) -> CliTypedResult<aptos_rest_client::Transaction> {
        let build_config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
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
        let sender_key = self.write_options.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        submit_transaction(
            self.write_options
                .rest_options
                .url(&self.profile_options.profile)?,
            self.write_options
                .chain_id(&self.profile_options.profile)
                .await?,
            sender_key,
            compiled_payload,
            self.write_options.max_gas,
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
) -> CliTypedResult<Transaction> {
    let client = Client::new(url);

    // Get sender address
    let sender_address = AuthenticationKey::ed25519(&sender_key.public_key()).derived_address();
    let sender_address = AccountAddress::new(*sender_address);

    // Get account to get the sequence number
    let account_response = client
        .get_account(sender_address)
        .await
        .map_err(|err| CliError::ApiError(err.to_string()))?;
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
        .map_err(|err| CliError::ApiError(err.to_string()))?;

    Ok(response.inner().clone())
}

/// Run a Move function
#[derive(Parser)]
pub struct RunFunction {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    write_options: WriteTransactionOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::Message::set_message`
    #[clap(long, parse(try_from_str = parse_function_name))]
    function_id: FunctionId,
    /// Hex encoded arguments separated by spaces.
    ///
    /// Example: `0x01 0x02 0x03`
    #[clap(long, multiple_values = true)]
    args: Vec<ArgWithType>,
    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u64 u128 bool address vector true false signer`
    #[clap(long, multiple_values = true)]
    type_args: Vec<MoveType>,
}

impl RunFunction {
    pub async fn execute(self) -> Result<Transaction, CliError> {
        let args: Vec<Vec<u8>> = self
            .args
            .iter()
            .map(|arg_with_type| arg_with_type.arg.clone())
            .collect();
        let mut type_args: Vec<TypeTag> = Vec::new();

        // These TypeArgs are used for generics
        for type_arg in self.type_args.iter().cloned() {
            let type_tag = TypeTag::try_from(type_arg)
                .map_err(|err| CliError::UnableToParse("--type-args", err.to_string()))?;
            type_args.push(type_tag)
        }

        let script_function = ScriptFunction::new(
            self.function_id.module_id.clone(),
            self.function_id.function_id.clone(),
            type_args,
            args,
        );

        submit_transaction(
            self.write_options
                .rest_options
                .url(&self.profile_options.profile)?,
            self.write_options
                .chain_id(&self.profile_options.profile)
                .await?,
            self.write_options.private_key_options.extract_private_key(
                self.encoding_options.encoding,
                &self.profile_options.profile,
            )?,
            TransactionPayload::ScriptFunction(script_function),
            self.write_options.max_gas,
        )
        .await
    }
}

#[derive(Clone, Debug)]
enum FunctionArgType {
    Address,
    Bool,
    Hex,
    String,
    U8,
    U64,
    U128,
}

impl FunctionArgType {
    fn parse_arg(&self, arg: &str) -> CliTypedResult<Vec<u8>> {
        match self {
            FunctionArgType::Address => bcs::to_bytes(
                &AccountAddress::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("address", err.to_string()))?,
            ),
            FunctionArgType::Bool => bcs::to_bytes(
                &bool::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("bool", err.to_string()))?,
            ),
            FunctionArgType::Hex => bcs::to_bytes(
                &hex::decode(arg).map_err(|err| CliError::UnableToParse("hex", err.to_string()))?,
            ),
            FunctionArgType::String => bcs::to_bytes(arg),
            FunctionArgType::U8 => bcs::to_bytes(
                &u8::from_str(arg).map_err(|err| CliError::UnableToParse("u8", err.to_string()))?,
            ),
            FunctionArgType::U64 => bcs::to_bytes(
                &u64::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u64", err.to_string()))?,
            ),
            FunctionArgType::U128 => bcs::to_bytes(
                &u128::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u128", err.to_string()))?,
            ),
        }
        .map_err(|err| CliError::BCS("arg", err))
    }
}

impl FromStr for FunctionArgType {
    type Err = CliError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "address" => Ok(FunctionArgType::Address),
            "bool" => Ok(FunctionArgType::Bool),
            "hex" => Ok(FunctionArgType::Hex),
            "string" => Ok(FunctionArgType::String),
            "u8" => Ok(FunctionArgType::U8),
            "u64" => Ok(FunctionArgType::U64),
            "u128" => Ok(FunctionArgType::U128),
            str => Err(CliError::CommandArgumentError(format!("Invalid arg type '{}'.  Must be one of: ['address','bool','hex','string','u8','u64','u128']", str))),
        }
    }
}

/// A parseable arg with a type separated by a colon
pub struct ArgWithType {
    _ty: FunctionArgType,
    arg: Vec<u8>,
}

impl FromStr for ArgWithType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<_> = s.split(':').collect();
        if parts.len() != 2 {
            return Err(CliError::CommandArgumentError(
                "Arguments must be pairs of <type>:<arg> e.g. bool:true".to_string(),
            ));
        }

        let ty = FunctionArgType::from_str(parts.first().unwrap())?;
        let arg = parts.last().unwrap();
        let arg = ty.parse_arg(arg)?;

        Ok(ArgWithType { _ty: ty, arg })
    }
}

pub struct FunctionId {
    pub module_id: ModuleId,
    pub function_id: Identifier,
}

fn parse_function_name(function_id: &str) -> CliTypedResult<FunctionId> {
    let ids: Vec<&str> = function_id.split_terminator("::").collect();
    if ids.len() != 3 {
        return Err(CliError::CommandArgumentError(
            "FunctionId is not well formed.  Must be of the form <address>::<module>::<function>"
                .to_string(),
        ));
    }
    let address = load_account_arg(ids.get(0).unwrap())?;
    let module = Identifier::from_str(ids.get(1).unwrap())
        .map_err(|err| CliError::UnableToParse("Module Name", err.to_string()))?;
    let function_id = Identifier::from_str(ids.get(2).unwrap())
        .map_err(|err| CliError::UnableToParse("Function Name", err.to_string()))?;
    let module_id = ModuleId::new(address, module);
    Ok(FunctionId {
        module_id,
        function_id,
    })
}
