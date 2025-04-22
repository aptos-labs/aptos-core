// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account::derive_resource_account::ResourceAccountSeed,
    common::{
        local_simulation,
        transactions::TxnOptions,
        types::{
            load_account_arg, ArgWithTypeJSON, ChunkedPublishOption, CliConfig, CliError,
            CliTypedResult, ConfigSearchMode, EntryFunctionArguments, EntryFunctionArgumentsJSON,
            MoveManifestAccountWrapper, MovePackageOptions, OverrideSizeCheckOption,
            ProfileOptions, PromptOptions, RestOptions, SaveFile, ScriptFunctionArguments,
            TransactionOptions, TransactionSummary, GIT_IGNORE,
        },
        utils::{
            check_if_file_exists, create_dir_if_not_exist, dir_default_to_current,
            profile_or_submit, prompt_yes_with_override, write_to_file,
        },
    },
    governance::CompileScriptFunction,
    move_tool::{
        bytecode::{Decompile, Disassemble},
        coverage::SummaryCoverage,
        fmt::Fmt,
        lint::LintPackage,
        manifest::{Dependency, ManifestNamedAddress, MovePackageManifest, PackageInfo},
    },
    CliCommand, CliResult,
};
use aptos_api_types::AptosErrorCode;
use aptos_crypto::HashValue;
use aptos_framework::{
    chunked_publish::{
        chunk_package_and_create_payloads, large_packages_cleanup_staging_area, PublishType,
        LARGE_PACKAGES_MODULE_ADDRESS,
    },
    docgen::DocgenOptions,
    extended_checks,
    natives::code::UpgradePolicy,
    prover::ProverOptions,
    BuildOptions, BuiltPackage,
};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters};
use aptos_move_debugger::aptos_debugger::AptosDebugger;
use aptos_rest_client::{
    aptos_api_types::{EntryFunctionId, HexEncodedBytes, IdentifierWrapper, MoveModuleId},
    error::RestError,
    AptosBaseUrl, Client,
};
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    object_address::create_object_code_deployment_address,
    on_chain_config::aptos_test_feature_flags_genesis,
    transaction::{Transaction, TransactionArgument, TransactionPayload, TransactionStatus},
};
use aptos_vm::data_cache::AsMoveResolver;
use async_trait::async_trait;
use clap::{Parser, Subcommand, ValueEnum};
use colored::Colorize;
use itertools::Itertools;
use move_cli::{self, base::test::UnitTestResult};
use move_command_line_common::{address::NumericalAddress, env::MOVE_HOME};
use move_core_types::{identifier::Identifier, language_storage::ModuleId, u256::U256};
use move_model::metadata::{CompilerVersion, LanguageVersion};
use move_package::{source_package::layout::SourcePackageLayout, BuildConfig, CompilerConfig};
use move_unit_test::UnitTestingConfig;
pub use package_hooks::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
    str::FromStr,
};
pub use stored_package::*;
use tokio::task;
use url::Url;

pub mod aptos_debug_natives;
mod bytecode;
pub mod coverage;
mod fmt;
mod lint;
mod manifest;
pub mod package_hooks;
mod show;
pub mod stored_package;

const HELLO_BLOCKCHAIN_EXAMPLE: &str = include_str!(
    "../../../../aptos-move/move-examples/hello_blockchain/sources/hello_blockchain.move"
);

/// Tool for Move smart contract related operations
///
/// This tool lets you compile, test, and publish Move code, in addition
/// to run any other tools that help run, verify, or provide information
/// about this code.
#[derive(Subcommand)]
pub enum MoveTool {
    BuildPublishPayload(BuildPublishPayload),
    Clean(CleanPackage),
    ClearStagingArea(ClearStagingArea),
    #[clap(alias = "build")]
    Compile(CompilePackage),
    #[clap(alias = "build-script")]
    CompileScript(CompileScript),
    #[clap(subcommand)]
    Coverage(coverage::CoveragePackage),
    CreateObjectAndPublishPackage(CreateObjectAndPublishPackage),
    UpgradeObjectPackage(UpgradeObjectPackage),
    DeployObject(DeployObjectCode),
    UpgradeObject(UpgradeCodeObject),
    CreateResourceAccountAndPublishPackage(CreateResourceAccountAndPublishPackage),
    Disassemble(Disassemble),
    Decompile(Decompile),
    #[clap(alias = "doc")]
    Document(DocumentPackage),
    Download(DownloadPackage),
    Init(InitPackage),
    Lint(LintPackage),
    List(ListPackage),
    Prove(ProvePackage),
    #[clap(alias = "deploy")]
    Publish(PublishPackage),
    Run(RunFunction),
    RunScript(RunScript),
    Simulate(Simulate),
    #[clap(subcommand, hide = true)]
    Show(show::ShowTool),
    Test(TestPackage),
    VerifyPackage(VerifyPackage),
    View(ViewFunction),
    Replay(Replay),
    Fmt(Fmt),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::BuildPublishPayload(tool) => tool.execute_serialized().await,
            MoveTool::Clean(tool) => tool.execute_serialized().await,
            MoveTool::ClearStagingArea(tool) => tool.execute_serialized().await,
            MoveTool::Compile(tool) => tool.execute_serialized().await,
            MoveTool::CompileScript(tool) => tool.execute_serialized().await,
            MoveTool::Coverage(tool) => tool.execute().await,
            MoveTool::CreateObjectAndPublishPackage(tool) => {
                tool.execute_serialized_success().await
            },
            MoveTool::UpgradeObjectPackage(tool) => tool.execute_serialized_success().await,
            MoveTool::DeployObject(tool) => tool.execute_serialized_success().await,
            MoveTool::UpgradeObject(tool) => tool.execute_serialized_success().await,
            MoveTool::CreateResourceAccountAndPublishPackage(tool) => {
                tool.execute_serialized_success().await
            },
            MoveTool::Disassemble(tool) => tool.execute_serialized().await,
            MoveTool::Decompile(tool) => tool.execute_serialized().await,
            MoveTool::Document(tool) => tool.execute_serialized().await,
            MoveTool::Download(tool) => tool.execute_serialized().await,
            MoveTool::Init(tool) => tool.execute_serialized_success().await,
            MoveTool::List(tool) => tool.execute_serialized().await,
            MoveTool::Prove(tool) => tool.execute_serialized().await,
            MoveTool::Publish(tool) => tool.execute_serialized().await,
            MoveTool::Run(tool) => tool.execute_serialized().await,
            MoveTool::RunScript(tool) => tool.execute_serialized().await,
            MoveTool::Simulate(tool) => tool.execute_serialized().await,
            MoveTool::Show(tool) => tool.execute_serialized().await,
            MoveTool::Test(tool) => tool.execute_serialized().await,
            MoveTool::VerifyPackage(tool) => tool.execute_serialized().await,
            MoveTool::View(tool) => tool.execute_serialized().await,
            MoveTool::Replay(tool) => tool.execute_serialized().await,
            MoveTool::Fmt(tool) => tool.execute_serialized().await,
            MoveTool::Lint(tool) => tool.execute_serialized().await,
        }
    }
}

#[derive(Default, Parser)]
pub struct FrameworkPackageArgs {
    /// Git revision or branch for the Aptos framework
    ///
    /// This is mutually exclusive with `--framework-local-dir`
    #[clap(long, group = "framework_package_args")]
    pub(crate) framework_git_rev: Option<String>,

    /// Local framework directory for the Aptos framework
    ///
    /// This is mutually exclusive with `--framework-git-rev`
    #[clap(long, value_parser, group = "framework_package_args")]
    pub(crate) framework_local_dir: Option<PathBuf>,

    /// Skip pulling the latest git dependencies
    ///
    /// If you don't have a network connection, the compiler may fail due
    /// to no ability to pull git dependencies.  This will allow overriding
    /// this for local development.
    #[clap(long)]
    pub(crate) skip_fetch_latest_git_deps: bool,
}

impl FrameworkPackageArgs {
    pub fn init_move_dir(
        &self,
        package_dir: &Path,
        name: &str,
        addresses: BTreeMap<String, ManifestNamedAddress>,
        prompt_options: PromptOptions,
    ) -> CliTypedResult<()> {
        const APTOS_FRAMEWORK: &str = "AptosFramework";
        const APTOS_GIT_PATH: &str = "https://github.com/aptos-labs/aptos-framework.git";
        const SUBDIR_PATH: &str = "aptos-framework";
        const DEFAULT_BRANCH: &str = "mainnet";

        let move_toml = package_dir.join(SourcePackageLayout::Manifest.path());
        check_if_file_exists(move_toml.as_path(), prompt_options)?;
        create_dir_if_not_exist(
            package_dir
                .join(SourcePackageLayout::Sources.path())
                .as_path(),
        )?;
        create_dir_if_not_exist(
            package_dir
                .join(SourcePackageLayout::Tests.path())
                .as_path(),
        )?;
        create_dir_if_not_exist(
            package_dir
                .join(SourcePackageLayout::Scripts.path())
                .as_path(),
        )?;

        // Add the framework dependency if it's provided
        let mut dependencies = BTreeMap::new();
        if let Some(ref path) = self.framework_local_dir {
            dependencies.insert(APTOS_FRAMEWORK.to_string(), Dependency {
                local: Some(path.display().to_string()),
                git: None,
                rev: None,
                subdir: None,
                aptos: None,
                address: None,
            });
        } else {
            let git_rev = self.framework_git_rev.as_deref().unwrap_or(DEFAULT_BRANCH);
            dependencies.insert(APTOS_FRAMEWORK.to_string(), Dependency {
                local: None,
                git: Some(APTOS_GIT_PATH.to_string()),
                rev: Some(git_rev.to_string()),
                subdir: Some(SUBDIR_PATH.to_string()),
                aptos: None,
                address: None,
            });
        }

        let manifest = MovePackageManifest {
            package: PackageInfo {
                name: name.to_string(),
                version: "1.0.0".to_string(),
                license: None,
                authors: vec![],
            },
            addresses,
            dependencies,
            dev_addresses: Default::default(),
            dev_dependencies: Default::default(),
        };

        write_to_file(
            move_toml.as_path(),
            SourcePackageLayout::Manifest.location_str(),
            toml::to_string_pretty(&manifest)
                .map_err(|err| CliError::UnexpectedError(err.to_string()))?
                .as_bytes(),
        )?;

        // Write a .gitignore
        let gitignore = package_dir.join(GIT_IGNORE);
        check_if_file_exists(gitignore.as_path(), prompt_options)?;
        write_to_file(
            gitignore.as_path(),
            GIT_IGNORE,
            ".aptos/\nbuild/".as_bytes(),
        )
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Template {
    HelloBlockchain,
}

/// Creates a new Move package at the given location
///
/// This will create a directory for a Move package and a corresponding
/// `Move.toml` file.
#[derive(Parser)]
pub struct InitPackage {
    /// Name of the new Move package
    #[clap(long)]
    pub(crate) name: String,

    /// Directory to create the new Move package
    #[clap(long, value_parser)]
    pub(crate) package_dir: Option<PathBuf>,

    /// Named addresses for the move binary
    ///
    /// Allows for an address to be put into the Move.toml, or a placeholder `_`
    ///
    /// Example: alice=0x1234,bob=0x5678,greg=_
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(
        long,
        value_parser = crate::common::utils::parse_map::<String, MoveManifestAccountWrapper>,
        default_value = ""
    )]
    pub(crate) named_addresses: BTreeMap<String, MoveManifestAccountWrapper>,

    /// Template name for initialization
    #[clap(long)]
    pub(crate) template: Option<Template>,

    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,

    #[clap(flatten)]
    pub(crate) framework_package_args: FrameworkPackageArgs,
}

#[async_trait]
impl CliCommand<()> for InitPackage {
    fn command_name(&self) -> &'static str {
        "InitPackage"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let package_dir = dir_default_to_current(self.package_dir.clone())?;
        let mut addresses: BTreeMap<String, ManifestNamedAddress> = self
            .named_addresses
            .into_iter()
            .map(|(key, value)| (key, value.account_address.into()))
            .collect();

