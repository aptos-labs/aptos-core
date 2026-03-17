// Copyright (c) Aptos Foundation
// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use super::package_layout::CompiledPackageLayout;
use crate::{
    compilation::{
        compiled_package::{
            build_and_report_no_exit_v2_driver, build_and_report_v2_driver,
            named_address_mapping_for_compiler, CompiledPackage, CompiledPackageInfo,
            CompiledUnitWithSource, OnDiskCompiledPackage,
        },
        interface_hash::compute_interface_hash,
    },
    resolution::resolution_graph::{ResolvedGraph, ResolvedPackage},
    source_package::parsed_manifest::{PackageDigest, PackageName},
    CompilerConfig, CompilerVersion,
};
use anyhow::{Context, Result};
use colored::Colorize;
use legacy_move_compiler::{
    compiled_unit::{AnnotatedCompiledUnit, CompiledUnit, NamedCompiledModule},
    diagnostics::FilesSourceText,
    shared::NumericalAddress,
};
use move_binary_format::file_format::CompiledModule;
use move_command_line_common::files::{extension_equals, find_filenames, MOVE_COMPILED_EXTENSION};
use move_compiler_v2::{external_checks::ExternalChecks, Experiment};
use move_model::model;
use petgraph::algo::toposort;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
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

/// Per-package state accumulated during the incremental build loop.
/// Stored for each compiled (or cache-hit) package so its dependents can
/// find the pre-compiled bytecode and interface hash.
struct PackageDepState {
    /// Hash of this package's public/friend API surface.
    interface_hash: PackageDigest,
    /// Absolute paths to `.mv` bytecode files produced for this package.
    bytecode_paths: Vec<PathBuf>,
}

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
        let build_opts = &self.resolution_graph.build_options;

        // Route to the incremental per-package path when:
        //   1. `modular_compilation` is enabled
        //   2. Not using compiler V1 (modular compilation is V2-only)
        //   3. Not generating docs (docs need a full GlobalEnv spanning all packages)
        //   4. Not generating a move model (callers expect a GlobalEnv with all modules)
        let use_incremental = build_opts.modular_compilation
            && !matches!(
                build_opts.compiler_config.compiler_version,
                Some(CompilerVersion::V1)
            )
            && !build_opts.generate_docs
            && !build_opts.generate_move_model;

        if use_incremental {
            return self.compile_packages_incrementally(writer, config, external_checks, driver);
        }

        // ---------------------------------------------------------------
        // Legacy monolithic path (unchanged)
        // ---------------------------------------------------------------
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

    // -----------------------------------------------------------------------
    // Incremental per-package compilation
    // -----------------------------------------------------------------------

    /// Compile each package in topological order (leaves first).
    ///
    /// Each package is compiled independently using only the pre-compiled
    /// bytecode of its dependencies. Packages whose source and dependency
    /// interfaces are unchanged are loaded from the on-disk cache.
    fn compile_packages_incrementally<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        mut driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(p) => p.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let build_root = project_root.join(CompiledPackageLayout::Root.path());

        // Accumulated per-package state: bytecode paths + interface hash.
        let mut dep_states: BTreeMap<PackageName, PackageDepState> = BTreeMap::new();
        let mut root_env: Option<model::GlobalEnv> = None;

        for &pkg_name in &self.sorted_deps {
            let is_root = pkg_name == self.root;
            let resolved_pkg = &self.resolution_graph.package_table[&pkg_name];

            // Interface hashes of this package's direct dependencies.
            let immediate_dep_names = resolved_pkg.immediate_dependencies(&self.resolution_graph);
            let current_dep_hashes: BTreeMap<PackageName, PackageDigest> = immediate_dep_names
                .iter()
                .filter_map(|d| dep_states.get(d).map(|s| (*d, s.interface_hash)))
                .collect();

            // All transitive dep bytecode paths (the compiler needs the full closure).
            let all_dep_bytecode_paths: Vec<PathBuf> = dep_states
                .values()
                .flat_map(|s| s.bytecode_paths.iter().cloned())
                .collect();

            let pkg_build_dir = build_root.join(pkg_name.as_str());

            // Try to use the on-disk cache.
            let is_cached = OnDiskCompiledPackage::from_path(&pkg_build_dir)
                .ok()
                .filter(|on_disk| {
                    CompiledPackage::is_package_cache_valid(
                        on_disk,
                        resolved_pkg,
                        &self.resolution_graph.build_options,
                        &current_dep_hashes,
                    )
                })
                .is_some();

            // Non-root cache hits skip compilation entirely.
            // Root is always compiled so callers (e.g. extended checks, ABI
            // generation) receive a fresh GlobalEnv. When root's artifacts are
            // already valid on disk we still compile it but skip re-saving them.
            let skip_compilation = !is_root && is_cached;

            if is_cached {
                writeln!(writer, "{} {}", "CACHED".green().bold(), pkg_name)?;
            } else {
                writeln!(writer, "{} {}", "BUILDING".cyan().bold(), pkg_name)?;
            }

            let interface_hash = if skip_compilation {
                // Load interface_hash from the on-disk record.
                OnDiskCompiledPackage::from_path(&pkg_build_dir)
                    .expect("on-disk package must be readable after cache check")
                    .package
                    .compiled_package_info
                    .interface_hash
                    .expect("cached package must have interface_hash set")
            } else {
                let (hash, env) = compile_single_package(
                    resolved_pkg,
                    &all_dep_bytecode_paths,
                    &current_dep_hashes,
                    config,
                    if is_root {
                        external_checks.clone()
                    } else {
                        vec![]
                    },
                    &mut driver,
                    &self.resolution_graph,
                    is_root,
                    &build_root,
                    !is_cached, // save_artifacts: skip when root's cache is still valid
                )?;
                root_env = env;
                hash
            };

            let bytecode_paths = collect_bytecode_module_paths(&pkg_build_dir);
            dep_states.insert(pkg_name, PackageDepState {
                interface_hash,
                bytecode_paths,
            });
        }

        // Assemble the final `CompiledPackage` with `deps_compiled_units` populated.
        let final_pkg = assemble_final_package(&self.sorted_deps, self.root, &build_root)?;

        Self::clean(&build_root, self.sorted_deps.iter().copied().collect())?;

        Ok((final_pkg, root_env))
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

