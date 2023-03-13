// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod aptos_debug_natives;
mod manifest;
pub mod package_hooks;

pub use package_hooks::*;

pub mod stored_package;
mod transactional_tests_runner;

use crate::{
    common::{
        types::{
            load_account_arg, CliConfig, CliError, CliTypedResult, ConfigSearchMode,
            MoveManifestAccountWrapper, MovePackageDir, ProfileOptions, PromptOptions, RestOptions,
            TransactionOptions, TransactionSummary,
        },
        utils::{
            check_if_file_exists, create_dir_if_not_exist, dir_default_to_current,
            prompt_yes_with_override, write_to_file,
        },
    },
    governance::CompileScriptFunction,
    move_tool::manifest::{Dependency, ManifestNamedAddress, MovePackageManifest, PackageInfo},
    CliCommand, CliResult,
};
use aptos_crypto::HashValue;
use aptos_framework::{
    build_model, docgen::DocgenOptions, extended_checks, natives::code::UpgradePolicy,
    prover::ProverOptions, BuildOptions, BuiltPackage,
};
use aptos_gas::{AbstractValueSizeGasParameters, NativeGasParameters};
use aptos_rest_client::aptos_api_types::{EntryFunctionId, MoveType, ViewRequest};
use aptos_transactional_test_harness::run_aptos_test;
use aptos_types::{
    account_address::{create_resource_address, AccountAddress},
    transaction::{EntryFunction, Script, TransactionArgument, TransactionPayload},
};
use async_trait::async_trait;
use clap::{ArgEnum, Parser, Subcommand};
use codespan_reporting::{
    diagnostic::Severity,
    term::termcolor::{ColorChoice, StandardStream},
};
use itertools::Itertools;
use move_cli::{
    self,
    base::{coverage::CoverageSummaryOptions, test::UnitTestResult},
};
use move_command_line_common::env::MOVE_HOME;
use move_compiler::compiled_unit::{CompiledUnit, NamedCompiledModule};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    u256::U256,
};
use move_coverage::{
    coverage_map::CoverageMap, format_csv_summary, format_human_summary,
    source_coverage::SourceCoverageBuilder, summary::summarize_inst_cov,
};
use move_disassembler::disassembler::Disassembler;
use move_package::{source_package::layout::SourcePackageLayout, BuildConfig};
use move_unit_test::UnitTestingConfig;
use serde::Serialize;
use std::{
    collections::BTreeMap,
    convert::TryFrom,
    fmt::{Display, Formatter},
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};
pub use stored_package::*;
use tokio::task;
use transactional_tests_runner::TransactionalTestOpts;

/// Tool for Move related operations
///
/// This tool lets you compile, test, and publish Move code, in addition
/// to run any other tools that help run, verify, or provide information
/// about this code.
#[derive(Subcommand)]
pub enum MoveTool {
    Compile(CompilePackage),
    CompileScript(CompileScript),
    Init(InitPackage),
    Publish(PublishPackage),
    Download(DownloadPackage),
    List(ListPackage),
    Clean(CleanPackage),
    VerifyPackage(VerifyPackage),
    Run(RunFunction),
    RunScript(RunScript),
    Test(TestPackage),
    Prove(ProvePackage),
    Document(DocumentPackage),
    TransactionalTest(TransactionalTestOpts),
    CreateResourceAccountAndPublishPackage(CreateResourceAccountAndPublishPackage),
    View(ViewFunction),
    Coverage(CoveragePackage),
}

impl MoveTool {
    pub async fn execute(self) -> CliResult {
        match self {
            MoveTool::Compile(tool) => tool.execute_serialized().await,
            MoveTool::CompileScript(tool) => tool.execute_serialized().await,
            MoveTool::Init(tool) => tool.execute_serialized_success().await,
            MoveTool::Publish(tool) => tool.execute_serialized().await,
            MoveTool::Download(tool) => tool.execute_serialized().await,
            MoveTool::List(tool) => tool.execute_serialized().await,
            MoveTool::Clean(tool) => tool.execute_serialized().await,
            MoveTool::VerifyPackage(tool) => tool.execute_serialized().await,
            MoveTool::Run(tool) => tool.execute_serialized().await,
            MoveTool::RunScript(tool) => tool.execute_serialized().await,
            MoveTool::Test(tool) => tool.execute_serialized().await,
            MoveTool::Prove(tool) => tool.execute_serialized().await,
            MoveTool::Document(tool) => tool.execute_serialized().await,
            MoveTool::TransactionalTest(tool) => tool.execute_serialized_success().await,
            MoveTool::CreateResourceAccountAndPublishPackage(tool) => {
                tool.execute_serialized_success().await
            },
            MoveTool::View(tool) => tool.execute_serialized().await,
            MoveTool::Coverage(tool) => tool.execute_serialized().await,
        }
    }
}