        // Add in any template associated
        match self.template {
            None => {
                // Initialize move directory
                // TODO: Communicate this breaking change before filling in the template as default
                let package_dir_path = package_dir.as_path();
                self.framework_package_args.init_move_dir(
                    package_dir_path,
                    &self.name,
                    addresses,
                    self.prompt_options,
                )
            },
            Some(Template::HelloBlockchain) => {
                // Setup the Hello blockchain template
                // Note: We have to override the addresses
                addresses.insert("hello_blockchain".to_string(), None.into());

                // Initialize move directory
                let package_dir_path = package_dir.as_path();
                self.framework_package_args.init_move_dir(
                    package_dir_path,
                    "HelloBlockchainExample",
                    addresses,
                    self.prompt_options,
                )?;

                write_to_file(
                    package_dir_path
                        .join("sources/hello_blockchain.move")
                        .as_path(),
                    "hello_blockchain.move",
                    HELLO_BLOCKCHAIN_EXAMPLE.as_bytes(),
                )
            },
        }
    }
}

/// Compiles a package and returns the associated ModuleIds
#[derive(Parser)]
pub struct CompilePackage {
    /// Save the package metadata in the package's build directory
    ///
    /// If set, package metadata should be generated and stored in the package's build directory.
    /// This metadata can be used to construct a transaction to publish a package.
    #[clap(long)]
    pub save_metadata: bool,

    /// Fetch dependencies of a package from the network, skipping the actual compilation
    #[clap(long)]
    pub fetch_deps_only: bool,

    #[clap(flatten)]
    pub included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub move_options: MovePackageOptions,
}

#[async_trait]
impl CliCommand<Vec<String>> for CompilePackage {
    fn command_name(&self) -> &'static str {
        "CompilePackage"
    }

    async fn execute(self) -> CliTypedResult<Vec<String>> {
        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            ..self
                .included_artifacts_args
                .included_artifacts
                .build_options(&self.move_options)?
        };
        let package_path = self.move_options.get_package_path()?;
        if self.fetch_deps_only {
            let config = BuiltPackage::create_build_config(&build_options)?;
            BuiltPackage::prepare_resolution_graph(package_path, config)?;
            return Ok(vec![]);
        }
        let pack = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;
        if self.save_metadata {
            pack.extract_metadata_and_save()?;
        }
        let ids = pack
            .modules()
            .map(|m| m.self_id().to_string())
            .collect::<Vec<_>>();
        // TODO: Also say how many scripts are compiled
        Ok(ids)
    }
}

/// Compiles a Move script into bytecode
///
/// Compiles a script into bytecode and provides a hash of the bytecode.
/// This can then be run with `aptos move run-script`
#[derive(Parser)]
pub struct CompileScript {
    #[clap(long, value_parser)]
    pub output_file: Option<PathBuf>,
    #[clap(flatten)]
    pub move_options: MovePackageOptions,
}

#[async_trait]
impl CliCommand<CompileScriptOutput> for CompileScript {
    fn command_name(&self) -> &'static str {
        "CompileScript"
    }

    async fn execute(self) -> CliTypedResult<CompileScriptOutput> {
        let (bytecode, script_hash) = self.compile_script().await?;
        let script_location = self.output_file.unwrap_or_else(|| {
            self.move_options
                .get_package_path()
                .unwrap()
                .join("script.mv")
        });
        write_to_file(script_location.as_path(), "Script", bytecode.as_slice())?;
        Ok(CompileScriptOutput {
            script_location,
            script_hash,
        })
    }
}

impl CompileScript {
    async fn compile_script(&self) -> CliTypedResult<(Vec<u8>, HashValue)> {
        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            ..IncludedArtifacts::None.build_options(&self.move_options)?
        };
        let package_dir = self.move_options.get_package_path()?;
        let pack = BuiltPackage::build(package_dir, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;

        let scripts_count = pack.script_count();
        if scripts_count != 1 {
            return Err(CliError::UnexpectedError(format!(
                "Only one script can be prepared a time. Make sure one and only one script file \
                is included in the Move package. Found {} scripts.",
                scripts_count
            )));
        }

        let bytecode = pack.extract_script_code().pop().unwrap();
        let script_hash = HashValue::sha3_256_of(bytecode.as_slice());
        Ok((bytecode, script_hash))
    }
}

#[derive(Debug, Serialize)]
pub struct CompileScriptOutput {
    pub script_location: PathBuf,
    pub script_hash: HashValue,
}

/// Runs Move unit tests for a package
///
/// This will run Move unit tests against a package with debug mode
/// turned on.  Note, that move code warnings currently block tests from running.
#[derive(Parser)]
pub struct TestPackage {
    /// A filter string to determine which unit tests to run
    #[clap(long, short)]
    pub filter: Option<String>,

    /// A boolean value to skip warnings.
    #[clap(long)]
    pub ignore_compile_warnings: bool,

    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,

    /// The maximum number of instructions that can be executed by a test
    ///
    /// If set, the number of instructions executed by one test will be bounded
    // TODO: Remove short, it's against the style guidelines, and update the name here
    #[clap(
        name = "instructions",
        default_value_t = 100000,
        short = 'i',
        long = "instructions"
    )]
    pub instruction_execution_bound: u64,

    /// Collect coverage information for later use with the various `aptos move coverage` subcommands
    #[clap(long = "coverage")]
    pub compute_coverage: bool,

    /// Dump storage state on failure.
    #[clap(long = "dump")]
    pub dump_state: bool,
}

pub(crate) fn fix_bytecode_version(
    bytecode_version_in: Option<u32>,
    language_version: Option<LanguageVersion>,
) -> Option<u32> {
    if bytecode_version_in.is_none() {
        if let Some(language_version) = language_version {
            Some(language_version.infer_bytecode_version(bytecode_version_in))
        } else {
            bytecode_version_in
        }
    } else {
        bytecode_version_in
    }
}

#[async_trait]
impl CliCommand<&'static str> for TestPackage {
    fn command_name(&self) -> &'static str {
        "TestPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let known_attributes = extended_checks::get_all_attribute_names();
        let mut config = BuildConfig {
            dev_mode: self.move_options.dev,
            additional_named_addresses: self.move_options.named_addresses(),
            test_mode: true,
            full_model_generation: !self.move_options.skip_checks_on_test_code,
            install_dir: self.move_options.output_dir.clone(),
            skip_fetch_latest_git_deps: self.move_options.skip_fetch_latest_git_deps,
            compiler_config: CompilerConfig {
                known_attributes: known_attributes.clone(),
                skip_attribute_checks: self.move_options.skip_attribute_checks,
                bytecode_version: fix_bytecode_version(
                    self.move_options.bytecode_version,
                    self.move_options.language_version,
                ),
                compiler_version: self
                    .move_options
                    .compiler_version
                    .or_else(|| Some(CompilerVersion::latest_stable())),
                language_version: self
                    .move_options
                    .language_version
                    .or_else(|| Some(LanguageVersion::latest_stable())),
                experiments: self.move_options.compute_experiments(),
            },
            ..Default::default()
        };

        let path = self.move_options.get_package_path()?;
        let result = move_cli::base::test::run_move_unit_tests(
            path.as_path(),
            config.clone(),
            UnitTestingConfig {
                filter: self.filter.clone(),
                report_stacktrace_on_abort: true,
                report_storage_on_error: self.dump_state,
                ignore_compile_warnings: self.ignore_compile_warnings,
                named_address_values: self
                    .move_options
                    .named_addresses
                    .iter()
                    .map(|(name, addr_wrap)| {
                        (
                            name.clone(),
                            NumericalAddress::from_account_address(addr_wrap.account_address),
                        )
                    })
                    .collect(),
                ..UnitTestingConfig::default()
            },
            // TODO(Gas): we may want to switch to non-zero costs in the future
            aptos_debug_natives::aptos_debug_natives(
                NativeGasParameters::zeros(),
                MiscGasParameters::zeros(),
            ),
            aptos_test_feature_flags_genesis(),
            None,
            None,
            self.compute_coverage,
            &mut std::io::stdout(),
        )
        .map_err(|err| CliError::UnexpectedError(format!("Failed to run tests: {:#}", err)))?;

        // Print coverage summary if --coverage is set
        if self.compute_coverage {
            // TODO: config seems to be dead here.
            config.test_mode = false;
            let summary = SummaryCoverage {
                summarize_functions: false,
                output_csv: false,
                filter: self.filter,
                move_options: self.move_options,
            };
            summary.coverage()?;

            println!("Please use `aptos move coverage -h` for more detailed source or bytecode test coverage of this package");
        }

        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Err(CliError::MoveTestError),
        }
    }
}

/// Proves a Move package
///
/// This is a tool for formal verification of a Move package using
/// the Move prover
#[derive(Parser)]
pub struct ProvePackage {
    #[clap(flatten)]
    move_options: MovePackageOptions,

    #[clap(flatten)]
    prover_options: ProverOptions,
}