// ---------------------------------------------------------------------------
// Incremental helpers (module-level free functions)
// ---------------------------------------------------------------------------

/// Compile a single package using pre-compiled bytecode of its dependencies.
///
/// Returns the interface hash of this package's public/friend API surface,
/// and — for the root package only — the `GlobalEnv` produced by the compiler.
/// The `GlobalEnv` is needed by callers such as `BuiltPackage::build()` that
/// run extended checks and generate ABI/metadata from it. Dependency packages
/// return `None` because their artifacts are already validated on disk.
/// The compiled artifacts are written to `pkg_build_dir`.
fn compile_single_package(
    resolved_pkg: &ResolvedPackage,
    all_dep_bytecode_paths: &[PathBuf],
    dep_interface_hashes: &BTreeMap<PackageName, PackageDigest>,
    config: &CompilerConfig,
    external_checks: Vec<Arc<dyn ExternalChecks>>,
    driver: &mut impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    resolution_graph: &ResolvedGraph,
    is_root: bool,
    build_root: &Path,
    save_artifacts: bool,
) -> Result<(PackageDigest, Option<model::GlobalEnv>)> {
    let build_opts = &resolution_graph.build_options;
    let pkg_name = resolved_pkg.source_package.package.name;

    // Collect this package's source files.
    let sources: Vec<String> = resolved_pkg
        .get_sources(build_opts)?
        .into_iter()
        .map(|s| s.as_str().to_owned())
        .collect();

    // Build the global named-address mapping (must be consistent across all packages).
    let named_address_mapping = build_global_address_mapping(resolution_graph)?;

    let effective_compiler_version = config.compiler_version.unwrap_or_default();
    let effective_language_version = config.language_version.unwrap_or_default();
    effective_compiler_version.check_language_support(effective_language_version)?;

    // KEY CHANGE from the monolithic path:
    //   sources      = this package's .move files
    //   sources_deps = [] (empty — deps are pre-compiled)
    //   dependencies = pre-compiled .mv files of ALL transitive deps
    let mut options = move_compiler_v2::Options {
        sources: sources.clone(),
        sources_deps: vec![],
        dependencies: all_dep_bytecode_paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        named_address_mapping,
        skip_attribute_checks: build_opts.compiler_config.skip_attribute_checks,
        known_attributes: build_opts.compiler_config.known_attributes.clone(),
        language_version: Some(effective_language_version),
        compiler_version: Some(effective_compiler_version),
        compile_test_code: build_opts.test_mode && is_root,
        experiments: config.experiments.clone(),
        external_checks,
        print_errors: config.print_errors,
        ..Default::default()
    };
    options = options.set_experiment(Experiment::ATTACH_COMPILED_MODULE, true);

    let (file_map, all_compiled_units, env) = driver(options)?;

    // Build a set of this package's source paths for filtering.
    let sources_set: BTreeSet<String> = sources.into_iter().collect();

    let mut root_compiled_units: Vec<CompiledUnitWithSource> = vec![];
    for annot_unit in all_compiled_units {
        let source_path_str = match file_map.get(&annot_unit.loc().file_hash()) {
            Some(s) => s.0.as_str().to_owned(),
            None => continue,
        };
        // Only keep units whose source belongs to this package.
        if !sources_set.contains(&source_path_str) {
            continue;
        }
        root_compiled_units.push(CompiledUnitWithSource {
            unit: annot_unit.into_compiled_unit(),
            source_path: PathBuf::from(&source_path_str),
        });
    }

    // Compute the interface hash from this package's compiled modules.
    let root_modules: Vec<&CompiledModule> = root_compiled_units
        .iter()
        .filter_map(|u| match &u.unit {
            CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
            _ => None,
        })
        .collect();
    let interface_hash = compute_interface_hash(&root_modules);

    // Build and save the CompiledPackage for this package.
    let bytecode_version =
        effective_language_version.infer_bytecode_version(config.bytecode_version);

    let compiled_package_info = CompiledPackageInfo {
        package_name: pkg_name,
        address_alias_instantiation: resolved_pkg.resolution_table.clone(),
        source_digest: Some(resolved_pkg.source_digest),
        build_flags: build_opts.clone(),
        interface_hash: Some(interface_hash),
    };

    // Build a transient CompiledPackage (no deps_compiled_units; each dep is
    // stored in its own directory).
    let compiled_pkg = CompiledPackage {
        compiled_package_info,
        root_compiled_units,
        deps_compiled_units: vec![],
        bytecode_deps: BTreeMap::new(),
        compiled_docs: None,
        compiled_abis: None,
    };

    // Save to disk with the dep interface hashes for future cache checks.
    // Skipped when the root package's existing artifacts are already valid
    // (compiled only to obtain GlobalEnv, not to regenerate bytecode).
    if save_artifacts {
        compiled_pkg.save_to_disk_with_dep_hashes(
            build_root.to_path_buf(),
            bytecode_version,
            dep_interface_hashes.clone(),
        )?;
    }

    // Return the GlobalEnv only for the root package — callers (e.g.
    // BuiltPackage::build_with_external_checks_to) need it to run extended
    // checks and generate metadata. Dependency packages discard it because
    // their artifacts have already been validated.
    let root_env = if is_root { Some(env) } else { None };
    Ok((interface_hash, root_env))
}