#[derive(Parser)]
pub struct FrameworkPackageArgs {
    /// Git revision or branch for the Aptos framework
    ///
    /// This is mutually exclusive with `--framework-local-dir`
    #[clap(long, group = "framework_package_args")]
    pub(crate) framework_git_rev: Option<String>,

    /// Local framework directory for the Aptos framework
    ///
    /// This is mutually exclusive with `--framework-git-rev`
    #[clap(long, parse(from_os_str), group = "framework_package_args")]
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
        const APTOS_GIT_PATH: &str = "https://github.com/aptos-labs/aptos-core.git";
        const SUBDIR_PATH: &str = "aptos-move/framework/aptos-framework";
        const DEFAULT_BRANCH: &str = "main";

        let move_toml = package_dir.join(SourcePackageLayout::Manifest.path());
        check_if_file_exists(move_toml.as_path(), prompt_options)?;
        create_dir_if_not_exist(
            package_dir
                .join(SourcePackageLayout::Sources.path())
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
                author: None,
            },
            addresses,
            dependencies,
        };

        write_to_file(
            move_toml.as_path(),
            SourcePackageLayout::Manifest.location_str(),
            toml::to_string_pretty(&manifest)
                .map_err(|err| CliError::UnexpectedError(err.to_string()))?
                .as_bytes(),
        )
    }
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
    #[clap(long, parse(from_os_str))]
    pub(crate) package_dir: Option<PathBuf>,

    /// Named addresses for the move binary
    ///
    /// Allows for an address to be put into the Move.toml, or a placeholder `_`
    ///
    /// Example: alice=0x1234,bob=0x5678,greg=_
    ///
    /// Note: This will fail if there are duplicates in the Move.toml file remove those first.
    #[clap(long, parse(try_from_str = crate::common::utils::parse_map), default_value = "")]
    pub(crate) named_addresses: BTreeMap<String, MoveManifestAccountWrapper>,

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
        let addresses = self
            .named_addresses
            .into_iter()
            .map(|(key, value)| (key, value.account_address.into()))
            .collect();

        self.framework_package_args.init_move_dir(
            package_dir.as_path(),
            &self.name,
            addresses,
            self.prompt_options,
        )
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
    pub(crate) save_metadata: bool,

    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
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
                .build_options(
                    self.move_options.skip_fetch_latest_git_deps,
                    self.move_options.named_addresses(),
                    self.move_options.bytecode_version,
                )
        };
        let pack = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;
        if self.save_metadata {
            pack.extract_metadata_and_save()?;
        }
        let ids = pack
            .modules()
            .into_iter()
            .map(|m| m.self_id().to_string())
            .collect::<Vec<_>>();
        Ok(ids)
    }
}

/// Compiles a Move script into bytecode
///
/// Compiles a script into bytecode and provides a hash of the bytecode.
/// This can then be run with `aptos move run-script`
#[derive(Parser)]
pub struct CompileScript {
    #[clap(long, parse(from_os_str))]
    pub output_file: Option<PathBuf>,
    #[clap(flatten)]
    pub move_options: MovePackageDir,
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
            ..IncludedArtifacts::None.build_options(
                self.move_options.skip_fetch_latest_git_deps,
                self.move_options.named_addresses(),
                self.move_options.bytecode_version,
            )
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
    pub(crate) move_options: MovePackageDir,

    /// The maximum number of instructions that can be executed by a test
    ///
    /// If set, the number of instructions executed by one test will be bounded
    // TODO: Remove short, it's against the style guidelines, and update the name here
    #[clap(
        name = "instructions",
        default_value = "100000",
        short = 'i',
        long = "instructions"
    )]
    pub instruction_execution_bound: u64,
    /// Collect coverage information for later use with the various `aptos move coverage` subcommands
    #[clap(long = "coverage")]
    pub compute_coverage: bool,
}