#[async_trait]
impl CliCommand<&'static str> for ProvePackage {
    fn command_name(&self) -> &'static str {
        "ProvePackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let ProvePackage {
            move_options,
            prover_options,
        } = self;

        let compiler_version = move_options
            .compiler_version
            .or_else(|| Some(CompilerVersion::latest_stable()));
        let language_version = move_options
            .language_version
            .or_else(|| Some(LanguageVersion::latest_stable()));

        let result = task::spawn_blocking(move || {
            prover_options.prove(
                move_options.dev,
                move_options.get_package_path()?.as_path(),
                move_options.named_addresses(),
                fix_bytecode_version(move_options.bytecode_version, language_version),
                compiler_version,
                language_version,
                move_options.skip_attribute_checks,
                extended_checks::get_all_attribute_names(),
                &[],
            )
        })
        .await
        .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
        match result {
            Ok(_) => Ok("Success"),
            Err(e) => Err(CliError::MoveProverError(format!("{:#}", e))),
        }
    }
}

/// Documents a Move package
///
/// This converts the content of the package into markdown for documentation.
#[derive(Parser)]
pub struct DocumentPackage {
    #[clap(flatten)]
    move_options: MovePackageOptions,

    #[clap(flatten)]
    docgen_options: DocgenOptions,
}

#[async_trait]
impl CliCommand<&'static str> for DocumentPackage {
    fn command_name(&self) -> &'static str {
        "DocumentPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let DocumentPackage {
            move_options,
            docgen_options,
        } = self;
        let build_options = BuildOptions {
            dev: move_options.dev,
            with_error_map: false,
            with_docs: true,
            named_addresses: move_options.named_addresses(),
            override_std: move_options.override_std.clone(),
            docgen_options: Some(docgen_options),
            skip_fetch_latest_git_deps: move_options.skip_fetch_latest_git_deps,
            bytecode_version: fix_bytecode_version(
                move_options.bytecode_version,
                move_options.language_version,
            ),
            compiler_version: move_options
                .compiler_version
                .or_else(|| Some(CompilerVersion::latest_stable())),
            language_version: move_options
                .language_version
                .or_else(|| Some(LanguageVersion::latest_stable())),
            skip_attribute_checks: move_options.skip_attribute_checks,
            check_test_code: !move_options.skip_checks_on_test_code,
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..BuildOptions::default()
        };
        BuiltPackage::build(move_options.get_package_path()?, build_options)?;
        Ok("succeeded")
    }
}

#[derive(Parser)]
pub struct IncludedArtifactsArgs {
    /// Artifacts to be generated when building the package
    ///
    /// Which artifacts to include in the package. This can be one of `none`, `sparse`, and
    /// `all`. `none` is the most compact form and does not allow to reconstruct a source
    /// package from chain; `sparse` is the minimal set of artifacts needed to reconstruct
    /// a source package; `all` includes all available artifacts. The choice of included
    /// artifacts heavily influences the size and therefore gas cost of publishing: `none`
    /// is the size of bytecode alone; `sparse` is roughly 2 times as much; and `all` 3-4
    /// as much.
    #[clap(long, default_value_t = IncludedArtifacts::Sparse)]
    pub included_artifacts: IncludedArtifacts,
}

/// Publishes the modules in a Move package to the Aptos blockchain
#[derive(Parser)]
pub struct PublishPackage {
    #[clap(flatten)]
    pub(crate) override_size_check_option: OverrideSizeCheckOption,
    #[clap(flatten)]
    pub(crate) chunked_publish_option: ChunkedPublishOption,
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

pub(crate) struct PackagePublicationData {
    metadata_serialized: Vec<u8>,
    compiled_units: Vec<Vec<u8>>,
    payload: TransactionPayload,
}

pub(crate) struct ChunkedPublishPayloads {
    payloads: Vec<TransactionPayload>,
}

/// Build a publication transaction payload and store it in a JSON output file.
#[derive(Parser)]
pub struct BuildPublishPayload {
    #[clap(flatten)]
    publish_package: PublishPackage,
    /// JSON output file to write publication transaction to
    #[clap(long, value_parser)]
    pub(crate) json_output_file: PathBuf,
}

impl TryInto<PackagePublicationData> for &PublishPackage {
    type Error = CliError;

    fn try_into(self) -> Result<PackagePublicationData, Self::Error> {
        let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;

        let package_publication_data =
            create_package_publication_data(package, PublishType::AccountDeploy, None)?;

        let size = bcs::serialized_size(&package_publication_data.payload)?;
        println!("package size {} bytes", size);
        if !self.override_size_check_option.override_size_check && size > MAX_PUBLISH_PACKAGE_SIZE {
            return Err(CliError::PackageSizeExceeded(
                size,
                MAX_PUBLISH_PACKAGE_SIZE,
            ));
        }

        Ok(package_publication_data)
    }
}

#[async_trait]
pub trait AsyncTryInto<T> {
    type Error;

    async fn async_try_into(self) -> Result<T, Self::Error>;
}

#[async_trait]
impl AsyncTryInto<ChunkedPublishPayloads> for &PublishPackage {
    type Error = CliError;

    async fn async_try_into(self) -> Result<ChunkedPublishPayloads, Self::Error> {
        let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;

        let chunked_publish_payloads = create_chunked_publish_payloads(
            package,
            PublishType::AccountDeploy,
            None,
            self.chunked_publish_option.large_packages_module_address,
            self.chunked_publish_option.chunk_size,
        )?;

        let size = &chunked_publish_payloads
            .payloads
            .iter()
            .map(bcs::serialized_size)
            .sum::<Result<usize, _>>()?;
        println!("package size {} bytes", size);

        Ok(chunked_publish_payloads)
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum IncludedArtifacts {
    None,
    Sparse,
    All,
}

impl Display for IncludedArtifacts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use IncludedArtifacts::*;
        match self {
            None => f.write_str("none"),
            Sparse => f.write_str("sparse"),
            All => f.write_str("all"),
        }
    }
}

impl FromStr for IncludedArtifacts {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use IncludedArtifacts::*;
        match s {
            "none" => Ok(None),
            "sparse" => Ok(Sparse),
            "all" => Ok(All),
            _ => Err("unknown variant"),
        }
    }
}

impl IncludedArtifacts {
    pub(crate) fn build_options(
        self,
        move_options: &MovePackageOptions,
    ) -> CliTypedResult<BuildOptions> {
        self.build_options_with_experiments(move_options, vec![], false)
    }

    pub(crate) fn build_options_with_experiments(
        self,
        move_options: &MovePackageOptions,
        mut more_experiments: Vec<String>,
        _skip_codegen: bool, // we currently cannot do this, so ignore it.
    ) -> CliTypedResult<BuildOptions> {
        let dev = move_options.dev;
        let skip_fetch_latest_git_deps = move_options.skip_fetch_latest_git_deps;
        let named_addresses = move_options.named_addresses();
        let override_std = move_options.override_std.clone();
        let bytecode_version =
            fix_bytecode_version(move_options.bytecode_version, move_options.language_version);
        let compiler_version = move_options
            .compiler_version
            .or_else(|| Some(CompilerVersion::latest_stable()));
        let language_version = move_options
            .language_version
            .or_else(|| Some(LanguageVersion::latest_stable()));
        let skip_attribute_checks = move_options.skip_attribute_checks;
        let check_test_code = !move_options.skip_checks_on_test_code;
        let mut experiments = move_options.compute_experiments();
        experiments.append(&mut more_experiments);

        let base_options = BuildOptions {
            dev,
            // Always enable error map bytecode injection
            with_error_map: true,
            named_addresses,
            override_std,
            skip_fetch_latest_git_deps,
            bytecode_version,
            compiler_version,
            language_version,
            skip_attribute_checks,
            check_test_code,
            experiments,
            known_attributes: extended_checks::get_all_attribute_names().clone(),
            ..BuildOptions::default()
        };
        use IncludedArtifacts::*;
        Ok(match self {
            None => BuildOptions {
                with_srcs: false,
                with_abis: false,
                with_source_maps: false,
                ..base_options
            },
            Sparse => BuildOptions {
                with_srcs: true,
                with_abis: false,
                with_source_maps: false,
                ..base_options
            },
            All => BuildOptions {
                with_srcs: true,
                with_abis: true,
                with_source_maps: true,
                ..base_options
            },
        })
    }
}

pub const MAX_PUBLISH_PACKAGE_SIZE: usize = 60_000;

// Get publication data for standard publish mode, which submits a single transaction for publishing.
fn create_package_publication_data(
    package: BuiltPackage,
    publish_type: PublishType,
    object_address: Option<AccountAddress>,
) -> CliTypedResult<PackagePublicationData> {
    let compiled_units = package.extract_code();
    let metadata = package.extract_metadata()?;
    let metadata_serialized = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");

    let payload = match publish_type {
        PublishType::AccountDeploy => {
            aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
                metadata_serialized.clone(),
                compiled_units.clone(),
            )
        },
        PublishType::ObjectDeploy => {
            aptos_cached_packages::aptos_stdlib::object_code_deployment_publish(
                metadata_serialized.clone(),
                compiled_units.clone(),
            )
        },
        PublishType::ObjectUpgrade => {
            aptos_cached_packages::aptos_stdlib::object_code_deployment_upgrade(
                metadata_serialized.clone(),
                compiled_units.clone(),
                object_address.expect("Object address must be provided for upgrading object code."),
            )
        },
    };

    Ok(PackagePublicationData {
        metadata_serialized,
        compiled_units,
        payload,
    })
}

// Get publication data for chunked publish mode, which submits multiple transactions for publishing.
fn create_chunked_publish_payloads(
    package: BuiltPackage,
    publish_type: PublishType,
    object_address: Option<AccountAddress>,
    large_packages_module_address: AccountAddress,
    chunk_size: usize,
) -> CliTypedResult<ChunkedPublishPayloads> {
    let compiled_units = package.extract_code();
    let metadata = package.extract_metadata()?;
    let metadata_serialized = bcs::to_bytes(&metadata).expect("PackageMetadata has BCS");

    let maybe_object_address = if let PublishType::ObjectUpgrade = publish_type {
        object_address
    } else {
        None
    };

    let payloads = chunk_package_and_create_payloads(
        metadata_serialized,
        compiled_units,
        publish_type,
        maybe_object_address,
        large_packages_module_address,
        chunk_size,
    );

    Ok(ChunkedPublishPayloads { payloads })
}

#[async_trait]
impl CliCommand<TransactionSummary> for PublishPackage {
    fn command_name(&self) -> &'static str {
        "PublishPackage"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        if self.chunked_publish_option.chunked_publish {
            let chunked_package_payloads: ChunkedPublishPayloads = (&self).async_try_into().await?;

            let message = format!("Publishing package in chunked mode will submit {} transactions for staging and publishing code.\n", &chunked_package_payloads.payloads.len());
            println!("{}", message.bold());
            submit_chunked_publish_transactions(
                chunked_package_payloads.payloads,
                &self.txn_options,
                self.chunked_publish_option.large_packages_module_address,
            )
            .await
        } else {
            let package_publication_data: PackagePublicationData = (&self).try_into()?;
            profile_or_submit(package_publication_data.payload, &self.txn_options).await
        }
    }
}

