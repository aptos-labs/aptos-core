// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod compilation;
pub mod resolution;
pub mod source_package;

use anyhow::Result;
use compilation::compiled_package::CompilationCachingStatus;
use move_core_types::account_address::AccountAddress;
use move_model::model::GlobalEnv;
use serde::{Deserialize, Serialize};
use source_package::layout::SourcePackageLayout;
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};
use structopt::*;

use crate::{
    compilation::{
        build_plan::BuildPlan, compiled_package::CompiledPackage, model_builder::ModelBuilder,
    },
    resolution::resolution_graph::{ResolutionGraph, ResolvedGraph},
    source_package::{layout, manifest_parser},
};

#[derive(Debug, StructOpt, Clone, Serialize, Deserialize, Eq, PartialEq, PartialOrd)]
#[structopt(
    name = "Move Package",
    about = "Package and build system for Move code."
)]
pub struct BuildConfig {
    /// Compile in 'dev' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used if
    /// this flag is set. This flag is useful for development of packages that expose named
    /// addresses that are not set to a specific value.
    #[structopt(name = "dev-mode", short = "d", long = "dev", global = true)]
    pub dev_mode: bool,

    /// Compile in 'test' mode. The 'dev-addresses' and 'dev-dependencies' fields will be used
    /// along with any code in the 'test' directory.
    #[structopt(name = "test-mode", long = "test", global = true)]
    pub test_mode: bool,

    /// Generate documentation for packages
    #[structopt(name = "generate-docs", long = "doc", global = true)]
    pub generate_docs: bool,

    /// Generate ABIs for packages
    #[structopt(name = "generate-abis", long = "abi", global = true)]
    pub generate_abis: bool,

    /// Installation directory for compiled artifacts. Defaults to current directory.
    #[structopt(long = "install-dir", parse(from_os_str), global = true)]
    pub install_dir: Option<PathBuf>,

    /// Force recompilation of all packages
    #[structopt(name = "force-recompilation", long = "force", global = true)]
    pub force_recompilation: bool,

    /// Additional named address mapping. Useful for tools in rust
    #[structopt(skip)]
    pub additional_named_addresses: BTreeMap<String, AccountAddress>,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            dev_mode: false,
            test_mode: false,
            generate_docs: false,
            generate_abis: false,
            install_dir: None,
            force_recompilation: false,
            additional_named_addresses: BTreeMap::new(),
        }
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
    /// Compile the package at `path` or the containing Move package.
    pub fn compile_package<W: Write>(self, path: &Path, writer: &mut W) -> Result<CompiledPackage> {
        Ok(self.compile_package_with_caching_info(path, writer)?.0)
    }

    /// Compile the package at `path` or the containing Move package and return whether or not all
    /// packages and dependencies were cached or not.
    pub fn compile_package_with_caching_info<W: Write>(
        self,
        path: &Path,
        writer: &mut W,
    ) -> Result<(CompiledPackage, CompilationCachingStatus)> {
        let resolved_graph = self.resolution_graph_for_package(path)?;
        BuildPlan::create(resolved_graph)?.compile(writer)
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
        let resolved_graph = self.resolution_graph_for_package(path)?;
        ModelBuilder::create(resolved_graph, model_config).build_model()
    }

    pub fn resolution_graph_for_package(mut self, path: &Path) -> Result<ResolvedGraph> {
        if self.test_mode {
            self.dev_mode = true;
        }
        let path = SourcePackageLayout::try_find_root(path)?;
        let manifest_string =
            std::fs::read_to_string(path.join(layout::SourcePackageLayout::Manifest.path()))?;
        let toml_manifest = manifest_parser::parse_move_manifest_string(manifest_string)?;
        let manifest = manifest_parser::parse_source_manifest(toml_manifest)?;
        let resolution_graph = ResolutionGraph::new(manifest, path, self)?;
        resolution_graph.resolve()
    }
}