/// After the per-package loop, reconstruct the `CompiledPackage` shape that
/// callers expect: a root package with `deps_compiled_units` fully populated
/// from the individual per-package on-disk artifacts.
fn assemble_final_package(
    sorted_deps: &[PackageName],
    root_name: PackageName,
    build_root: &Path,
) -> Result<CompiledPackage> {
    // Load the root package from disk.
    let root_dir = build_root.join(root_name.as_str());
    let root_on_disk = OnDiskCompiledPackage::from_path(&root_dir)
        .with_context(|| format!("loading root package '{}'", root_name))?;
    // `into_compiled_package` reads `root_compiled_units` correctly; its
    // `deps_compiled_units` may be empty for per-package builds (which is fine
    // since we rebuild it below).
    let root_pkg = root_on_disk.into_compiled_package()?;

    let mut deps_compiled_units: Vec<(PackageName, CompiledUnitWithSource)> = vec![];

    for &dep_name in sorted_deps {
        if dep_name == root_name {
            continue;
        }
        let dep_dir = build_root.join(dep_name.as_str());
        let dep_on_disk = OnDiskCompiledPackage::from_path(&dep_dir)
            .with_context(|| format!("loading dep package '{}'", dep_name))?;
        // Only take the root compiled units of each dep (their own modules).
        let dep_pkg = dep_on_disk.into_compiled_package()?;
        for unit in dep_pkg.root_compiled_units {
            deps_compiled_units.push((dep_name, unit));
        }
    }

    Ok(CompiledPackage {
        deps_compiled_units,
        ..root_pkg
    })
}

/// Return the absolute paths of all `.mv` bytecode files for `pkg_build_dir`.
fn collect_bytecode_module_paths(pkg_build_dir: &Path) -> Vec<PathBuf> {
    let module_path = pkg_build_dir.join(CompiledPackageLayout::CompiledModules.path());
    let script_path = pkg_build_dir.join(CompiledPackageLayout::CompiledScripts.path());
    let mut dirs = vec![];
    if module_path.exists() {
        dirs.push(module_path.to_string_lossy().into_owned());
    }
    if script_path.exists() {
        dirs.push(script_path.to_string_lossy().into_owned());
    }
    if dirs.is_empty() {
        return vec![];
    }
    find_filenames(&dirs, |p| extension_equals(p, MOVE_COMPILED_EXTENSION))
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect()
}

/// Build the global named-address mapping string vector from all packages in
/// the resolution graph (same logic as `build_all`, extracted for reuse).
fn build_global_address_mapping(resolution_graph: &ResolvedGraph) -> Result<Vec<String>> {
    let mut map: BTreeMap<String, NumericalAddress> = BTreeMap::new();

    for (pkg_name, pkg) in &resolution_graph.package_table {
        let named = named_address_mapping_for_compiler(&pkg.resolution_table);
        for (sym, num_addr) in named {
            let entry = map.entry(sym.as_str().to_owned()).or_insert(num_addr);
            if entry.into_bytes() != num_addr.into_bytes() {
                anyhow::bail!(
                    "found remapped address alias `{}` in package `{}`, \
                    please use unique address aliases across dependencies",
                    sym,
                    pkg_name
                );
            }
        }
    }

    // Also include `additional_named_addresses` from build options.
    for (name, addr) in &resolution_graph.build_options.additional_named_addresses {
        let num = NumericalAddress::new(
            addr.into_bytes(),
            legacy_move_compiler::shared::NumberFormat::Hex,
        );
        map.entry(name.clone()).or_insert(num);
    }

    Ok(map
        .into_iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect())
}