#[async_trait]
impl CliCommand<String> for BuildPublishPayload {
    fn command_name(&self) -> &'static str {
        "BuildPublishPayload"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let package_publication_data: PackagePublicationData =
            (&self.publish_package).try_into()?;
        // Extract entry function data from publication payload.
        let entry_function = package_publication_data.payload.into_entry_function();
        let entry_function_id = EntryFunctionId {
            module: MoveModuleId::from(entry_function.module().clone()),
            name: IdentifierWrapper::from(entry_function.function()),
        };
        let package_metadata_hex =
            HexEncodedBytes(package_publication_data.metadata_serialized).to_string();
        let package_code_hex_vec: Vec<String> = package_publication_data
            .compiled_units
            .into_iter()
            .map(|element| HexEncodedBytes(element).to_string())
            .collect();
        // Construct entry function JSON file representation from entry function data.
        let json = EntryFunctionArgumentsJSON {
            function_id: entry_function_id.to_string(),
            type_args: vec![],
            args: vec![
                ArgWithTypeJSON {
                    arg_type: "hex".to_string(),
                    value: serde_json::Value::String(package_metadata_hex),
                },
                ArgWithTypeJSON {
                    arg_type: "hex".to_string(),
                    value: json!(package_code_hex_vec),
                },
            ],
        };
        // Create save file options for checking and saving file to disk.
        let save_file = SaveFile {
            output_file: self.json_output_file,
            prompt_options: self.publish_package.txn_options.prompt_options,
        };
        save_file.check_file()?;
        save_file.save_to_file(
            "Publication entry function JSON file",
            serde_json::to_string_pretty(&json)
                .map_err(|err| CliError::UnexpectedError(format!("{}", err)))?
                .as_bytes(),
        )?;
        Ok(format!(
            "Publication payload entry function JSON file saved to {}",
            save_file.output_file.display()
        ))
    }
}

/// Publishes the modules in a Move package to the Aptos blockchain, under an object (legacy version of `deploy-object`)
#[derive(Parser)]
pub struct CreateObjectAndPublishPackage {
    /// The named address for compiling and using in the contract
    ///
    /// This will take the derived account address for the object and put it in this location
    #[clap(long)]
    pub(crate) address_name: String,
    #[clap(flatten)]
    pub(crate) override_size_check_option: OverrideSizeCheckOption,
    #[clap(flatten)]
    pub(crate) chunked_publish_option: ChunkedPublishOption,
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateObjectAndPublishPackage {
    fn command_name(&self) -> &'static str {
        "CreateObjectAndPublishPackage"
    }

    // TODO[Ordereless]: Update this code to support stateless accounts that don't have a sequence number
    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let sender_address = self.txn_options.get_public_key_and_address()?.1;

        let sequence_number = if self.chunked_publish_option.chunked_publish {
            // Perform a preliminary build to determine the number of transactions needed for chunked publish mode.
            // This involves building the package with mock account address `0xcafe` to calculate the transaction count.
            let mock_object_address = AccountAddress::from_hex_literal("0xcafe").unwrap();
            self.move_options
                .add_named_address(self.address_name.clone(), mock_object_address.to_string());
            let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;
            let mock_payloads = create_chunked_publish_payloads(
                package,
                PublishType::AccountDeploy,
                None,
                self.chunked_publish_option.large_packages_module_address,
                self.chunked_publish_option.chunk_size,
            )?
            .payloads;
            let staging_tx_count = (mock_payloads.len() - 1) as u64;
            self.txn_options.sequence_number(sender_address).await? + staging_tx_count + 1
        } else {
            self.txn_options.sequence_number(sender_address).await? + 1
        };

        let object_address = create_object_code_deployment_address(sender_address, sequence_number);

        self.move_options
            .add_named_address(self.address_name, object_address.to_string());

        let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;
        let message = format!(
            "Do you want to publish this package at object address {}",
            object_address
        );
        prompt_yes_with_override(&message, self.txn_options.prompt_options)?;

        let result = if self.chunked_publish_option.chunked_publish {
            let payloads = create_chunked_publish_payloads(
                package,
                PublishType::ObjectDeploy,
                None,
                self.chunked_publish_option.large_packages_module_address,
                self.chunked_publish_option.chunk_size,
            )?
            .payloads;

            let size = &payloads
                .iter()
                .map(bcs::serialized_size)
                .sum::<Result<usize, _>>()?;
            println!("package size {} bytes", size);
            let message = format!("Publishing package in chunked mode will submit {} transactions for staging and publishing code.\n", &payloads.len());
            println!("{}", message.bold());

            submit_chunked_publish_transactions(
                payloads,
                &self.txn_options,
                self.chunked_publish_option.large_packages_module_address,
            )
            .await
        } else {
            let payload = create_package_publication_data(
                package,
                PublishType::ObjectDeploy,
                Some(object_address),
            )?
            .payload;
            let size = bcs::serialized_size(&payload)?;
            println!("package size {} bytes", size);

            if !self.override_size_check_option.override_size_check
                && size > MAX_PUBLISH_PACKAGE_SIZE
            {
                return Err(CliError::PackageSizeExceeded(
                    size,
                    MAX_PUBLISH_PACKAGE_SIZE,
                ));
            }
            self.txn_options
                .submit_transaction(payload)
                .await
                .map(TransactionSummary::from)
        };

        if result.is_ok() {
            println!(
                "Code was successfully deployed to object address {}",
                object_address
            );
        }
        result
    }
}

/// Upgrades the modules in a Move package deployed under an object (legacy version of `upgrade-object`)
#[derive(Parser)]
pub struct UpgradeObjectPackage {
    /// Address of the object the package was deployed to
    ///
    /// This must be an already deployed object containing the package
    /// if the package is not already created, it will fail.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) object_address: AccountAddress,
    #[clap(flatten)]
    pub(crate) override_size_check_option: OverrideSizeCheckOption,
    #[clap(flatten)]
    pub(crate) chunked_publish_option: ChunkedPublishOption,
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for UpgradeObjectPackage {
    fn command_name(&self) -> &'static str {
        "UpgradeObjectPackage"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let built_package =
            build_package_options(&self.move_options, &self.included_artifacts_args)?;
        let url = self
            .txn_options
            .rest_options
            .url(&self.txn_options.profile_options)?;

        // Get the `PackageRegistry` at the given object address.
        let registry = CachedPackageRegistry::create(url, self.object_address, false).await?;
        let package = registry
            .get_package(built_package.name())
            .await
            .map_err(|s| CliError::CommandArgumentError(s.to_string()))?;

        if package.upgrade_policy() == UpgradePolicy::immutable() {
            return Err(CliError::CommandArgumentError(
                "A package with upgrade policy `immutable` cannot be upgraded".to_owned(),
            ));
        }

        let message = format!(
            "Do you want to upgrade the package '{}' at object address {}",
            package.name(),
            self.object_address
        );
        prompt_yes_with_override(&message, self.txn_options.prompt_options)?;

        let result = if self.chunked_publish_option.chunked_publish {
            let payloads = create_chunked_publish_payloads(
                built_package,
                PublishType::ObjectUpgrade,
                Some(self.object_address),
                self.chunked_publish_option.large_packages_module_address,
                self.chunked_publish_option.chunk_size,
            )?
            .payloads;

            let size = &payloads
                .iter()
                .map(bcs::serialized_size)
                .sum::<Result<usize, _>>()?;
            println!("package size {} bytes", size);
            let message = format!("Upgrading package in chunked mode will submit {} transactions for staging and upgrading code.\n", &payloads.len());
            println!("{}", message.bold());
            submit_chunked_publish_transactions(
                payloads,
                &self.txn_options,
                self.chunked_publish_option.large_packages_module_address,
            )
            .await
        } else {
            let payload = create_package_publication_data(
                built_package,
                PublishType::ObjectUpgrade,
                Some(self.object_address),
            )?
            .payload;

            let size = bcs::serialized_size(&payload)?;
            println!("package size {} bytes", size);

            if !self.override_size_check_option.override_size_check
                && size > MAX_PUBLISH_PACKAGE_SIZE
            {
                return Err(CliError::PackageSizeExceeded(
                    size,
                    MAX_PUBLISH_PACKAGE_SIZE,
                ));
            }
            self.txn_options
                .submit_transaction(payload)
                .await
                .map(TransactionSummary::from)
        };

        if result.is_ok() {
            println!(
                "Code was successfully upgraded at object address {}",
                self.object_address
            );
        }
        result
    }
}

/// Publishes the modules in a Move package to the Aptos blockchain, under an object.
#[derive(Parser)]
pub struct DeployObjectCode {
    /// The named address for compiling and using in the contract
    ///
    /// This will take the derived account address for the object and put it in this location
    #[clap(long)]
    pub(crate) address_name: String,
    #[clap(flatten)]
    pub(crate) override_size_check_option: OverrideSizeCheckOption,
    #[clap(flatten)]
    pub(crate) chunked_publish_option: ChunkedPublishOption,
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for DeployObjectCode {
    fn command_name(&self) -> &'static str {
        "DeployObject"
    }

    // TODO[Ordereless]: Update this code to support stateless accounts that don't have a sequence number
    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        let sender_address = self.txn_options.get_public_key_and_address()?.1;
        let sequence_number = if self.chunked_publish_option.chunked_publish {
            // Perform a preliminary build to determine the number of transactions needed for chunked publish mode.
            // This involves building the package with mock account address `0xcafe` to calculate the transaction count.
            let mock_object_address = AccountAddress::from_hex_literal("0xcafe").unwrap();
            self.move_options
                .add_named_address(self.address_name.clone(), mock_object_address.to_string());
            let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;
            let mock_payloads = create_chunked_publish_payloads(
                package,
                PublishType::AccountDeploy,
                None,
                self.chunked_publish_option.large_packages_module_address,
                self.chunked_publish_option.chunk_size,
            )?
            .payloads;
            let staging_tx_count = (mock_payloads.len() - 1) as u64;
            self.txn_options.sequence_number(sender_address).await? + staging_tx_count + 1
        } else {
            self.txn_options.sequence_number(sender_address).await? + 1
        };

        let object_address = create_object_code_deployment_address(sender_address, sequence_number);

        self.move_options
            .add_named_address(self.address_name, object_address.to_string());

        let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;
        let message = format!(
            "Do you want to deploy this package at object address {}",
            object_address
        );
        prompt_yes_with_override(&message, self.txn_options.prompt_options)?;