#[async_trait]
impl CliCommand<&'static str> for TestPackage {
    fn command_name(&self) -> &'static str {
        "TestPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let mut config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
            test_mode: true,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };

        // Build the Move model for extended checks
        let model = &build_model(
            self.move_options.get_package_path()?.as_path(),
            self.move_options.named_addresses(),
            None,
            self.move_options.bytecode_version,
        )?;
        let _ = extended_checks::run_extended_checks(model);
        if model.diag_count(Severity::Warning) > 0 {
            let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
            model.report_diag(&mut error_writer, Severity::Warning);
            if model.has_errors() {
                return Err(CliError::MoveCompilationError(
                    "extended checks failed".to_string(),
                ));
            }
        }
        let path = self.move_options.get_package_path()?;
        let result = move_cli::base::test::run_move_unit_tests(
            path.as_path(),
            config.clone(),
            UnitTestingConfig {
                filter: self.filter.clone(),
                report_stacktrace_on_abort: true,
                ignore_compile_warnings: self.ignore_compile_warnings,
                ..UnitTestingConfig::default_with_bound(None)
            },
            // TODO(Gas): we may want to switch to non-zero costs in the future
            aptos_debug_natives::aptos_debug_natives(
                NativeGasParameters::zeros(),
                AbstractValueSizeGasParameters::zeros(),
            ),
            None,
            self.compute_coverage,
            &mut std::io::stdout(),
        )
        .map_err(|err| CliError::UnexpectedError(err.to_string()))?;

        // Print coverage summary if --coverage is set
        if self.compute_coverage {
            config.test_mode = false;
            let summary = CoverageSummaryOptions::Summary {
                functions: false,
                output_csv: false,
            };
            generate_coverage_info(summary, path.as_path(), config, self.filter).unwrap();
            println!("Please use `aptos move coverage -h` for more detailed test coverage of this package");
        }

        match result {
            UnitTestResult::Success => Ok("Success"),
            UnitTestResult::Failure => Err(CliError::MoveTestError),
        }
    }
}

#[async_trait]
impl CliCommand<()> for TransactionalTestOpts {
    fn command_name(&self) -> &'static str {
        "TransactionalTest"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let root_path = self.root_path.display().to_string();

        let requirements = vec![transactional_tests_runner::Requirements::new(
            run_aptos_test,
            "tests".to_string(),
            root_path,
            self.pattern.clone(),
        )];

        transactional_tests_runner::runner(&self, &requirements)
    }
}

/// Proves a Move package
///
/// This is a tool for formal verification of a Move package using
/// the Move prover
#[derive(Parser)]
pub struct ProvePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,

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

        let result = task::spawn_blocking(move || {
            prover_options.prove(
                move_options.get_package_path()?.as_path(),
                move_options.named_addresses(),
                move_options.bytecode_version,
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

/// Generate coverage info
fn generate_coverage_info(
    options: CoverageSummaryOptions,
    path: &Path,
    config: BuildConfig,
    filter: Option<String>,
) -> anyhow::Result<()> {
    let coverage_map = CoverageMap::from_binary_file(path.join(".coverage_map.mvcov"))?;
    let package = config.compile_package(path, &mut Vec::new())?;
    match options {
        CoverageSummaryOptions::Source { module_name } => {
            let unit = package.get_module_by_name_from_root(&module_name)?;
            let source_path = &unit.source_path;
            let (module, source_map) = match &unit.unit {
                CompiledUnit::Module(NamedCompiledModule {
                    module, source_map, ..
                }) => (module, source_map),
                _ => panic!("Should all be modules"),
            };
            let source_coverage = SourceCoverageBuilder::new(module, &coverage_map, source_map);
            source_coverage
                .compute_source_coverage(source_path)
                .output_source_coverage(&mut std::io::stdout())
                .unwrap();
        },
        CoverageSummaryOptions::Summary {
            functions,
            output_csv,
            ..
        } => {
            let modules: Vec<_> = package
                .root_modules()
                .filter_map(|unit| {
                    let mut retain = true;
                    if let Some(filter_str) = &filter {
                        if !&unit.unit.name().as_str().contains(filter_str.as_str()) {
                            retain = false;
                        }
                    }
                    match &unit.unit {
                        CompiledUnit::Module(NamedCompiledModule { module, .. }) if retain => {
                            Some(module.clone())
                        },
                        _ => None,
                    }
                })
                .collect();
            let coverage_map = coverage_map.to_unified_exec_map();
            if output_csv {
                format_csv_summary(
                    modules.as_slice(),
                    &coverage_map,
                    summarize_inst_cov,
                    &mut std::io::stdout(),
                )
            } else {
                format_human_summary(
                    modules.as_slice(),
                    &coverage_map,
                    summarize_inst_cov,
                    &mut std::io::stdout(),
                    functions,
                )
            }
        },
        CoverageSummaryOptions::Bytecode { module_name } => {
            let unit = package.get_module_by_name_from_root(&module_name)?;
            let mut disassembler = Disassembler::from_unit(&unit.unit);
            disassembler.add_coverage_map(coverage_map.to_unified_exec_map());
            println!("{}", disassembler.disassemble()?);
        },
    }
    Ok(())
}

/// Computes coverage for a package
///
/// Computes coverage on a previous unit test run for a package.  Coverage input must
/// first be built with `aptos move test --coverage`
#[derive(Parser)]
pub struct CoveragePackage {
    #[clap(flatten)]
    move_options: MovePackageDir,

    #[clap(subcommand)]
    pub coverage_options: CoverageSummaryOptions,
}

#[async_trait]
impl CliCommand<&'static str> for CoveragePackage {
    fn command_name(&self) -> &'static str {
        "CoveragePackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let config = BuildConfig {
            additional_named_addresses: self.move_options.named_addresses(),
            test_mode: false,
            install_dir: self.move_options.output_dir.clone(),
            ..Default::default()
        };
        let path = self.move_options.get_package_path()?;
        let result = generate_coverage_info(self.coverage_options, path.as_path(), config, None);
        match result {
            Ok(_) => Ok("Success"),
            Err(e) => Err(CliError::CoverageError(format!("{:#}", e))),
        }
    }
}

