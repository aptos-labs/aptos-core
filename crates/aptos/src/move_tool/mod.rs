// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod aptos_debug_natives;

use crate::{
    common::{
        types::{
            load_account_arg, AccountAddressWrapper, CliError, CliTypedResult, EncodingOptions,
            MovePackageDir, ProfileOptions, PromptOptions, TransactionSummary,
            WriteTransactionOptions,
        },
        utils::{check_if_file_exists, submit_transaction},
    },
    CliCommand, CliResult,
};
use aptos_rest_client::aptos_api_types::MoveType;
use aptos_types::transaction::{ModuleBundle, ScriptFunction, TransactionPayload};
use async_trait::async_trait;
use clap::{Parser, Subcommand};
use move_deps::{
    move_cli,
    move_cli::package::cli::UnitTestResult,
    move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{ModuleId, TypeTag},
    },
    move_package::{
        compilation::compiled_package::CompiledPackage,
        source_package::layout::SourcePackageLayout, BuildConfig,
    },
    move_unit_test::UnitTestingConfig,
};
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    fs::create_dir_all,
    io::Write,
    path::{Path, PathBuf},
    str::FromStr,
};

/// CLI tool for performing Move tasks
///
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    Init(InitPackage),
    Publish(PublishPackage),
    Run(RunFunction),
    Test(TestPackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => tool.execute_serialized().await,
            MoveTool::Init(tool) => tool.execute_serialized_success().await,
            MoveTool::Publish(tool) => tool.execute_serialized().await,
            MoveTool::Run(tool) => tool.execute_serialized().await,
            MoveTool::Test(tool) => tool.execute_serialized().await,
        }
    }
}

/// Creates a new Move package at the given location
#[derive(Parser)]
pub struct InitPackage {
    /// Name of the new move package
    #[clap(long)]
    name: String,
    /// Path to create the new move package
    #[clap(long, parse(from_os_str), default_value_os_t = crate::common::utils::current_dir())]
    package_dir: PathBuf,
    /// Named addresses for the move binary
    ///
    /// Example: alice=0x1234, bob=0x5678
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(long, parse(try_from_str = crate::common::utils::parse_map), default_value = "")]
    named_addresses: BTreeMap<String, AccountAddressWrapper>,
    #[clap(flatten)]
    prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<()> for InitPackage {
    fn command_name(&self) -> &'static str {
        "InitPackage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let move_toml = self.package_dir.join(SourcePackageLayout::Manifest.path());
        check_if_file_exists(move_toml.as_path(), self.prompt_options)?;
        create_dir_all(self.package_dir.join(SourcePackageLayout::Sources.path())).map_err(
            |err| {
                CliError::IO(
                    format!(
                        "Failed to create {} move package directories",
                        self.package_dir.display()
                    ),
                    err,
                )
            },
        )?;
        let mut w = std::fs::File::create(move_toml.as_path()).map_err(|err| {
            CliError::UnexpectedError(format!(
                "Failed to create {}: {}",
                self.package_dir.join(Path::new("Move.toml")).display(),
                err
            ))
        })?;

        let addresses: BTreeMap<String, String> = self
            .named_addresses
            .clone()
            .into_iter()
            .map(|(key, value)| (key, value.account_address.to_hex_literal()))
            .collect();

        // TODO: Support Git as default when Github credentials are properly handled from GH CLI
        writeln!(
            &mut w,
            "[package]
name = \"{}\"
version = \"0.0.0\"

[dependencies]
AptosFramework = {{ git = \"https://github.com/aptos-labs/aptos-core.git\", subdir = \"aptos-move/framework/aptos-framework/\", rev = \"main\" }}

[addresses]
{}
",
            self.name,
            toml::to_string(&addresses).unwrap()
        )
        .map_err(|err| {
            CliError::UnexpectedError(format!(
                "Failed to write {:?}: {}",
                self.package_dir.join(Path::new("Move.toml")),
                err
            ))
        })
    }
}

/// Compiles a package and returns the [`ModuleId`]s
#[derive(Parser)]
pub struct CompilePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,
}

#[async_trait]
impl CliCommand<Vec<String>> for CompilePackage {
    fn command_name(&self) -> &'static str {
        "CompilePackage"
    }

    async fn execute(self) -> CliTypedResult<Vec<String>> {
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

    /// A filter string to determine which unit tests to run
    #[clap(long)]
    pub filter: Option<String>,
}

#[async_trait]
impl CliCommand<&'static str> for TestPackage {
    fn command_name(&self) -> &'static str {
        "TestPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let result = move_cli::package::cli::run_move_unit_tests(
            self.move_options.package_dir.as_path(),
            config,
            UnitTestingConfig {
                filter: self.filter,
                ..UnitTestingConfig::default_with_bound(Some(100_000))
            },
            aptos_debug_natives::aptos_debug_natives(),
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

#[async_trait]
impl CliCommand<TransactionSummary> for PublishPackage {
    fn command_name(&self) -> &'static str {
        "PublishPackage"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
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
        .map(TransactionSummary::from)
    }
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

#[async_trait]
impl CliCommand<TransactionSummary> for RunFunction {
    fn command_name(&self) -> &'static str {
        "RunFunction"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
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
        .map(TransactionSummary::from)
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