        let result = if self.chunked_publish_option.chunked_publish {
            let payloads = create_chunked_publish_payloads(
                package,
                PublishType::ObjectDeploy,
                None,
                self.chunked_publish_option.large_packages_module_address,
                self.chunked_publish_option.chunk_size,
            )?
            .payloads;

            let size = &payloads
                .iter()
                .map(bcs::serialized_size)
                .sum::<Result<usize, _>>()?;
            println!("package size {} bytes", size);
            let message = format!("Publishing package in chunked mode will submit {} transactions for staging and publishing code.\n", &payloads.len());
            println!("{}", message.bold());

            submit_chunked_publish_transactions(
                payloads,
                &self.txn_options,
                self.chunked_publish_option.large_packages_module_address,
            )
            .await
        } else {
            let payload = create_package_publication_data(
                package,
                PublishType::ObjectDeploy,
                Some(object_address),
            )?
            .payload;

            let size = bcs::serialized_size(&payload)?;
            println!("package size {} bytes", size);

            if !self.override_size_check_option.override_size_check
                && size > MAX_PUBLISH_PACKAGE_SIZE
            {
                return Err(CliError::PackageSizeExceeded(
                    size,
                    MAX_PUBLISH_PACKAGE_SIZE,
                ));
            }
            self.txn_options
                .submit_transaction(payload)
                .await
                .map(TransactionSummary::from)
        };

        if result.is_ok() {
            println!(
                "Code was successfully deployed to object address {}",
                object_address
            );
        }
        result
    }
}

/// Upgrades the modules in a Move package deployed under an object.
#[derive(Parser)]
pub struct UpgradeCodeObject {
    /// The named address for compiling and using in the contract
    #[clap(long)]
    pub(crate) address_name: String,
    /// Address of the object the package was deployed to
    ///
    /// This must be an already deployed object containing the package
    /// if the package is not already created, it will fail.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) object_address: AccountAddress,
    #[clap(flatten)]
    pub(crate) override_size_check_option: OverrideSizeCheckOption,
    #[clap(flatten)]
    pub(crate) chunked_publish_option: ChunkedPublishOption,
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for UpgradeCodeObject {
    fn command_name(&self) -> &'static str {
        "UpgradeObject"
    }

    async fn execute(mut self) -> CliTypedResult<TransactionSummary> {
        self.move_options
            .add_named_address(self.address_name, self.object_address.to_string());

        let package = build_package_options(&self.move_options, &self.included_artifacts_args)?;
        let url = self
            .txn_options
            .rest_options
            .url(&self.txn_options.profile_options)?;

        // Get the `PackageRegistry` at the given code object address.
        let registry = CachedPackageRegistry::create(url, self.object_address, false).await?;
        let package_info = registry
            .get_package(package.name())
            .await
            .map_err(|s| CliError::CommandArgumentError(s.to_string()))?;

        if package_info.upgrade_policy() == UpgradePolicy::immutable() {
            return Err(CliError::CommandArgumentError(
                "A code package with upgrade policy `immutable` cannot be upgraded".to_owned(),
            ));
        }

        let message = format!(
            "Do you want to upgrade the code package '{}' at object address {}",
            package_info.name(),
            self.object_address
        );
        prompt_yes_with_override(&message, self.txn_options.prompt_options)?;

        let result = if self.chunked_publish_option.chunked_publish {
            let payloads = create_chunked_publish_payloads(
                package,
                PublishType::ObjectUpgrade,
                Some(self.object_address),
                self.chunked_publish_option.large_packages_module_address,
                self.chunked_publish_option.chunk_size,
            )?
            .payloads;

            let size = &payloads
                .iter()
                .map(bcs::serialized_size)
                .sum::<Result<usize, _>>()?;
            println!("package size {} bytes", size);
            let message = format!("Upgrading package in chunked mode will submit {} transactions for staging and upgrading code.\n", &payloads.len());
            println!("{}", message.bold());
            submit_chunked_publish_transactions(
                payloads,
                &self.txn_options,
                self.chunked_publish_option.large_packages_module_address,
            )
            .await
        } else {
            let payload = create_package_publication_data(
                package,
                PublishType::ObjectUpgrade,
                Some(self.object_address),
            )?
            .payload;

            let size = bcs::serialized_size(&payload)?;
            println!("package size {} bytes", size);

            if !self.override_size_check_option.override_size_check
                && size > MAX_PUBLISH_PACKAGE_SIZE
            {
                return Err(CliError::PackageSizeExceeded(
                    size,
                    MAX_PUBLISH_PACKAGE_SIZE,
                ));
            }
            self.txn_options
                .submit_transaction(payload)
                .await
                .map(TransactionSummary::from)
        };

        if result.is_ok() {
            println!(
                "Code was successfully upgraded at object address {}",
                self.object_address
            );
        }
        result
    }
}

fn build_package_options(
    move_options: &MovePackageOptions,
    included_artifacts_args: &IncludedArtifactsArgs,
) -> anyhow::Result<BuiltPackage> {
    let options = included_artifacts_args
        .included_artifacts
        .build_options(move_options)?;
    BuiltPackage::build(move_options.get_package_path()?, options)
}

async fn submit_chunked_publish_transactions(
    payloads: Vec<TransactionPayload>,
    txn_options: &TransactionOptions,
    large_packages_module_address: AccountAddress,
) -> CliTypedResult<TransactionSummary> {
    let mut publishing_result = Err(CliError::UnexpectedError(
        "No payload provided for batch transaction run".to_string(),
    ));
    let payloads_length = payloads.len() as u64;
    let mut tx_hashes = vec![];

    let account_address = txn_options.profile_options.account_address()?;

    if !is_staging_area_empty(txn_options, large_packages_module_address).await? {
        let message = format!(
            "The resource {}::large_packages::StagingArea under account {} is not empty.\
        \nThis may cause package publishing to fail if the data is unexpected. \
        \nUse the `aptos move clear-staging-area` command to clean up the `StagingArea` resource under the account.",
            large_packages_module_address, account_address,
        )
            .bold();
        println!("{}", message);
        prompt_yes_with_override("Do you want to proceed?", txn_options.prompt_options)?;
    }

    for (idx, payload) in payloads.into_iter().enumerate() {
        println!("Transaction {} of {}", idx + 1, payloads_length);
        let result = txn_options
            .submit_transaction(payload)
            .await
            .map(TransactionSummary::from);

        match result {
            Ok(tx_summary) => {
                let tx_hash = tx_summary.transaction_hash.to_string();
                let status = tx_summary.success.map_or_else(String::new, |success| {
                    if success {
                        "Success".to_string()
                    } else {
                        "Failed".to_string()
                    }
                });
                println!("Transaction executed: {} ({})\n", status, &tx_hash);
                tx_hashes.push(tx_hash);
                publishing_result = Ok(tx_summary);
            },

            Err(e) => {
                println!("{}", "Caution: An error occurred while submitting chunked publish transactions. \
                \nDue to this error, there may be incomplete data left in the `StagingArea` resource. \
                \nThis could cause further errors if you attempt to run the chunked publish command again. \
                \nTo avoid this, use the `aptos move clear-staging-area` command to clean up the `StagingArea` resource under your account before retrying.".bold());
                return Err(e);
            },
        }
    }

    println!(
        "{}",
        "All Transactions Submitted Successfully.".bold().green()
    );
    let tx_hash_formatted = format!(
        "Submitted Transactions:\n[\n    {}\n]",
        tx_hashes
            .iter()
            .map(|tx| format!("\"{}\"", tx))
            .collect::<Vec<_>>()
            .join(",\n    ")
    );
    println!("\n{}\n", tx_hash_formatted);
    publishing_result
}

async fn is_staging_area_empty(
    txn_options: &TransactionOptions,
    large_packages_module_address: AccountAddress,
) -> CliTypedResult<bool> {
    let url = txn_options.rest_options.url(&txn_options.profile_options)?;
    let client = Client::new(url);

    let staging_area_response = client
        .get_account_resource(
            txn_options.profile_options.account_address()?,
            &format!(
                "{}::large_packages::StagingArea",
                large_packages_module_address
            ),
        )
        .await;

    match staging_area_response {
        Ok(response) => match response.into_inner() {
            Some(_) => Ok(false), // StagingArea is not empty
            None => Ok(true),     // TODO: determine which case this is
        },
        Err(RestError::Api(aptos_error_response))
            if aptos_error_response.error.error_code == AptosErrorCode::ResourceNotFound =>
        {
            Ok(true) // The resource doesn't exist
        },
        Err(rest_err) => Err(CliError::from(rest_err)),
    }
}

/// Cleans up the `StagingArea` resource under an account, which is used for chunked publish operations
#[derive(Parser)]
pub struct ClearStagingArea {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,

    /// Address of the `large_packages` move module for chunked publishing
    #[clap(long, default_value = LARGE_PACKAGES_MODULE_ADDRESS, value_parser = crate::common::types::load_account_arg)]
    pub(crate) large_packages_module_address: AccountAddress,
}

#[async_trait]
impl CliCommand<TransactionSummary> for ClearStagingArea {
    fn command_name(&self) -> &'static str {
        "ClearStagingArea"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        println!(
            "Cleaning up resource {}::large_packages::StagingArea under account {}.",
            &self.large_packages_module_address,
            self.txn_options.profile_options.account_address()?
        );
        let payload = large_packages_cleanup_staging_area(self.large_packages_module_address);
        self.txn_options
            .submit_transaction(payload)
            .await
            .map(TransactionSummary::from)
    }
}

/// Publishes the modules in a Move package to the Aptos blockchain under a resource account
#[derive(Parser)]
pub struct CreateResourceAccountAndPublishPackage {
    /// The named address for compiling and using in the contract
    ///
    /// This will take the derived account address for the resource account and put it in this location
    #[clap(long)]
    pub(crate) address_name: String,

    #[clap(flatten)]
    pub(crate) override_size_check_option: OverrideSizeCheckOption,