/// Documents a Move package
///
/// This converts the content of the package into markdown for documentation.
#[derive(Parser)]
pub struct DocumentPackage {
    #[clap(flatten)]
    move_options: MovePackageDir,

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
            with_srcs: false,
            with_abis: false,
            with_source_maps: false,
            with_error_map: false,
            with_docs: true,
            install_dir: None,
            named_addresses: move_options.named_addresses(),
            docgen_options: Some(docgen_options),
            skip_fetch_latest_git_deps: move_options.skip_fetch_latest_git_deps,
            bytecode_version: move_options.bytecode_version,
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
    pub(crate) included_artifacts: IncludedArtifacts,
}

/// Publishes the modules in a Move package to the Aptos blockchain
#[derive(Parser)]
pub struct PublishPackage {
    /// Whether to override the check for maximal size of published data
    #[clap(long)]
    pub(crate) override_size_check: bool,

    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[derive(ArgEnum, Clone, Copy, Debug)]
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
        skip_fetch_latest_git_deps: bool,
        named_addresses: BTreeMap<String, AccountAddress>,
        bytecode_version: Option<u32>,
    ) -> BuildOptions {
        use IncludedArtifacts::*;
        match self {
            None => BuildOptions {
                with_srcs: false,
                with_abis: false,
                with_source_maps: false,
                // Always enable error map bytecode injection
                with_error_map: true,
                named_addresses,
                skip_fetch_latest_git_deps,
                bytecode_version,
                ..BuildOptions::default()
            },
            Sparse => BuildOptions {
                with_srcs: true,
                with_abis: false,
                with_source_maps: false,
                with_error_map: true,
                named_addresses,
                skip_fetch_latest_git_deps,
                bytecode_version,
                ..BuildOptions::default()
            },
            All => BuildOptions {
                with_srcs: true,
                with_abis: true,
                with_source_maps: true,
                with_error_map: true,
                named_addresses,
                skip_fetch_latest_git_deps,
                bytecode_version,
                ..BuildOptions::default()
            },
        }
    }
}

pub const MAX_PUBLISH_PACKAGE_SIZE: usize = 60_000;

#[async_trait]
impl CliCommand<TransactionSummary> for PublishPackage {
    fn command_name(&self) -> &'static str {
        "PublishPackage"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let PublishPackage {
            move_options,
            txn_options,
            override_size_check,
            included_artifacts_args,
        } = self;
        let package_path = move_options.get_package_path()?;
        let options = included_artifacts_args.included_artifacts.build_options(
            move_options.skip_fetch_latest_git_deps,
            move_options.named_addresses(),
            move_options.bytecode_version,
        );
        let package = BuiltPackage::build(package_path, options)?;
        let compiled_units = package.extract_code();

