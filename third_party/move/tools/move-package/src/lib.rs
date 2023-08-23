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
use anyhow::{bail, Result};
use clap::*;
use move_compiler::{
    command_line::SKIP_ATTRIBUTE_CHECKS, shared::known_attributes::KnownAttribute,
};
use move_core_types::account_address::AccountAddress;
use move_model::model::GlobalEnv;
use serde::{Deserialize, Serialize};
use source_package::layout::SourcePackageLayout;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Architecture {
    Move,

    AsyncMove,

    Ethereum,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Move => write!(f, "move"),

            Self::AsyncMove => write!(f, "async-move"),

            Self::Ethereum => write!(f, "ethereum"),
        }
    }
}

impl Architecture {
    fn all() -> impl Iterator<Item = Self> {
        IntoIterator::into_iter([
            Self::Move,
            Self::AsyncMove,
            #[cfg(feature = "evm-backend")]
            Self::Ethereum,
        ])
    }

    fn try_parse_from_str(s: &str) -> Result<Self> {
        Ok(match s {
            "move" => Self::Move,

            "async-move" => Self::AsyncMove,

            "ethereum" => Self::Ethereum,

            _ => {
                let supported_architectures = Self::all()
                    .map(|arch| format!("\"{}\"", arch))
                    .collect::<Vec<_>>();
                let be = if supported_architectures.len() == 1 {
                    "is"
                } else {
                    "are"
                };
                bail!(
                    "Unrecognized architecture {} -- only {} {} supported",
                    s,
                    supported_architectures.join(", "),
                    be
                )
            },
        })
    }
}

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

    /// Generate documentation for packages
    #[clap(name = "generate-docs", long = "doc", global = true)]
    pub generate_docs: bool,

    /// Generate ABIs for packages
    #[clap(name = "generate-abis", long = "abi", global = true)]
    pub generate_abis: bool,

    /// Installation directory for compiled artifacts. Defaults to current directory.
    #[clap(long = "install-dir", value_parser, global = true)]
    pub install_dir: Option<PathBuf>,

    /// Force recompilation of all packages
    #[clap(name = "force-recompilation", long = "force", global = true)]
    pub force_recompilation: bool,

    /// Additional named address mapping. Useful for tools in rust
    #[clap(skip)]
    pub additional_named_addresses: BTreeMap<String, AccountAddress>,

    #[clap(long = "arch", global = true, value_parser = Architecture::try_parse_from_str)]
    pub architecture: Option<Architecture>,

    /// Only fetch dependency repos to MOVE_HOME
    #[clap(long = "fetch-deps-only", global = true)]
    pub fetch_deps_only: bool,

    /// Skip fetching latest git dependencies
    #[clap(long = "skip-fetch-latest-git-deps", global = true)]
    pub skip_fetch_latest_git_deps: bool,

    #[clap(flatten)]
    pub compiler_config: CompilerConfig,
}

#[derive(Parser, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd, Default, Debug)]
pub struct CompilerConfig {
    /// Bytecode version to compile move code
    #[clap(long = "bytecode-version", global = true)]
    pub bytecode_version: Option<u32>,

    // Known attribute names.  Depends on compilation context (Move variant)
    #[clap(skip = KnownAttribute::get_all_attribute_names().clone())]
    pub known_attributes: BTreeSet<String>,

    /// Do not complain about an unknown attribute in Move code.
    #[clap(long = SKIP_ATTRIBUTE_CHECKS, default_value = "false")]
    pub skip_attribute_checks: bool,

    /// Compiler version to use
    #[clap(long = "compiler-version", global = true)]
    pub compiler_version: Option<CompilerVersion>,
}

#[derive(ValueEnum, Clone, Copy, Debug, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
pub enum CompilerVersion {
    V1,
    V2,
}

impl Default for CompilerVersion {
    fn default() -> Self {
        Self::V1
    }
}

#[derive(Debug, Clone, Eq, PartialEq, PartialOrd)]
pub struct ModelConfig {
    /// If set, also files which are in dependent packages are considered as targets.
    pub all_files_as_targets: bool,
    /// If set, a string how targets are filtered. A target is included if its file name
    /// contains this string. This is similar as the `cargo test <string>` idiom.
    pub target_filter: Option<String>,
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
    pub fn compile_package_no_exit<W: Write>(
        self,
        path: &Path,
        writer: &mut W,
    ) -> Result<CompiledPackage> {
        let config = self.compiler_config.clone(); // Need clone because of mut self
        let resolved_graph = self.resolution_graph_for_package(path, writer)?;
        let mutx = PackageLock::lock();
        let ret = BuildPlan::create(resolved_graph)?.compile_no_exit(&config, writer);
        mutx.unlock();
        ret
    }

    #[cfg(feature = "evm-backend")]
    pub fn compile_package_evm<W: Write>(self, path: &Path, writer: &mut W) -> Result<()> {
        // resolution graph diagnostics are only needed for CLI commands so ignore them by passing a
        // vector as the writer
        let resolved_graph = self.resolution_graph_for_package(path, &mut Vec::new())?;
        let mutx = PackageLock::lock();
        let ret = BuildPlan::create(resolved_graph)?.compile_evm(writer);
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
    ) -> Result<GlobalEnv> {
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
        let mutx = PackageLock::lock();
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
