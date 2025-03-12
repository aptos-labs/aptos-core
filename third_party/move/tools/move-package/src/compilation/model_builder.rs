// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compilation::compiled_package::make_source_and_deps_for_compiler,
    resolution::resolution_graph::ResolvedGraph, CompilerVersion, ModelConfig,
};
use anyhow::Result;
use itertools::Itertools;
use move_compiler::shared::PackagePaths;
use move_compiler_v2::Options;
use move_model::model::GlobalEnv;
use termcolor::{ColorChoice, StandardStream};

#[derive(Debug, Clone)]
pub struct ModelBuilder {
    resolution_graph: ResolvedGraph,
    model_config: ModelConfig,
}

impl ModelBuilder {
    pub fn create(resolution_graph: ResolvedGraph, model_config: ModelConfig) -> Self {
        Self {
            resolution_graph,
            model_config,
        }
    }

    // NOTE: If there are now renamings, then the root package has the global resolution of all named
    // addresses in the package graph in scope. So we can simply grab all of the source files
    // across all packages and build the Move model from that.
    // TODO: In the future we will need a better way to do this to support renaming in packages
    // where we want to support building a Move model.
    pub fn build_model(&self) -> Result<GlobalEnv> {
        // Make sure no renamings have been performed
        if let Some(pkg_name) = self.resolution_graph.contains_renaming() {
            anyhow::bail!(
                "Found address renaming in package '{}' when \
                    building Move model -- this is currently not supported",
                pkg_name
            )
        }

        // Targets are all files in the root package
        let root_name = &self.resolution_graph.root_package.package.name;
        let root_package = self.resolution_graph.get_package(root_name).clone();
        let deps_source_info = self
            .resolution_graph
            .package_table
            .iter()
            .filter_map(|(nm, pkg)| {
                if nm == root_name {
                    return None;
                }
                let mut dep_source_paths = pkg
                    .get_sources(&self.resolution_graph.build_options)
                    .unwrap();
                let mut source_available = true;
                // If source is empty, search bytecode
                if dep_source_paths.is_empty() {
                    dep_source_paths = pkg.get_bytecodes().unwrap();
                    source_available = false;
                }
                Some(Ok((
                    *nm,
                    dep_source_paths,
                    &pkg.resolution_table,
                    source_available,
                )))
            })
            .collect::<Result<Vec<_>>>()?;
        let (target, deps) = make_source_and_deps_for_compiler(
            &self.resolution_graph,
            &root_package,
            deps_source_info,
        )?;
        let (all_targets, all_deps) = if self.model_config.all_files_as_targets {
            let mut targets = vec![target];
            targets.extend(deps.into_iter().map(|(p, _)| p).collect_vec());
            (targets, vec![])
        } else {
            (vec![target], deps)
        };
        let (all_targets, all_deps) = match &self.model_config.target_filter {
            Some(filter) => {
                let mut new_targets = vec![];
                let mut new_deps = all_deps.into_iter().map(|(p, _)| p).collect_vec();
                for PackagePaths {
                    name,
                    paths,
                    named_address_map,
                } in all_targets
                {
                    let (true_targets, false_targets): (Vec<_>, Vec<_>) =
                        paths.into_iter().partition(|t| t.contains(filter));
                    if !true_targets.is_empty() {
                        new_targets.push(PackagePaths {
                            name,
                            paths: true_targets,
                            named_address_map: named_address_map.clone(),
                        })
                    }
                    if !false_targets.is_empty() {
                        new_deps.push(PackagePaths {
                            name,
                            paths: false_targets,
                            named_address_map,
                        })
                    }
                }
                (new_targets, new_deps)
            },
            None => (
                all_targets,
                all_deps.into_iter().map(|(p, _)| p).collect_vec(),
            ),
        };

        let skip_attribute_checks = self
            .resolution_graph
            .build_options
            .compiler_config
            .skip_attribute_checks;
        let known_attributes = &self
            .resolution_graph
            .build_options
            .compiler_config
            .known_attributes;
        match self.model_config.compiler_version {
            CompilerVersion::V1 => anyhow::bail!("Compiler v1 is no longer supported"),
            CompilerVersion::V2_0 | CompilerVersion::V2_1 => {
                let mut options = make_options_for_v2_compiler(all_targets, all_deps);
                options.language_version = self
                    .resolution_graph
                    .build_options
                    .compiler_config
                    .language_version;
                options.compiler_version = Some(self.model_config.compiler_version);
                options.known_attributes.clone_from(known_attributes);
                options.skip_attribute_checks = skip_attribute_checks;
                options.compile_verify_code = true;
                let mut error_writer = StandardStream::stderr(ColorChoice::Auto);
                move_compiler_v2::run_move_compiler_for_analysis(&mut error_writer, options)
            },
        }
    }
}

fn make_options_for_v2_compiler(targets: Vec<PackagePaths>, deps: Vec<PackagePaths>) -> Options {
    let mut options = Options {
        sources: targets
            .iter()
            .flat_map(|p| p.paths.iter().map(|s| s.to_string()).collect_vec())
            .collect(),
        ..Options::default()
    };
    options.dependencies = deps
        .iter()
        .flat_map(|p| p.paths.iter().map(|s| s.to_string()).collect_vec())
        .collect();
    options.named_address_mapping = targets
        .into_iter()
        .chain(deps)
        .flat_map(|p| {
            p.named_address_map
                .iter()
                .map(|(n, a)| format!("{}={}", n, a.into_inner()))
                .collect_vec()
        })
        .collect_vec();
    options
}
