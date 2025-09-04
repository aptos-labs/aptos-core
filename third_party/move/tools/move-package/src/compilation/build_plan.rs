// Copyright (c) Velor Foundation
// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::package_layout::CompiledPackageLayout;
use crate::{
    compilation::compiled_package::{
        build_and_report_no_exit_v2_driver, build_and_report_v2_driver, CompiledPackage,
    },
    resolution::resolution_graph::ResolvedGraph,
    source_package::parsed_manifest::PackageName,
    CompilerConfig,
};
use anyhow::{Context, Result};
use legacy_move_compiler::{compiled_unit::AnnotatedCompiledUnit, diagnostics::FilesSourceText};
use move_compiler_v2::external_checks::ExternalChecks;
use move_model::model;
use petgraph::algo::toposort;
use std::{collections::BTreeSet, io::Write, path::Path, sync::Arc};

#[derive(Debug, Clone)]
pub struct BuildPlan {
    root: PackageName,
    sorted_deps: Vec<PackageName>,
    resolution_graph: ResolvedGraph,
}

/// A container for compiler results from either V1 or V2,
/// with all info needed for building various artifacts.
pub type CompilerDriverResult = anyhow::Result<(
    // The names and contents of all source files.
    FilesSourceText,
    // The compilation artifacts, including V1 intermediate ASTs.
    Vec<AnnotatedCompiledUnit>,
    // For compilation with V2, compiled program model.
    model::GlobalEnv,
)>;

impl BuildPlan {
    pub fn create(resolution_graph: ResolvedGraph) -> Result<Self> {
        let mut sorted_deps = match toposort(&resolution_graph.graph, None) {
            Ok(nodes) => nodes,
            Err(err) => {
                // Is a DAG after resolution otherwise an error should be raised from that.
                anyhow::bail!("IPE: Cyclic dependency found after resolution {:?}", err)
            },
        };

        sorted_deps.reverse();

        Ok(Self {
            root: resolution_graph.root_package.package.name,
            sorted_deps,
            resolution_graph,
        })
    }

    /// Compilation results in the process exit upon warning/failure
    pub fn compile<W: Write>(
        &self,
        config: &CompilerConfig,
        writer: &mut W,
    ) -> Result<CompiledPackage> {
        self.compile_with_driver(writer, config, vec![], build_and_report_v2_driver)
            .map(|(package, _)| package)
    }

    /// Compilation process does not exit even if warnings/failures are encountered.
    /// External checks on Move code can be provided via `external_checks`.
    pub fn compile_no_exit<W: Write>(
        &self,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        writer: &mut W,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        self.compile_with_driver(
            writer,
            config,
            external_checks,
            build_and_report_no_exit_v2_driver,
        )
    }

    pub fn compile_with_driver<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        let root_package = &self.resolution_graph.package_table[&self.root];
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let immediate_dependencies_names =
            root_package.immediate_dependencies(&self.resolution_graph);
        let transitive_dependencies = root_package
            .transitive_dependencies(&self.resolution_graph)
            .into_iter()
            .map(|package_name| {
                let dep_package = self
                    .resolution_graph
                    .package_table
                    .get(&package_name)
                    .unwrap();
                let mut dep_source_paths = dep_package
                    .get_sources(&self.resolution_graph.build_options)
                    .unwrap();
                let mut source_available = true;
                // If source is empty, search bytecode(mv) files
                if dep_source_paths.is_empty() {
                    dep_source_paths = dep_package.get_bytecodes().unwrap();
                    source_available = false;
                }
                (
                    package_name,
                    immediate_dependencies_names.contains(&package_name),
                    dep_source_paths,
                    &dep_package.resolution_table,
                    source_available,
                )
            })
            .collect();

        let (compiled, model) = CompiledPackage::build_all(
            writer,
            &project_root,
            root_package.clone(),
            transitive_dependencies,
            config,
            external_checks,
            &self.resolution_graph,
            driver,
        )?;

        Self::clean(
            &project_root.join(CompiledPackageLayout::Root.path()),
            self.sorted_deps.iter().copied().collect(),
        )?;
        Ok((compiled, model))
    }

    // Clean out old packages that are no longer used, or no longer used under the current
    // compilation flags
    fn clean(build_root: &Path, keep_paths: BTreeSet<PackageName>) -> Result<()> {
        for dir in std::fs::read_dir(build_root)? {
            let path = dir
                .with_context(|| {
                    format!(
                        "Cleaning subdirectories of build root {}",
                        build_root.to_string_lossy()
                    )
                })?
                .path();
            if path.is_dir() && !keep_paths.iter().any(|name| path.ends_with(name.as_str())) {
                std::fs::remove_dir_all(&path).with_context(|| {
                    format!("When deleting directory {}", path.to_string_lossy())
                })?;
            }
        }
        Ok(())
    }
}