    #[clap(flatten)]
    pub(crate) seed_args: ResourceAccountSeed,
    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for CreateResourceAccountAndPublishPackage {
    fn command_name(&self) -> &'static str {
        "ResourceAccountPublishPackage"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let CreateResourceAccountAndPublishPackage {
            address_name,
            mut move_options,
            txn_options,
            override_size_check_option,
            included_artifacts_args,
            seed_args,
        } = self;

        let account = if let Some(Some(account)) = CliConfig::load_profile(
            txn_options.profile_options.profile_name(),
            ConfigSearchMode::CurrentDirAndParents,
        )?
        .map(|p| p.account)
        {
            account
        } else {
            return Err(CliError::CommandArgumentError(
                "Please provide an account using --profile or run aptos init".to_string(),
            ));
        };
        let seed = seed_args.seed()?;

        let resource_address = create_resource_address(account, &seed);
        move_options.add_named_address(address_name, resource_address.to_string());

        let package_path = move_options.get_package_path()?;
        let options = included_artifacts_args
            .included_artifacts
            .build_options(&move_options)?;
        let package = BuiltPackage::build(package_path, options)?;
        let compiled_units = package.extract_code();

        // Send the compiled module and metadata using the code::publish_package_txn.
        let metadata = package.extract_metadata()?;

        let message = format!(
            "Do you want to publish this package under the resource account's address {}?",
            resource_address
        );
        prompt_yes_with_override(&message, txn_options.prompt_options)?;

        let payload = aptos_cached_packages::aptos_stdlib::resource_account_create_resource_account_and_publish_package(
            seed,
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            compiled_units,
        );
        let size = bcs::serialized_size(&payload)?;
        println!("package size {} bytes", size);
        if !override_size_check_option.override_size_check && size > MAX_PUBLISH_PACKAGE_SIZE {
            return Err(CliError::UnexpectedError(format!(
                "The package is larger than {} bytes ({} bytes)! To lower the size \
                you may want to include less artifacts via `--included-artifacts`. \
                You can also override this check with `--override-size-check",
                MAX_PUBLISH_PACKAGE_SIZE, size
            )));
        }
        txn_options
            .submit_transaction(payload)
            .await
            .map(TransactionSummary::from)
    }
}

/// Downloads a package and stores it in a directory named after the package
///
/// This lets you retrieve packages directly from the blockchain for inspection
/// and use as a local dependency in testing.
#[derive(Parser)]
pub struct DownloadPackage {
    /// Address of the account containing the package
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) account: AccountAddress,

    /// Name of the package
    #[clap(long)]
    pub package: String,

    /// Directory to store downloaded package. Defaults to the current directory.
    #[clap(long, value_parser)]
    pub output_dir: Option<PathBuf>,

    /// Whether to download the bytecode of the package.
    #[clap(long, short)]
    pub bytecode: bool,

    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    /// Print metadata of the package
    #[clap(long)]
    pub print_metadata: bool,
}

#[async_trait]
impl CliCommand<&'static str> for DownloadPackage {
    fn command_name(&self) -> &'static str {
        "DownloadPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let url = self.rest_options.url(&self.profile_options)?;
        let registry = CachedPackageRegistry::create(url, self.account, self.bytecode).await?;
        let output_dir = dir_default_to_current(self.output_dir)?;

        let package = registry
            .get_package(self.package)
            .await
            .map_err(|s| CliError::CommandArgumentError(s.to_string()))?;
        if package.upgrade_policy() == UpgradePolicy::arbitrary() {
            return Err(CliError::CommandArgumentError(
                "A package with upgrade policy `arbitrary` cannot be downloaded \
                since it is not safe to depend on such packages."
                    .to_owned(),
            ));
        }
        if self.print_metadata {
            println!("{}", package);
        }
        let package_path = output_dir.join(package.name());
        package
            .save_package_to_disk(package_path.as_path())
            .map_err(|e| CliError::UnexpectedError(format!("Failed to save package: {}", e)))?;
        if self.bytecode {
            for module in package.module_names() {
                if let Some(bytecode) = registry.get_bytecode(module).await? {
                    package.save_bytecode_to_disk(package_path.as_path(), module, bytecode)?
                }
            }
        };
        println!(
            "Saved package with {} module(s) to `{}`",
            package.module_names().len(),
            package_path.display()
        );
        Ok("Download succeeded")
    }
}

/// Downloads a package and verifies the bytecode
///
/// Downloads the package from onchain and verifies the bytecode matches a local compilation of the Move code
#[derive(Parser)]
pub struct VerifyPackage {
    /// Address of the account containing the package
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) account: AccountAddress,

    /// Artifacts to be generated when building this package.
    #[clap(long, default_value_t = IncludedArtifacts::Sparse)]
    pub(crate) included_artifacts: IncludedArtifacts,

    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[async_trait]
impl CliCommand<&'static str> for VerifyPackage {
    fn command_name(&self) -> &'static str {
        "VerifyPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        // First build the package locally to get the package metadata
        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            bytecode_version: fix_bytecode_version(
                self.move_options.bytecode_version,
                self.move_options.language_version,
            ),
            ..self.included_artifacts.build_options(&self.move_options)?
        };
        let pack = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;
        let compiled_metadata = pack.extract_metadata()?;

        // Now pull the compiled package
        let url = self.rest_options.url(&self.profile_options)?;
        let registry = CachedPackageRegistry::create(url, self.account, false).await?;
        let package = registry
            .get_package(pack.name())
            .await
            .map_err(|s| CliError::CommandArgumentError(s.to_string()))?;

        // We can't check the arbitrary, because it could change on us
        if package.upgrade_policy() == UpgradePolicy::arbitrary() {
            return Err(CliError::CommandArgumentError(
                "A package with upgrade policy `arbitrary` cannot be downloaded \
                since it is not safe to depend on such packages."
                    .to_owned(),
            ));
        }

        // Verify that the source digest matches
        package.verify(&compiled_metadata)?;

        Ok("Successfully verified source of package")
    }
}

/// Lists information about packages and modules on-chain for an account
#[derive(Parser)]
pub struct ListPackage {
    /// Address of the account for which to list packages.
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub(crate) account: AccountAddress,

    /// Type of items to query
    ///
    /// Current supported types `[packages]`
    #[clap(long, default_value_t = MoveListQuery::Packages)]
    query: MoveListQuery,

    #[clap(flatten)]
    rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum MoveListQuery {
    Packages,
}

impl Display for MoveListQuery {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MoveListQuery::Packages => "packages",
        })
    }
}

impl FromStr for MoveListQuery {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "packages" => Ok(MoveListQuery::Packages),
            _ => Err("Invalid query. Valid values are modules, packages"),
        }
    }
}

#[async_trait]
impl CliCommand<&'static str> for ListPackage {
    fn command_name(&self) -> &'static str {
        "ListPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let url = self.rest_options.url(&self.profile_options)?;
        let registry = CachedPackageRegistry::create(url, self.account, false).await?;
        match self.query {
            MoveListQuery::Packages => {
                for name in registry.package_names() {
                    let data = registry.get_package(name).await?;
                    println!("package {}", data.name());
                    println!("  upgrade_policy: {}", data.upgrade_policy());
                    println!("  upgrade_number: {}", data.upgrade_number());
                    println!("  source_digest: {}", data.source_digest());
                    println!("  modules: {}", data.module_names().into_iter().join(", "));
                }
            },
        }
        Ok("list succeeded")
    }
}

/// Cleans derived artifacts of a package.
#[derive(Parser)]
pub struct CleanPackage {
    #[clap(flatten)]
    pub(crate) move_options: MovePackageOptions,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
}

#[async_trait]
impl CliCommand<&'static str> for CleanPackage {
    fn command_name(&self) -> &'static str {
        "CleanPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let path = self.move_options.get_package_path()?;
        let build_dir = path.join("build");
        // Only remove the build dir if it exists, allowing for users to still clean their cache
        if build_dir.exists() {
            std::fs::remove_dir_all(build_dir.as_path())
                .map_err(|e| CliError::IO(build_dir.display().to_string(), e))?;
        }

        let move_dir = PathBuf::from(MOVE_HOME.as_str());
        if move_dir.exists()
            && prompt_yes_with_override(
                &format!(
                    "Do you also want to delete the local package download cache at `{}`?",
                    move_dir.display()
                ),
                self.prompt_options,
            )
            .is_ok()
        {
            std::fs::remove_dir_all(move_dir.as_path())
                .map_err(|e| CliError::IO(move_dir.display().to_string(), e))?;
        }
        Ok("succeeded")
    }
}

/// Run a Move function
#[derive(Parser)]
pub struct RunFunction {
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for RunFunction {
    fn command_name(&self) -> &'static str {
        "RunFunction"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        profile_or_submit(
            TransactionPayload::EntryFunction(self.entry_function_args.try_into()?),
            &self.txn_options,
        )
        .await
    }
}

/// BETA: Simulate a Move function or script
///
/// BETA: subject to change
///
/// This allows you to simulate and see the output from any function or Move script all in one command.
/// It additionally lets you simulate for any account
///
/// TODO: This should be simpler than the rest of the commands, soon to move other commands to a flow like this
#[derive(Parser)]
pub struct Simulate {
    #[clap(flatten)]
    txn_options: TxnOptions,
    // TODO: Mix entry function and script together with some smarts
    #[clap(flatten)]
    entry_function_args: EntryFunctionArguments,

    #[clap(long)]
    local: bool,
}

#[async_trait]
impl CliCommand<TransactionSummary> for Simulate {
    fn command_name(&self) -> &'static str {
        "Simulate"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let payload = TransactionPayload::EntryFunction(self.entry_function_args.try_into()?);

        if self.local {
            self.txn_options.simulate_locally(payload).await
        } else {
            self.txn_options.simulate_remotely(payload).await
        }
    }
}

/// Run a view function
#[derive(Parser)]
pub struct ViewFunction {
    #[clap(flatten)]
    pub(crate) entry_function_args: EntryFunctionArguments,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<serde_json::Value>> for ViewFunction {
    fn command_name(&self) -> &'static str {
        "RunViewFunction"
    }

    async fn execute(self) -> CliTypedResult<Vec<serde_json::Value>> {
        self.txn_options
            .view(self.entry_function_args.try_into()?)
            .await
    }
}

/// Run a Move script
#[derive(Parser)]
pub struct RunScript {
    #[clap(flatten)]
    pub txn_options: TransactionOptions,
    #[clap(flatten)]
    pub compile_proposal_args: CompileScriptFunction,
    #[clap(flatten)]
    pub script_function_args: ScriptFunctionArguments,
}

#[async_trait]
impl CliCommand<TransactionSummary> for RunScript {
    fn command_name(&self) -> &'static str {
        "RunScript"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let (bytecode, _script_hash) = self
            .compile_proposal_args
            .compile("RunScript", self.txn_options.prompt_options)?;

        profile_or_submit(
            self.script_function_args.create_script_payload(bytecode)?,
            &self.txn_options,
        )
        .await
    }
}

#[derive(Clone, Debug)]
pub enum ReplayNetworkSelection {
    Mainnet,
    Testnet,
    Devnet,
    RestEndpoint(String),
}

/// Replay a comitted transaction using a local VM.
#[derive(Parser, Debug)]
pub struct Replay {
    /// The network to replay on.
    ///
    /// Possible values:
    ///     mainnet, testnet, <REST_ENDPOINT_URL>
    #[clap(long)]
    pub(crate) network: ReplayNetworkSelection,

