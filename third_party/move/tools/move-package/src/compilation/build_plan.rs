// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::package_layout::CompiledPackageLayout;
use crate::{
    compilation::compiled_package::{
        build_and_report_no_exit_v2_driver, build_and_report_v2_driver, get_module_in_package,
        CompiledPackage, OnDiskCompiledPackage,
    },
    resolution::resolution_graph::{ResolvedGraph, ResolvedPackage},
    source_package::parsed_manifest::PackageName,
    CompilerConfig,
};
use anyhow::{Context, Result};
use colored::Colorize;
use legacy_move_compiler::{compiled_unit::AnnotatedCompiledUnit, diagnostics::FilesSourceText};
use move_command_line_common::files::{extension_equals, find_filenames};
use move_compiler_v2::external_checks::ExternalChecks;
use move_model::model;
use petgraph::algo::toposort;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::Path,
    sync::Arc,
};

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
        self.compile_with_driver(
            writer,
            config,
            vec![],
            /*model_required*/ false,
            build_and_report_v2_driver,
        )
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
            /*model_required*/ true,
            build_and_report_no_exit_v2_driver,
        )
    }

    /// The source dependencies of `package`, in the shape `build_all` expects.
    fn source_dependencies_of(
        &self,
        package: &crate::resolution::resolution_graph::ResolvedPackage,
    ) -> Vec<(
        PackageName,
        bool,
        Vec<move_symbol_pool::Symbol>,
        &crate::resolution::resolution_graph::ResolvedTable,
        bool,
    )> {
        let immediate = package.immediate_dependencies(&self.resolution_graph);
        package
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
                    immediate.contains(&package_name),
                    dep_source_paths,
                    &dep_package.resolution_table,
                    source_available,
                )
            })
            .collect()
    }

    /// `model_required` says whether the caller uses the returned
    /// [`model::GlobalEnv`]. It matters only under modular compilation: a
    /// package served from cache is never compiled, so it produces no model,
    /// and the root has to be built even when unchanged for one to exist. A
    /// caller that discards the model should pass `false` and keep the faster
    /// path.
    pub fn compile_with_driver<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        model_required: bool,
        driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        if self.resolution_graph.build_options.modular_compilation {
            return self.compile_modular(writer, config, external_checks, model_required, driver);
        }
        let root_package = &self.resolution_graph.package_table[&self.root];
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let transitive_dependencies = self.source_dependencies_of(root_package);

        let (compiled, model) = CompiledPackage::build_all(
            writer,
            &project_root,
            root_package.clone(),
            transitive_dependencies,
            config,
            external_checks,
            &self.resolution_graph,
            // The monolithic path compiles every dependency's source in the
            // same invocation, so it needs no interfaces.
            vec![],
            driver,
        )?;

        Self::clean(
            &project_root.join(CompiledPackageLayout::Root.path()),
            self.sorted_deps.iter().copied().collect(),
        )?;
        Ok((compiled, model))
    }

    /// Compiles each package on its own, in dependency order, against its
    /// dependencies' XIR interfaces rather than their sources.
    ///
    /// A package is compiled against its dependencies' sources instead, as the
    /// monolithic path would, in two cases. Some dependency published no
    /// interface, because it has no sources or its export failed. Or compiling
    /// against interfaces failed, which prints a `FALLBACK` line: either the
    /// package has an error, which the source build reports again, or an
    /// interface is wrong. Retrying needs a driver that returns errors;
    /// `build_and_report_v2_driver` exits the process instead.
    ///
    /// Either way the package is cached, keyed on what it was compiled against.
    fn compile_modular<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        model_required: bool,
        mut driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let build_root = project_root.join(CompiledPackageLayout::Root.path());
        let dependency_cache_root = self
            .resolution_graph
            .build_options
            .dependency_cache_dir
            .as_ref()
            .map(|dir| dir.join(CompiledPackageLayout::Root.path()));
        let bytecode_version = config
            .language_version
            .unwrap_or_default()
            .infer_bytecode_version(config.bytecode_version);

        let mut interfaces: BTreeMap<PackageName, Vec<String>> = BTreeMap::new();
        let mut built: BTreeMap<PackageName, CompiledPackage> = BTreeMap::new();
        // Only the root's reuse matters below; a dependency's is reported and
        // then forgotten.
        let mut root_reused = false;
        let mut root_model = None;

        for package_name in &self.sorted_deps {
            let package = self.resolution_graph.package_table[package_name].clone();
            let is_root = *package_name == self.root;

            // Interfaces of everything this package depends on, transitively.
            // A dependency with none — because it fell back, or could not
            // export — simply contributes nothing, and this package will then
            // fail its modular attempt and fall back too.
            let dependencies = package.transitive_dependencies(&self.resolution_graph);
            let available = dependencies
                .iter()
                .filter_map(|dep| interfaces.get(dep).cloned())
                .flatten()
                .collect::<Vec<_>>();
            let complete = dependencies.iter().all(|dep| interfaces.contains_key(dep));

            // What this package is about to be compiled against. A dependency
            // contributes its interface hash where it has one, and otherwise a
            // digest of what is consumed in its place: its sources, or its
            // bytecode when it has none.
            let mut dependency_keys = BTreeMap::new();
            for dep in &dependencies {
                let key = match built.get(dep).and_then(|p| p.interface_hash.clone()) {
                    Some(hash) => hash,
                    None => self.content_digest(&self.resolution_graph.package_table[dep])?,
                };
                dependency_keys.insert(*dep, key);
            }

            // Reuse the previous build when nothing it depends on has moved.
            //
            // Except the root when the caller needs a model: reusing artifacts
            // skips compilation, and the model is a product of compiling. Its
            // dependencies still come from interfaces, which is where the time
            // goes, so this costs one small package rather than the graph.
            //
            // `dependency_cache_root` is searched first and never written, so
            // a package found there costs nothing and leaves nothing behind.
            // It holds dependencies only: the root is this build's product, and
            // a stale copy of it in a shared directory must not stand in for
            // the one being asked for.
            let search_roots: Vec<&Path> = if is_root {
                vec![build_root.as_path()]
            } else {
                dependency_cache_root
                    .as_deref()
                    .into_iter()
                    .chain([build_root.as_path()])
                    .collect()
            };
            let cached = if is_root && model_required {
                None
            } else {
                search_roots.into_iter().find_map(|root| {
                    let on_disk = OnDiskCompiledPackage::from_path(
                        &root
                            .join(package_name.as_str())
                            .join(CompiledPackageLayout::BuildInfo.path()),
                    )
                    .ok()?;
                    if !CompiledPackage::can_load_cached_modular(
                        &on_disk,
                        &self.resolution_graph,
                        &package,
                        is_root,
                        &dependency_keys,
                    ) {
                        return None;
                    }
                    // Paired with the root it came from: interfaces are read
                    // from there, which is not necessarily where this build
                    // writes.
                    Some((on_disk.into_compiled_package().ok()?, root))
                })
            };

            if let Some((compiled, found_in)) = cached {
                writeln!(writer, "{} {}", "CACHED".bold().green(), package_name)?;
                if compiled.interface_hash.is_some() {
                    interfaces.insert(
                        *package_name,
                        Self::interface_paths(found_in, *package_name)?,
                    );
                }
                root_reused |= is_root;
                built.insert(*package_name, compiled);
                continue;
            }

            let modular = if complete {
                match CompiledPackage::build_all(
                    writer,
                    &project_root,
                    package.clone(),
                    vec![],
                    config,
                    external_checks.clone(),
                    &self.resolution_graph,
                    available,
                    &mut driver,
                ) {
                    Ok(result) => Some(result),
                    Err(error) => {
                        writeln!(
                            writer,
                            "{} {}: compiling against interfaces failed ({:#}), \
                             recompiling against sources",
                            "FALLBACK".bold().yellow(),
                            package_name,
                            error
                        )?;
                        None
                    },
                }
            } else {
                None
            };

            let (mut compiled, model) = match modular {
                Some(result) => result,
                None => CompiledPackage::build_all(
                    writer,
                    &project_root,
                    package.clone(),
                    self.source_dependencies_of(&package),
                    config,
                    external_checks.clone(),
                    &self.resolution_graph,
                    vec![],
                    &mut driver,
                )?,
            };

            compiled.dependency_keys = dependency_keys;
            // `build_all` has already written this package out. Only the
            // dependency keys are learned here, and they live in
            // `BuildInfo.yaml` — re-saving the package would delete and
            // rewrite every artifact to record them.
            compiled.save_build_info(&build_root)?;
            if compiled.interface_hash.is_some() {
                interfaces.insert(
                    *package_name,
                    Self::interface_paths(&build_root, *package_name)?,
                );
            }
            if is_root {
                root_model = model;
            }
            built.insert(*package_name, compiled);
        }

        // Assemble the root exactly as the monolithic path presents it: its own
        // units plus every dependency's, so callers see no difference.
        let mut root = built
            .remove(&self.root)
            .ok_or_else(|| anyhow::anyhow!("the root package was not built"))?;
        // Replaced rather than appended to, so this holds whatever the root
        // arrived with. Today a cached root arrives with none — each package is
        // built alone here, so it records no dependencies for
        // `into_compiled_package` to load — but were that to change, appending
        // would leave a superseded copy ahead of the rebuilt one, and
        // `get_module_by_name` returns the first match. Every package in the
        // graph was just visited, so `built` is by construction the current set.
        root.deps_compiled_units.clear();
        // Consumed rather than borrowed: `built` is not read again, and each
        // unit holds a `CompiledModule` and its source map, so cloning them
        // here would deep-copy the whole dependency graph's bytecode.
        for (name, package) in built {
            root.deps_compiled_units.extend(
                package
                    .root_compiled_units
                    .into_iter()
                    .map(|unit| (name, unit)),
            );
        }
        // A cached root loads with no bytecode dependencies, and one compiled
        // against interfaces was given none. Publishing declares them in the
        // package metadata, so recover them the way `build_all` finds them.
        root.bytecode_deps.clear();
        let root_package = &self.resolution_graph.package_table[&self.root];
        for dep in root_package.transitive_dependencies(&self.resolution_graph) {
            let dep_package = &self.resolution_graph.package_table[&dep];
            if dep_package
                .get_sources(&self.resolution_graph.build_options)?
                .is_empty()
            {
                for path in dep_package.get_bytecodes()? {
                    root.bytecode_deps
                        .insert(dep, get_module_in_package(dep, path.as_str())?);
                }
            }
        }

        if root_reused {
            // The root's own artifacts are already on disk and correct. Only
            // its copies of dependency bytecode can be stale — a dependency
            // whose *body* changed keeps its interface hash, so the root stays
            // cached while its bytecode moves — and those are refreshed in
            // place, because re-saving the root would delete the sources its
            // cached units still point at.
            let on_disk = OnDiskCompiledPackage::from_path(
                &build_root
                    .join(self.root.as_str())
                    .join(CompiledPackageLayout::BuildInfo.path()),
            )?;
            on_disk.refresh_dependency_units(&root.deps_compiled_units, bytecode_version)?;
        } else {
            root.save_to_disk(build_root.clone(), bytecode_version)?;
        }

        Self::clean(&build_root, self.sorted_deps.iter().copied().collect())?;
        Ok((root, root_model))
    }

    /// A digest of what a dependency without an interface is consumed as: its
    /// sources, or its `.mv` files when it has none, which `source_digest`
    /// does not cover.
    fn content_digest(&self, package: &ResolvedPackage) -> Result<String> {
        if !package
            .get_sources(&self.resolution_graph.build_options)?
            .is_empty()
        {
            return Ok(package.source_digest.to_string());
        }
        let mut hashes = package
            .get_bytecodes()?
            .iter()
            .map(|path| Ok(format!("{:X}", Sha256::digest(&fs::read(path.as_str())?))))
            .collect::<Result<Vec<_>>>()?;
        hashes.sort();
        Ok(format!("{:X}", Sha256::digest(hashes.concat().as_bytes())))
    }

    /// The interface files a package wrote, as compiler inputs.
    fn interface_paths(build_root: &Path, package_name: PackageName) -> Result<Vec<String>> {
        let dir = build_root
            .join(package_name.as_str())
            .join(CompiledPackageLayout::CompiledInterfaces.path());
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        // Filtered on extension, the same way the rest of the build directory
        // is read. Taking every entry would hand an editor swap file or a
        // `.DS_Store` to the compiler as an interface.
        let mut paths = find_filenames(&[dir.to_string_lossy().into_owned()], |path| {
            extension_equals(path, "json")
        })?;
        // Sorted so the compiler sees a fixed order regardless of the
        // filesystem's; import ordering is resolved by dependency anyway, but a
        // stable input order keeps builds reproducible.
        paths.sort();
        Ok(paths)
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