        // Send the compiled module and metadata using the code::publish_package_txn.
        let metadata = package.extract_metadata()?;
        let payload = aptos_cached_packages::aptos_stdlib::code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            compiled_units,
        );
        let size = bcs::serialized_size(&payload)?;
        println!("package size {} bytes", size);
        if !override_size_check && size > MAX_PUBLISH_PACKAGE_SIZE {
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

/// Publishes the modules in a Move package to the Aptos blockchain under a resource account
#[derive(Parser)]
pub struct CreateResourceAccountAndPublishPackage {
    #[clap(long)]
    pub(crate) seed: String,

    #[clap(long)]
    pub(crate) address_name: String,

    /// Whether to override the check for maximal size of published data
    #[clap(long)]
    pub(crate) override_size_check: bool,

    #[clap(flatten)]
    pub(crate) included_artifacts_args: IncludedArtifactsArgs,
    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
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
            seed,
            address_name,
            mut move_options,
            txn_options,
            override_size_check,
            included_artifacts_args,
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

        let resource_address =
            create_resource_address(account, &bcs::to_bytes(&seed.clone()).unwrap());
        move_options.add_named_address(address_name, resource_address.to_string());

        let package_path = move_options.get_package_path()?;
        let options = included_artifacts_args.included_artifacts.build_options(
            move_options.skip_fetch_latest_git_deps,
            move_options.named_addresses(),
            move_options.bytecode_version,
        );
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
            bcs::to_bytes(&seed)?,
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            compiled_units,
        );
        let size = bcs::serialized_size(&payload)?;
        println!("package size {} bytes", size);
        if !override_size_check && size > MAX_PUBLISH_PACKAGE_SIZE {
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
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Name of the package
    #[clap(long)]
    pub package: String,

    /// Directory to store downloaded package. Defaults to the current directory.
    #[clap(long, parse(from_os_str))]
    pub output_dir: Option<PathBuf>,

    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[async_trait]
impl CliCommand<&'static str> for DownloadPackage {
    fn command_name(&self) -> &'static str {
        "DownloadPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        let url = self.rest_options.url(&self.profile_options)?;
        let registry = CachedPackageRegistry::create(url, self.account).await?;
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
        let package_path = output_dir.join(package.name());
        package
            .save_package_to_disk(package_path.as_path())
            .map_err(|e| CliError::UnexpectedError(format!("Failed to save package: {}", e)))?;
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
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Artifacts to be generated when building this package.
    #[clap(long, default_value_t = IncludedArtifacts::Sparse)]
    pub(crate) included_artifacts: IncludedArtifacts,

    #[clap(flatten)]
    pub(crate) move_options: MovePackageDir,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[async_trait]
impl CliCommand<&'static str> for VerifyPackage {
    fn command_name(&self) -> &'static str {
        "DownloadPackage"
    }

    async fn execute(self) -> CliTypedResult<&'static str> {
        // First build the package locally to get the package metadata
        let build_options = BuildOptions {
            install_dir: self.move_options.output_dir.clone(),
            bytecode_version: self.move_options.bytecode_version,
            ..self.included_artifacts.build_options(
                self.move_options.skip_fetch_latest_git_deps,
                self.move_options.named_addresses(),
                self.move_options.bytecode_version,
            )
        };
        let pack = BuiltPackage::build(self.move_options.get_package_path()?, build_options)
            .map_err(|e| CliError::MoveCompilationError(format!("{:#}", e)))?;
        let compiled_metadata = pack.extract_metadata()?;

        // Now pull the compiled package
        let url = self.rest_options.url(&self.profile_options)?;
        let registry = CachedPackageRegistry::create(url, self.account).await?;
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
    #[clap(long, parse(try_from_str = crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Type of items to query
    ///
    /// Current supported types [packages]
    #[clap(long, default_value_t = MoveListQuery::Packages)]
    query: MoveListQuery,

    #[clap(flatten)]
    rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[derive(ArgEnum, Clone, Copy, Debug)]
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
        let registry = CachedPackageRegistry::create(url, self.account).await?;
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
    pub(crate) move_options: MovePackageDir,
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
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::message::set_message`
    #[clap(long)]
    pub(crate) function_id: MemberId,

    /// Arguments combined with their type separated by spaces.
    ///
    /// Supported types [u8, u16, u32, u64, u128, u256, bool, hex, string, address, raw, vector<inner_type>]
    ///
    /// Example: `address:0x1 bool:true u8:0 u256:1234 'vector<u32>:a,b,c,d'`
    #[clap(long, multiple_values = true)]
    pub(crate) args: Vec<ArgWithType>,

    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u16 u32 u64 u128 u256 bool address vector signer`
    #[clap(long, multiple_values = true)]
    pub(crate) type_args: Vec<MoveType>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<TransactionSummary> for RunFunction {
    fn command_name(&self) -> &'static str {
        "RunFunction"
    }

    async fn execute(self) -> CliTypedResult<TransactionSummary> {
        let args: Vec<Vec<u8>> = self
            .args
            .into_iter()
            .map(|arg_with_type| arg_with_type.arg)
            .collect();
        let mut type_args: Vec<TypeTag> = Vec::new();

        // These TypeArgs are used for generics
        for type_arg in self.type_args.into_iter() {
            let type_tag = TypeTag::try_from(type_arg)
                .map_err(|err| CliError::UnableToParse("--type-args", err.to_string()))?;
            type_args.push(type_tag)
        }

        self.txn_options
            .submit_transaction(TransactionPayload::EntryFunction(EntryFunction::new(
                self.function_id.module_id,
                self.function_id.member_id,
                type_args,
                args,
            )))
            .await
            .map(TransactionSummary::from)
    }
}

/// Run a Move function
#[derive(Parser)]
pub struct ViewFunction {
    /// Function name as `<ADDRESS>::<MODULE_ID>::<FUNCTION_NAME>`
    ///
    /// Example: `0x842ed41fad9640a2ad08fdd7d3e4f7f505319aac7d67e1c0dd6a7cce8732c7e3::message::set_message`
    #[clap(long)]
    pub(crate) function_id: MemberId,