    /// The id of the transaction to replay. Also being referred to as "version" in some contexts.
    #[clap(long)]
    pub(crate) txn_id: u64,

    /// If this option is set, benchmark the transaction and report the running time(s).
    #[clap(long)]
    pub(crate) benchmark: bool,

    /// If this option is set, profile the transaction and generate a detailed report of its gas usage.
    #[clap(long)]
    pub(crate) profile_gas: bool,

    /// If present, skip the comparison against the expected transaction output.
    #[clap(long)]
    pub(crate) skip_comparison: bool,

    /// Key to use for ratelimiting purposes with the node API. This value will be used
    /// as `Authorization: Bearer <key>`
    #[clap(long)]
    pub(crate) node_api_key: Option<String>,
}

impl FromStr for ReplayNetworkSelection {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "mainnet" => Self::Mainnet,
            "testnet" => Self::Testnet,
            "devnet" => Self::Devnet,
            _ => Self::RestEndpoint(s.to_owned()),
        })
    }
}

#[async_trait]
impl CliCommand<TransactionSummary> for Replay {
    fn command_name(&self) -> &'static str {
        "Replay"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        use ReplayNetworkSelection::*;

        if self.profile_gas && self.benchmark {
            return Err(CliError::UnexpectedError(
                "Cannot perform benchmarking and gas profiling at the same time.".to_string(),
            ));
        }

        let rest_endpoint = match &self.network {
            Mainnet => "https://fullnode.mainnet.aptoslabs.com",
            Testnet => "https://fullnode.testnet.aptoslabs.com",
            Devnet => "https://fullnode.devnet.aptoslabs.com",
            RestEndpoint(url) => url,
        };

        // Build the client
        let client = Client::builder(AptosBaseUrl::Custom(
            Url::parse(rest_endpoint)
                .map_err(|_err| CliError::UnableToParse("url", rest_endpoint.to_string()))?,
        ));

        // add the node API key if it is provided
        let client = if let Some(api_key) = self.node_api_key {
            client.api_key(&api_key).unwrap().build()
        } else {
            client.build()
        };

        let debugger = AptosDebugger::rest_client(client)?;

        // Fetch the transaction to replay.
        let (txn, txn_info) = debugger
            .get_committed_transaction_at_version(self.txn_id)
            .await?;

        let txn = match txn {
            Transaction::UserTransaction(txn) => txn,
            _ => {
                return Err(CliError::UnexpectedError(
                    "Unsupported transaction type. Only user transactions are supported."
                        .to_string(),
                ))
            },
        };

        let hash = txn.committed_hash();

        // Execute the transaction.
        let (vm_status, vm_output) = if self.profile_gas {
            println!("Profiling transaction...");
            local_simulation::profile_transaction_using_debugger(
                &debugger,
                self.txn_id,
                txn.clone(),
                hash,
            )?
        } else if self.benchmark {
            println!("Benchmarking transaction...");
            local_simulation::benchmark_transaction_using_debugger(
                &debugger,
                self.txn_id,
                txn.clone(),
                hash,
            )?
        } else {
            println!("Replaying transaction...");
            local_simulation::run_transaction_using_debugger(
                &debugger,
                self.txn_id,
                txn.clone(),
                hash,
            )?
        };

        // Materialize into transaction output and check if the outputs match.
        let state_view = debugger.state_view_at_version(self.txn_id);
        let resolver = state_view.as_move_resolver();

        let txn_output = vm_output
            .try_materialize_into_transaction_output(&resolver)
            .map_err(|err| {
                CliError::UnexpectedError(format!(
                    "Failed to materialize into transaction output: {}",
                    err
                ))
            })?;

        if !self.skip_comparison {
            txn_output
                .ensure_match_transaction_info(self.txn_id, &txn_info, None, None)
                .map_err(|msg| CliError::UnexpectedError(msg.to_string()))?;
        }

        // Generate the transaction summary.
        let success = match txn_output.status() {
            TransactionStatus::Keep(exec_status) => Some(exec_status.is_success()),
            TransactionStatus::Discard(_) | TransactionStatus::Retry => None,
        };

        let summary = TransactionSummary {
            transaction_hash: txn.committed_hash().into(),
            gas_used: Some(txn_output.gas_used()),
            gas_unit_price: Some(txn.gas_unit_price()),
            pending: None,
            sender: Some(txn.sender()),
            sequence_number: Some(txn.sequence_number()),
            success,
            timestamp_us: None,
            version: Some(self.txn_id),
            vm_status: Some(vm_status.to_string()),
        };

        Ok(summary)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FunctionArgType {
    Address,
    Bool,
    Hex,
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Raw,
}

impl Display for FunctionArgType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionArgType::Address => write!(f, "address"),
            FunctionArgType::Bool => write!(f, "bool"),
            FunctionArgType::Hex => write!(f, "hex"),
            FunctionArgType::String => write!(f, "string"),
            FunctionArgType::U8 => write!(f, "u8"),
            FunctionArgType::U16 => write!(f, "u16"),
            FunctionArgType::U32 => write!(f, "u32"),
            FunctionArgType::U64 => write!(f, "u64"),
            FunctionArgType::U128 => write!(f, "u128"),
            FunctionArgType::U256 => write!(f, "u256"),
            FunctionArgType::Raw => write!(f, "raw"),
        }
    }
}

