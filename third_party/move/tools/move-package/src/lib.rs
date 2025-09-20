// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

mod package_lock;

pub mod compilation;
pub mod package_hooks;
pub mod resolution;
pub mod source_package;

use crate::{
    compilation::{
        build_plan::BuildPlan, compiled_package::CompiledPackage, model_builder::ModelBuilder,
    },
    package_lock::PackageLock,
    resolution::resolution_graph::{ResolutionGraph, ResolvedGraph},
    source_package::manifest_parser,
};
use anyhow::Result;
use clap::*;
use legacy_move_compiler::{
    command_line::SKIP_ATTRIBUTE_CHECKS, shared::known_attributes::KnownAttribute,
};
use move_compiler_v2::external_checks::ExternalChecks;
use move_core_types::account_address::AccountAddress;
use move_model::{
    metadata::{CompilerVersion, LanguageVersion},
    model,
};
use serde::{Deserialize, Serialize};
use source_package::{layout::SourcePackageLayout, std_lib::StdVersion};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Parser, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Default)]
#[clap(author, version, about)]
pub struct BuildConfig {
    /// Compile in 'dev' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used if
    /// this flag is set. This flag is useful for development of packages that expose named
    /// addresses that are not set to a specific value.
    #[clap(name = "dev-mode", short = 'd', long = "dev", global = true)]
    pub dev_mode: bool,

    /// Compile in 'test' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used
    /// along with any code in the 'tests' directory.
    #[clap(name = "test-mode", long = "test", global = true)]
    pub test_mode: bool,

    /// Whether to override the standard library with the given version.
    #[clap(long = "override-std", global = true, value_parser)]
    pub override_std: Option<StdVersion>,

    /// Generate documentation for packages
    #[clap(name = "generate-docs", long = "doc", global = true)]
    pub generate_docs: bool,

    /// Generate ABIs for packages
    #[clap(name = "generate-abis", long = "abi", global = true)]
    pub generate_abis: bool,

    /// Whether to generate a move model. Used programmatically only.
    #[clap(skip)]
    pub generate_move_model: bool,

    /// Whether the generated model shall contain all functions, including test-only ones.
    #[clap(skip)]
    pub full_model_generation: bool,

    /// Installation directory for compiled artifacts. Defaults to current directory.
    #[clap(long = "install-dir", value_parser, global = true)]
    pub install_dir: Option<PathBuf>,

    /// Force recompilation of all packages
    #[clap(name = "force-recompilation", long = "force", global = true)]
    pub force_recompilation: bool,

    /// Additional named address mapping. Useful for tools in rust
    #[clap(skip)]
    pub additional_named_addresses: BTreeMap<String, AccountAddress>,

    /// Only fetch dependency repos to MOVE_HOME
    #[clap(long = "fetch-deps-only", global = true)]
    pub fetch_deps_only: bool,

    /// Skip fetching latest git dependencies
    #[clap(long = "skip-fetch-latest-git-deps", global = true)]
    pub skip_fetch_latest_git_deps: bool,

    /// Skip downloading git tree for git dependencies
    #[clap(long = "skip-download-git-tree", global = false)]
    pub skip_download_git_tree: bool,

    #[clap(flatten)]
    pub compiler_config: CompilerConfig,
}

#[derive(Parser, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Default, Debug)]
pub struct CompilerConfig {
    /// Bytecode version to compile move code
    #[clap(long, global = true)]
    pub bytecode_version: Option<u32>,

    // Known attribute names.  Depends on compilation context (Move variant)
    #[clap(skip = KnownAttribute::get_all_attribute_names().clone())]
    pub known_attributes: BTreeSet<String>,

    /// Do not complain about an unknown attribute in Move code.
    #[clap(long = SKIP_ATTRIBUTE_CHECKS, default_value = "false")]
    pub skip_attribute_checks: bool,

    /// Compiler version to use
    #[clap(long, global = true, value_parser = clap::value_parser!(CompilerVersion))]
    pub compiler_version: Option<CompilerVersion>,

    /// Language version to support
    #[clap(long, global = true, value_parser = clap::value_parser!(LanguageVersion))]
    pub language_version: Option<LanguageVersion>,