    /// Arguments combined with their type separated by spaces.
    ///
    /// Supported types [u8, u16, u32, u64, u128, u256, bool, hex, string, address, raw, vector<inner_type>]
    ///
    /// Example: `address:0x1 bool:true u8:0 u256:1234 'vector<u32>:a,b,c,d'`
    #[clap(long, multiple_values = true)]
    pub(crate) args: Vec<ArgWithType>,

    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u16 u32 u64 u128 u256 bool address vector signer`
    #[clap(long, multiple_values = true)]
    pub(crate) type_args: Vec<MoveType>,

    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
}

#[async_trait]
impl CliCommand<Vec<serde_json::Value>> for ViewFunction {
    fn command_name(&self) -> &'static str {
        "RunViewFunction"
    }

    async fn execute(self) -> CliTypedResult<Vec<serde_json::Value>> {
        let mut args: Vec<serde_json::Value> = vec![];
        for arg in self.args {
            args.push(arg.to_json()?);
        }

        let view_request = ViewRequest {
            function: EntryFunctionId {
                module: self.function_id.module_id.into(),
                name: self.function_id.member_id.into(),
            },
            type_arguments: self.type_args,
            arguments: args,
        };

        self.txn_options.view(view_request).await
    }
}

/// Run a Move script
#[derive(Parser)]
pub struct RunScript {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    #[clap(flatten)]
    pub(crate) compile_proposal_args: CompileScriptFunction,
    /// Arguments combined with their type separated by spaces.
    ///
    /// Supported types [u8, u16, u32, u64, u128, u256, bool, hex, string, address, raw]
    ///
    /// Example: `address:0x1 bool:true u8:0 u256:1234`
    #[clap(long, multiple_values = true)]
    pub(crate) args: Vec<ArgWithType>,
    /// TypeTag arguments separated by spaces.
    ///
    /// Example: `u8 u16 u32 u64 u128 u256 bool address vector signer`
    #[clap(long, multiple_values = true)]
    pub(crate) type_args: Vec<MoveType>,
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

        let mut args: Vec<TransactionArgument> = vec![];
        for arg in self.args {
            args.push(arg.try_into()?);
        }

        let mut type_args: Vec<TypeTag> = Vec::new();

        // These TypeArgs are used for generics
        for type_arg in self.type_args.into_iter() {
            let type_tag = TypeTag::try_from(type_arg)
                .map_err(|err| CliError::UnableToParse("--type-args", err.to_string()))?;
            type_args.push(type_tag)
        }

        let txn = self
            .txn_options
            .submit_transaction(TransactionPayload::Script(Script::new(
                bytecode, type_args, args,
            )))
            .await?;
        Ok(TransactionSummary::from(&txn))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FunctionArgType {
    Address,
    Bool,
    Hex,
    HexArray,
    String,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Raw,
    Vector(Box<FunctionArgType>),
}

impl Display for FunctionArgType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionArgType::Address => write!(f, "address"),
            FunctionArgType::Bool => write!(f, "bool"),
            FunctionArgType::Hex => write!(f, "hex"),
            FunctionArgType::HexArray => write!(f, "hex_array"),
            FunctionArgType::String => write!(f, "string"),
            FunctionArgType::U8 => write!(f, "u8"),
            FunctionArgType::U16 => write!(f, "u16"),
            FunctionArgType::U32 => write!(f, "u32"),
            FunctionArgType::U64 => write!(f, "u64"),
            FunctionArgType::U128 => write!(f, "u128"),
            FunctionArgType::U256 => write!(f, "u256"),
            FunctionArgType::Raw => write!(f, "raw"),
            FunctionArgType::Vector(inner) => write!(f, "vector<{}>", inner),
        }
    }
}

impl FunctionArgType {
    fn parse_arg(&self, arg: &str) -> CliTypedResult<Vec<u8>> {
        match self {
            FunctionArgType::Address => bcs::to_bytes(
                &load_account_arg(arg)
                    .map_err(|err| CliError::UnableToParse("address", err.to_string()))?,
            ),
            FunctionArgType::Bool => bcs::to_bytes(
                &bool::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("bool", err.to_string()))?,
            ),
            FunctionArgType::Hex => bcs::to_bytes(
                &hex::decode(arg).map_err(|err| CliError::UnableToParse("hex", err.to_string()))?,
            ),
            FunctionArgType::HexArray => {
                let mut encoded = vec![];
                for sub_arg in arg.split(',') {
                    encoded.push(hex::decode(sub_arg).map_err(|err| {
                        CliError::UnableToParse(
                            "hex_array",
                            format!("Failed to parse hex array: {}", err),
                        )
                    })?);
                }
                bcs::to_bytes(&encoded)
            },
            FunctionArgType::String => bcs::to_bytes(arg),
            FunctionArgType::U8 => bcs::to_bytes(
                &u8::from_str(arg).map_err(|err| CliError::UnableToParse("u8", err.to_string()))?,
            ),
            FunctionArgType::U16 => bcs::to_bytes(
                &u16::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u16", err.to_string()))?,
            ),
            FunctionArgType::U32 => bcs::to_bytes(
                &u32::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u32", err.to_string()))?,
            ),
            FunctionArgType::U64 => bcs::to_bytes(
                &u64::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u64", err.to_string()))?,
            ),
            FunctionArgType::U128 => bcs::to_bytes(
                &u128::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u128", err.to_string()))?,
            ),
            FunctionArgType::U256 => bcs::to_bytes(
                &U256::from_str(arg)
                    .map_err(|err| CliError::UnableToParse("u256", err.to_string()))?,
            ),
            FunctionArgType::Raw => {
                let raw = hex::decode(arg)
                    .map_err(|err| CliError::UnableToParse("raw", err.to_string()))?;
                Ok(raw)
            },
            FunctionArgType::Vector(inner) => {
                let parsed = match inner.deref() {
                    FunctionArgType::Address => parse_vector_arg(arg, |arg| {
                        load_account_arg(arg).map_err(|err| {
                            CliError::UnableToParse("vector<address>", err.to_string())
                        })
                    }),
                    FunctionArgType::Bool => parse_vector_arg(arg, |arg| {
                        bool::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<bool>", err.to_string()))
                    }),
                    FunctionArgType::Hex => parse_vector_arg(arg, |arg| {
                        hex::decode(arg)
                            .map_err(|err| CliError::UnableToParse("vector<hex>", err.to_string()))
                    }),
                    FunctionArgType::U8 => parse_vector_arg(arg, |arg| {
                        u8::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<u8>", err.to_string()))
                    }),
                    FunctionArgType::U16 => parse_vector_arg(arg, |arg| {
                        u16::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<u16>", err.to_string()))
                    }),
                    FunctionArgType::U32 => parse_vector_arg(arg, |arg| {
                        u32::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<u32>", err.to_string()))
                    }),
                    FunctionArgType::U64 => parse_vector_arg(arg, |arg| {
                        u64::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<u64>", err.to_string()))
                    }),
                    FunctionArgType::U128 => parse_vector_arg(arg, |arg| {
                        u128::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<u128>", err.to_string()))
                    }),
                    FunctionArgType::U256 => parse_vector_arg(arg, |arg| {
                        U256::from_str(arg)
                            .map_err(|err| CliError::UnableToParse("vector<u256>", err.to_string()))
                    }),
                    vector_type => {
                        panic!("Unsupported vector type vector<{}>", vector_type)
                    },
                }?;
                Ok(parsed)
            },
        }
        .map_err(|err| CliError::BCS("arg", err))
    }
}

fn parse_vector_arg<T: Serialize, F: Fn(&str) -> CliTypedResult<T>>(
    args: &str,
    parse: F,
) -> CliTypedResult<Vec<u8>> {
    let mut parsed_args = vec![];
    let args = args.split(',');
    for arg in args {
        parsed_args.push(parse(arg)?);
    }

    bcs::to_bytes(&parsed_args).map_err(|err| CliError::BCS("arg", err))
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
            "hex_array" => Ok(FunctionArgType::HexArray),
            "raw" => Ok(FunctionArgType::Raw),
            str => {
                // If it's a vector, go one level inside
                if str.starts_with("vector<") && str.ends_with('>') {
                    let arg = FunctionArgType::from_str(&str[7..str.len() - 1])?;

                    // String gets confusing on parsing by commas
                    if arg == FunctionArgType::String {
                        return Err(CliError::CommandArgumentError(
                            "vector<string> is not supported".to_string(),
                        ));
                    } else if arg == FunctionArgType::Raw {
                        return Err(CliError::CommandArgumentError(
                            "vector<raw> is not supported".to_string(),
                        ));
                    } else if matches!(arg, FunctionArgType::Vector(_)) {
                        return Err(CliError::CommandArgumentError(
                            "nested vector<vector<_>> is not supported".to_string(),
                        ));
                    } else if arg == FunctionArgType::HexArray {
                        return Err(CliError::CommandArgumentError(
                            "nested vector<hex_array> is not supported".to_string(),
                        ));
                    }

                    Ok(FunctionArgType::Vector(Box::new(arg)))
                } else {
                    Err(CliError::CommandArgumentError(format!("Invalid arg type '{}'.  Must be one of: ['address','bool','hex','hex_array','string','u8','u16','u32','u64','u128','u256','raw', 'vector<inner_type>']", str)))
                }
            },
        }
    }
}

/// A parseable arg with a type separated by a colon
pub struct ArgWithType {
    pub(crate) _ty: FunctionArgType,
    pub(crate) arg: Vec<u8>,
}

impl ArgWithType {
    pub fn address(account_address: AccountAddress) -> Self {
        ArgWithType {
            _ty: FunctionArgType::Address,
            arg: bcs::to_bytes(&account_address).unwrap(),
        }
    }

    pub fn u64(arg: u64) -> Self {
        ArgWithType {
            _ty: FunctionArgType::U64,
            arg: bcs::to_bytes(&arg).unwrap(),
        }
    }

    pub fn bytes(arg: Vec<u8>) -> Self {
        ArgWithType {
            _ty: FunctionArgType::Raw,
            arg: bcs::to_bytes(&arg).unwrap(),
        }
    }

    pub fn raw(arg: Vec<u8>) -> Self {
        ArgWithType {
            _ty: FunctionArgType::Raw,
            arg,
        }
    }

    pub fn to_json(&self) -> CliTypedResult<serde_json::Value> {
        match self._ty.clone() {
            FunctionArgType::Address => {
                serde_json::to_value(bcs::from_bytes::<AccountAddress>(&self.arg)?)
            },
            FunctionArgType::Bool => serde_json::to_value(bcs::from_bytes::<bool>(&self.arg)?),
            FunctionArgType::Hex => serde_json::to_value(bcs::from_bytes::<Vec<u8>>(&self.arg)?),
            FunctionArgType::String => serde_json::to_value(bcs::from_bytes::<String>(&self.arg)?),
            FunctionArgType::U8 => serde_json::to_value(bcs::from_bytes::<u32>(&self.arg)?),
            FunctionArgType::U16 => serde_json::to_value(bcs::from_bytes::<u32>(&self.arg)?),
            FunctionArgType::U32 => serde_json::to_value(bcs::from_bytes::<u32>(&self.arg)?),
            FunctionArgType::U64 => {
                serde_json::to_value(bcs::from_bytes::<u64>(&self.arg)?.to_string())
            },
            FunctionArgType::U128 => {
                serde_json::to_value(bcs::from_bytes::<u128>(&self.arg)?.to_string())
            },
            FunctionArgType::U256 => {
                serde_json::to_value(bcs::from_bytes::<U256>(&self.arg)?.to_string())
            },
            FunctionArgType::Raw => serde_json::to_value(&self.arg),
            FunctionArgType::HexArray => {
                serde_json::to_value(bcs::from_bytes::<Vec<Vec<u8>>>(&self.arg)?)
            },
            FunctionArgType::Vector(inner) => match inner.deref() {
                FunctionArgType::Address => {
                    serde_json::to_value(bcs::from_bytes::<Vec<AccountAddress>>(&self.arg)?)
                },
                FunctionArgType::Bool => {
                    serde_json::to_value(bcs::from_bytes::<Vec<bool>>(&self.arg)?)
                },
                FunctionArgType::Hex => {
                    serde_json::to_value(bcs::from_bytes::<Vec<Vec<u8>>>(&self.arg)?)
                },
                FunctionArgType::String => {
                    serde_json::to_value(bcs::from_bytes::<Vec<String>>(&self.arg)?)
                },
                FunctionArgType::U8 => serde_json::to_value(bcs::from_bytes::<Vec<u8>>(&self.arg)?),
                FunctionArgType::U16 => {
                    serde_json::to_value(bcs::from_bytes::<Vec<u16>>(&self.arg)?)
                },
                FunctionArgType::U32 => {
                    serde_json::to_value(bcs::from_bytes::<Vec<u32>>(&self.arg)?)
                },
                FunctionArgType::U64 => {
                    serde_json::to_value(bcs::from_bytes::<Vec<u64>>(&self.arg)?)
                },
                FunctionArgType::U128 => {
                    serde_json::to_value(bcs::from_bytes::<Vec<u128>>(&self.arg)?)
                },
                FunctionArgType::U256 => {
                    serde_json::to_value(bcs::from_bytes::<Vec<U256>>(&self.arg)?)
                },
                FunctionArgType::Raw | FunctionArgType::HexArray | FunctionArgType::Vector(_) => {
                    return Err(CliError::UnexpectedError(
                        "Nested vectors not supported".to_string(),
                    ));
                },
            },
        }
        .map_err(|err| {
            CliError::UnexpectedError(format!("Failed to parse argument to JSON {}", err))
        })
    }
}

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
        let arg = parts.last().unwrap();
        let arg = ty.parse_arg(arg)?;

        Ok(ArgWithType { _ty: ty, arg })
    }
}

impl TryInto<TransactionArgument> for ArgWithType {
    type Error = CliError;

    fn try_into(self) -> Result<TransactionArgument, Self::Error> {
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
            FunctionArgType::HexArray => Ok(TransactionArgument::U8Vector(txn_arg_parser(
                &self.arg,
                "hex_array",
            )?)),
            FunctionArgType::String => Ok(TransactionArgument::U8Vector(txn_arg_parser(
                &self.arg, "string",
            )?)),
            FunctionArgType::U8 => Ok(TransactionArgument::U8(txn_arg_parser(&self.arg, "u8")?)),
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
            arg_type => Err(CliError::CommandArgumentError(format!(
                "Input type {} not supported",
                arg_type
            ))),
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