impl FunctionArgType {
    /// Parse a standalone argument (not a vector) from string slice into BCS representation.
    fn parse_arg_str(&self, arg: &str) -> CliTypedResult<Vec<u8>> {
        match self {
            FunctionArgType::Address => bcs::to_bytes(
                &load_account_arg(arg)
                    .map_err(|err| CliError::UnableToParse("address", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::Bool => bcs::to_bytes(
                &bool::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("bool", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::Hex => bcs::to_bytes(
                HexEncodedBytes::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("hex", err.to_string()))?
                    .inner(),
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::String => bcs::to_bytes(arg).map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::U8 => bcs::to_bytes(
                &u8::from_str(arg).map_err(|err| CliError::UnableToParse("u8", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::U16 => bcs::to_bytes(
                &u16::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u16", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::U32 => bcs::to_bytes(
                &u32::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u32", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::U64 => bcs::to_bytes(
                &u64::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u64", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::U128 => bcs::to_bytes(
                &u128::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u128", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::U256 => bcs::to_bytes(
                &U256::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u256", err.to_string()))?,
            )
            .map_err(|err| CliError::BCS("arg", err)),
            FunctionArgType::Raw => Ok(HexEncodedBytes::from_str(arg)
                .map_err(|err| CliError::UnableToParse("raw", err.to_string()))?
                .inner()
                .to_vec()),
        }
    }

    /// Recursively parse argument JSON into BCS representation.
    pub fn parse_arg_json(&self, arg: &serde_json::Value) -> CliTypedResult<ArgWithType> {
        match arg {
            serde_json::Value::Bool(value) => Ok(ArgWithType {
                _ty: self.clone(),
                _vector_depth: 0,
                arg: self.parse_arg_str(value.to_string().as_str())?,
            }),
            serde_json::Value::Number(value) => Ok(ArgWithType {
                _ty: self.clone(),
                _vector_depth: 0,
                arg: self.parse_arg_str(value.to_string().as_str())?,
            }),
            serde_json::Value::String(value) => Ok(ArgWithType {
                _ty: self.clone(),
                _vector_depth: 0,
                arg: self.parse_arg_str(value.as_str())?,
            }),
            serde_json::Value::Array(_) => {
                let mut bcs: Vec<u8> = vec![]; // BCS representation of argument.
                let mut common_sub_arg_depth = None;
                // Prepend argument sequence length to BCS bytes vector.
                write_u64_as_uleb128(&mut bcs, arg.as_array().unwrap().len());
                // Loop over all of the vector's sub-arguments, which may also be vectors:
                for sub_arg in arg.as_array().unwrap() {
                    let ArgWithType {
                        _ty: _,
                        _vector_depth: sub_arg_depth,
                        arg: mut sub_arg_bcs,
                    } = self.parse_arg_json(sub_arg)?;
                    // Verify all sub-arguments have same depth.
                    if let Some(check_depth) = common_sub_arg_depth {
                        if check_depth != sub_arg_depth {
                            return Err(CliError::CommandArgumentError(
                                "Variable vector depth".to_string(),
                            ));
                        }
                    };
                    common_sub_arg_depth = Some(sub_arg_depth);
                    bcs.append(&mut sub_arg_bcs); // Append sub-argument BCS.
                }
                // Default sub-argument depth is 0 for when no sub-arguments were looped over.
                Ok(ArgWithType {
                    _ty: self.clone(),
                    _vector_depth: common_sub_arg_depth.unwrap_or(0) + 1,
                    arg: bcs,
                })
            },
            serde_json::Value::Null => {
                Err(CliError::CommandArgumentError("Null argument".to_string()))
            },
            serde_json::Value::Object(_) => Err(CliError::CommandArgumentError(
                "JSON object argument".to_string(),
            )),
        }
    }
}

// TODO use from move_binary_format::file_format_common if it is made public.
fn write_u64_as_uleb128(binary: &mut Vec<u8>, mut val: usize) {
    loop {
        let cur = val & 0x7F;
        if cur != val {
            binary.push((cur | 0x80) as u8);
            val >>= 7;
        } else {
            binary.push(cur as u8);
            break;
        }
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
            "u16" => Ok(FunctionArgType::U16),
            "u32" => Ok(FunctionArgType::U32),
            "u64" => Ok(FunctionArgType::U64),
            "u128" => Ok(FunctionArgType::U128),
            "u256" => Ok(FunctionArgType::U256),
            "raw" => Ok(FunctionArgType::Raw),
            str => {
                Err(CliError::CommandArgumentError(format!(
                    "Invalid arg type '{}'.  Must be one of: ['{}','{}','{}','{}','{}','{}','{}','{}','{}','{}','{}']",
                    str,
                    FunctionArgType::Address,
                    FunctionArgType::Bool,
                    FunctionArgType::Hex,
                    FunctionArgType::String,
                    FunctionArgType::U8,
                    FunctionArgType::U16,
                    FunctionArgType::U32,
                    FunctionArgType::U64,
                    FunctionArgType::U128,
                    FunctionArgType::U256,
                    FunctionArgType::Raw)))
            }
        }
    }
}

/// A parseable arg with a type separated by a colon
#[derive(Clone, Debug)]
pub struct ArgWithType {
    pub(crate) _ty: FunctionArgType,
    pub(crate) _vector_depth: u8,
    pub(crate) arg: Vec<u8>,
}

impl ArgWithType {
    pub fn address(account_address: AccountAddress) -> Self {
        ArgWithType {
            _ty: FunctionArgType::Address,
            _vector_depth: 0,
            arg: bcs::to_bytes(&account_address).unwrap(),
        }
    }

    pub fn u64(arg: u64) -> Self {
        ArgWithType {
            _ty: FunctionArgType::U64,
            _vector_depth: 0,
            arg: bcs::to_bytes(&arg).unwrap(),
        }
    }

    pub fn bytes(arg: Vec<u8>) -> Self {
        ArgWithType {
            _ty: FunctionArgType::Raw,
            _vector_depth: 0,
            arg: bcs::to_bytes(&arg).unwrap(),
        }
    }

    pub fn raw(arg: Vec<u8>) -> Self {
        ArgWithType {
            _ty: FunctionArgType::Raw,
            _vector_depth: 0,
            arg,
        }
    }

    pub fn bcs_value_to_json<'a, T: Deserialize<'a> + Serialize>(
        &'a self,
    ) -> CliTypedResult<serde_json::Value> {
        match self._vector_depth {
            0 => match self._ty.clone() {
                FunctionArgType::U64 => {
                    serde_json::to_value(bcs::from_bytes::<u64>(&self.arg)?.to_string())
                        .map_err(|err| CliError::UnexpectedError(err.to_string()))
                },
                FunctionArgType::U128 => {
                    serde_json::to_value(bcs::from_bytes::<u128>(&self.arg)?.to_string())
                        .map_err(|err| CliError::UnexpectedError(err.to_string()))
                },
                FunctionArgType::U256 => {
                    serde_json::to_value(bcs::from_bytes::<U256>(&self.arg)?.to_string())
                        .map_err(|err| CliError::UnexpectedError(err.to_string()))
                },
                FunctionArgType::Raw => serde_json::to_value(&self.arg)
                    .map_err(|err| CliError::UnexpectedError(err.to_string())),
                _ => serde_json::to_value(bcs::from_bytes::<T>(&self.arg)?)
                    .map_err(|err| CliError::UnexpectedError(err.to_string())),
            },
            1 => match self._ty.clone() {
                FunctionArgType::U64 => {
                    let u64_vector: Vec<u64> = bcs::from_bytes::<Vec<u64>>(&self.arg)?;
                    let string_vector: Vec<String> =
                        u64_vector.iter().map(ToString::to_string).collect();
                    serde_json::to_value(string_vector)
                        .map_err(|err| CliError::UnexpectedError(err.to_string()))
                },
                FunctionArgType::U128 => {
                    let u128_vector: Vec<u128> = bcs::from_bytes::<Vec<u128>>(&self.arg)?;
                    let string_vector: Vec<String> =
                        u128_vector.iter().map(ToString::to_string).collect();
                    serde_json::to_value(string_vector)
                        .map_err(|err| CliError::UnexpectedError(err.to_string()))
                },
                FunctionArgType::U256 => {
                    let u256_vector: Vec<U256> = bcs::from_bytes::<Vec<U256>>(&self.arg)?;
                    let string_vector: Vec<String> =
                        u256_vector.iter().map(ToString::to_string).collect();
                    serde_json::to_value(string_vector)
                        .map_err(|err| CliError::UnexpectedError(err.to_string()))
                },
                FunctionArgType::Raw => serde_json::to_value(&self.arg)
                    .map_err(|err| CliError::UnexpectedError(err.to_string())),
                _ => serde_json::to_value(bcs::from_bytes::<Vec<T>>(&self.arg)?)
                    .map_err(|err| CliError::UnexpectedError(err.to_string())),
            },

            2 => serde_json::to_value(bcs::from_bytes::<Vec<Vec<T>>>(&self.arg)?)
                .map_err(|err| CliError::UnexpectedError(err.to_string())),

            3 => serde_json::to_value(bcs::from_bytes::<Vec<Vec<Vec<T>>>>(&self.arg)?)
                .map_err(|err| CliError::UnexpectedError(err.to_string())),

            4 => serde_json::to_value(bcs::from_bytes::<Vec<Vec<Vec<Vec<T>>>>>(&self.arg)?)
                .map_err(|err| CliError::UnexpectedError(err.to_string())),
            5 => serde_json::to_value(bcs::from_bytes::<Vec<Vec<Vec<Vec<Vec<T>>>>>>(&self.arg)?)
                .map_err(|err| CliError::UnexpectedError(err.to_string())),
            6 => serde_json::to_value(bcs::from_bytes::<Vec<Vec<Vec<Vec<Vec<Vec<T>>>>>>>(
                &self.arg,
            )?)
            .map_err(|err| CliError::UnexpectedError(err.to_string())),
            7 => serde_json::to_value(bcs::from_bytes::<Vec<Vec<Vec<Vec<Vec<Vec<Vec<T>>>>>>>>(
                &self.arg,
            )?)
            .map_err(|err| CliError::UnexpectedError(err.to_string())),
            depth => Err(CliError::UnexpectedError(format!(
                "Vector of depth {depth} is overly nested"
            ))),
        }
    }

    pub fn to_json(&self) -> CliTypedResult<serde_json::Value> {
        match self._ty {
            FunctionArgType::Address => self.bcs_value_to_json::<AccountAddress>(),
            FunctionArgType::Bool => self.bcs_value_to_json::<bool>(),
            FunctionArgType::Hex => self.bcs_value_to_json::<Vec<u8>>(),
            FunctionArgType::String => self.bcs_value_to_json::<String>(),
            FunctionArgType::U8 => self.bcs_value_to_json::<u8>(),
            FunctionArgType::U16 => self.bcs_value_to_json::<u16>(),
            FunctionArgType::U32 => self.bcs_value_to_json::<u32>(),
            FunctionArgType::U64 => self.bcs_value_to_json::<u64>(),
            FunctionArgType::U128 => self.bcs_value_to_json::<u128>(),
            FunctionArgType::U256 => self.bcs_value_to_json::<U256>(),
            FunctionArgType::Raw => serde_json::to_value(&self.arg)
                .map_err(|err| CliError::UnexpectedError(err.to_string())),
        }
        .map_err(|err| {
            CliError::UnexpectedError(format!("Failed to parse argument to JSON {}", err))
        })
    }
}

/// Does not support string arguments that contain the following characters:
///
/// * `,`
/// * `[`
/// * `]`
impl FromStr for ArgWithType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Splits on the first colon, returning at most `2` elements
        // This is required to support args that contain a colon
        let parts: Vec<_> = s.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(CliError::CommandArgumentError(
                "Arguments must be pairs of <type>:<arg> e.g. bool:true".to_string(),
            ));
        }
        let ty = FunctionArgType::from_str(parts.first().unwrap())?;
        let mut arg = String::from(*parts.last().unwrap());
        // May need to surround with quotes if not an array, so arg can be parsed into JSON.
        if !arg.starts_with('[') {
            if let FunctionArgType::Address
            | FunctionArgType::Hex
            | FunctionArgType::String
            | FunctionArgType::Raw = ty
            {
                arg = format!("\"{}\"", arg);
            }
        }
        let json = serde_json::from_str::<serde_json::Value>(arg.as_str())
            .map_err(|err| CliError::UnexpectedError(err.to_string()))?;
        ty.parse_arg_json(&json)
    }
}

impl TryInto<TransactionArgument> for &ArgWithType {
    type Error = CliError;

    fn try_into(self) -> Result<TransactionArgument, Self::Error> {
        if self._vector_depth > 0 && self._ty != FunctionArgType::U8 {
            return Err(CliError::UnexpectedError(
                "Unable to parse non-u8 vector to transaction argument".to_string(),
            ));
        }
        match self._ty {
            FunctionArgType::Address => Ok(TransactionArgument::Address(txn_arg_parser(
                &self.arg, "address",
            )?)),
            FunctionArgType::Bool => Ok(TransactionArgument::Bool(txn_arg_parser(
                &self.arg, "bool",
            )?)),
            FunctionArgType::Hex => Ok(TransactionArgument::U8Vector(txn_arg_parser(
                &self.arg, "hex",
            )?)),
            FunctionArgType::String => Ok(TransactionArgument::U8Vector(txn_arg_parser(
                &self.arg, "string",
            )?)),
            FunctionArgType::U8 => match self._vector_depth {
                0 => Ok(TransactionArgument::U8(txn_arg_parser(&self.arg, "u8")?)),
                1 => Ok(TransactionArgument::U8Vector(txn_arg_parser(
                    &self.arg,
                    "vector<u8>",
                )?)),
                depth => Err(CliError::UnexpectedError(format!(
                    "Unable to parse u8 vector of depth {} to transaction argument",
                    depth
                ))),
            },
            FunctionArgType::U16 => Ok(TransactionArgument::U16(txn_arg_parser(&self.arg, "u16")?)),
            FunctionArgType::U32 => Ok(TransactionArgument::U32(txn_arg_parser(&self.arg, "u32")?)),
            FunctionArgType::U64 => Ok(TransactionArgument::U64(txn_arg_parser(&self.arg, "u64")?)),
            FunctionArgType::U128 => Ok(TransactionArgument::U128(txn_arg_parser(
                &self.arg, "u128",
            )?)),
            FunctionArgType::U256 => Ok(TransactionArgument::U256(txn_arg_parser(
                &self.arg, "u256",
            )?)),
            FunctionArgType::Raw => Ok(TransactionArgument::U8Vector(txn_arg_parser(
                &self.arg, "raw",
            )?)),
        }
    }
}

fn txn_arg_parser<T: serde::de::DeserializeOwned>(
    data: &[u8],
    label: &'static str,
) -> Result<T, CliError> {
    bcs::from_bytes(data).map_err(|err| CliError::UnableToParse(label, err.to_string()))
}

/// Identifier of a module member (function or struct).
/// Duplicated from aptos_types, as we also need to load_account_arg from the CLI.
#[derive(Debug, Clone)]
pub struct MemberId {
    pub module_id: ModuleId,
    pub member_id: Identifier,
}

fn parse_member_id(function_id: &str) -> CliTypedResult<MemberId> {
    let ids: Vec<&str> = function_id.split_terminator("::").collect();
    if ids.len() != 3 {
        return Err(CliError::CommandArgumentError(
            "FunctionId is not well formed.  Must be of the form <address>::<module>::<function>"
                .to_string(),
        ));
    }
    let address = load_account_arg(ids.first().unwrap())?;
    let module = Identifier::from_str(ids.get(1).unwrap())
        .map_err(|err| CliError::UnableToParse("Module Name", err.to_string()))?;
    let member_id = Identifier::from_str(ids.get(2).unwrap())
        .map_err(|err| CliError::UnableToParse("Member Name", err.to_string()))?;
    let module_id = ModuleId::new(address, module);
    Ok(MemberId {
        module_id,
        member_id,
    })
}

impl FromStr for MemberId {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_member_id(s)
    }
}