    /// Experiments for v2 compiler to set to true
    #[clap(long, global = true)]
    pub experiments: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub struct ModelConfig {
    /// If set, also files which are in dependent packages are considered as targets.
    pub all_files_as_targets: bool,
    /// If set, a string how targets are filtered. A target is included if its file name
    /// contains this string. This is similar as the `cargo test <string>` idiom.
    pub target_filter: Option<String>,
    /// The compiler version used to build the model
    pub compiler_version: CompilerVersion,
    /// The language version used to build the model
    pub language_version: LanguageVersion,
}

impl BuildConfig {
    /// Compile the package at `path` or the containing Move package. Exit process on warning or
    /// failure.
    pub fn compile_package<W: Write>(self, path: &Path, writer: &mut W) -> Result<CompiledPackage> {
        let config = self.compiler_config.clone(); // Need clone because of mut self
        let resolved_graph = self.resolution_graph_for_package(path, writer)?;
        let mutx = PackageLock::lock();
        let ret = BuildPlan::create(resolved_graph)?.compile(&config, writer);
        mutx.unlock();
        ret
    }

    /// Compile the package at `path` or the containing Move package. Do not exit process on warning
    /// or failure.
    /// External checks on Move code can be provided, these are only run if compiler v2 is used.
    pub fn compile_package_no_exit<W: Write>(
        self,
        resolved_graph: ResolvedGraph,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        writer: &mut W,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        let config = self.compiler_config.clone(); // Need clone because of mut self
        let mutx = PackageLock::lock();
        let ret =
            BuildPlan::create(resolved_graph)?.compile_no_exit(&config, external_checks, writer);
        mutx.unlock();
        ret
    }

    // NOTE: If there are no renamings, then the root package has the global resolution of all named
    // addresses in the package graph in scope. So we can simply grab all of the source files
    // across all packages and build the Move model from that.
    // TODO: In the future we will need a better way to do this to support renaming in packages
    // where we want to support building a Move model.
    pub fn move_model_for_package(
        self,
        path: &Path,
        model_config: ModelConfig,
    ) -> Result<model::GlobalEnv> {
        // resolution graph diagnostics are only needed for CLI commands so ignore them by passing a
        // vector as the writer
        let resolved_graph = self.resolution_graph_for_package(path, &mut Vec::new())?;
        let mutx = PackageLock::lock();
        let ret = ModelBuilder::create(resolved_graph, model_config).build_model();
        mutx.unlock();
        ret
    }

    pub fn download_deps_for_package<W: Write>(&self, path: &Path, writer: &mut W) -> Result<()> {
        let path = SourcePackageLayout::try_find_root(path)?;
        let toml_manifest =
            self.parse_toml_manifest(path.join(SourcePackageLayout::Manifest.path()))?;
        let mutx = PackageLock::strict_lock();
        // This should be locked as it inspects the environment for `MOVE_HOME` which could
        // possibly be set by a different process in parallel.
        let manifest = manifest_parser::parse_source_manifest(toml_manifest)?;
        ResolutionGraph::download_dependency_repos(&manifest, self, &path, writer)?;
        mutx.unlock();
        Ok(())
    }

    pub fn resolution_graph_for_package<W: Write>(
        mut self,
        path: &Path,
        writer: &mut W,
    ) -> Result<ResolvedGraph> {
        if self.test_mode {
            self.dev_mode = true;
        }
        let path = SourcePackageLayout::try_find_root(path)?;
        let toml_manifest =
            self.parse_toml_manifest(path.join(SourcePackageLayout::Manifest.path()))?;
        let mutx = PackageLock::lock();
        // This should be locked as it inspects the environment for `MOVE_HOME` which could
        // possibly be set by a different process in parallel.
        let manifest = manifest_parser::parse_source_manifest(toml_manifest)?;
        let resolution_graph = ResolutionGraph::new(manifest, path, self, writer)?;
        let ret = resolution_graph.resolve();
        mutx.unlock();
        ret
    }

    fn parse_toml_manifest(&self, path: PathBuf) -> Result<toml::Value> {
        let manifest_string = std::fs::read_to_string(path)?;
        manifest_parser::parse_move_manifest_string(manifest_string)
    }
}
