// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compilation::compiled_package::CompiledPackage, resolution::resolution_graph::ResolvedGraph,
    source_package::parsed_manifest::PackageName,
};
use anyhow::Result;
use move_lang::{compiled_unit::AnnotatedCompiledUnit, diagnostics::FilesSourceText, Compiler};
use petgraph::algo::toposort;
use std::{collections::BTreeMap, io::Write};

use super::compiled_package::CompilationCachingStatus;

#[derive(Debug, Clone)]
pub struct BuildPlan {
    root: PackageName,
    sorted_deps: Vec<PackageName>,
    resolution_graph: ResolvedGraph,
}

impl BuildPlan {
    pub fn create(resolution_graph: ResolvedGraph) -> Result<Self> {
        let mut sorted_deps = match toposort(&resolution_graph.graph, None) {
            Ok(nodes) => nodes,
            Err(err) => {
                // Is a DAG after resolution otherwise an error should be raised from that.
                anyhow::bail!("IPE: Cyclic dependency found after resolution {:?}", err)
            }
        };

        sorted_deps.reverse();

        Ok(Self {
            root: resolution_graph.root_package.package.name,
            sorted_deps,
            resolution_graph,
        })
    }

    pub fn compile<W: Write>(
        &self,
        writer: &mut W,
    ) -> Result<(CompiledPackage, CompilationCachingStatus)> {
        self.compile_with_driver(writer, |compiler, _| compiler.build_and_report())
    }

    pub fn compile_with_driver<W: Write>(
        &self,
        writer: &mut W,
        mut compiler_driver: impl FnMut(
            Compiler,
            bool,
        )
            -> anyhow::Result<(FilesSourceText, Vec<AnnotatedCompiledUnit>)>,
    ) -> Result<(CompiledPackage, CompilationCachingStatus)> {
        let package_root = &self.resolution_graph.package_table[&self.root];
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let mut compiled: BTreeMap<PackageName, (CompiledPackage, CompilationCachingStatus)> =
            BTreeMap::new();
        for package_ident in &self.sorted_deps {
            let resolved_package = self.resolution_graph.get_package(package_ident);
            let dependencies: Vec<_> = resolved_package
                .transitive_dependencies(&self.resolution_graph)
                .into_iter()
                .map(|package_name| compiled.get(&package_name).unwrap().clone())
                .collect();
            let compiled_package = CompiledPackage::build(
                writer,
                &project_root,
                resolved_package.clone(),
                dependencies,
                &self.resolution_graph,
                package_ident == &package_root.source_package.package.name,
                &mut compiler_driver,
            )?;
            compiled.insert(*package_ident, compiled_package);
        }
        Ok(compiled
            .remove(&package_root.source_package.package.name)
            .unwrap())
    }
}
