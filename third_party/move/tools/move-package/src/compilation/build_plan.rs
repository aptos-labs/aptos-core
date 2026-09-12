// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::package_layout::CompiledPackageLayout;
use crate::{
    compilation::compiled_package::{
        build_and_report_no_exit_v2_driver, build_and_report_v2_driver, CompiledPackage,
        OnDiskCompiledPackage,
    },
    resolution::resolution_graph::ResolvedGraph,
    source_package::parsed_manifest::PackageName,
    CompilerConfig,
};
use anyhow::{Context, Result};
use colored::Colorize;
use legacy_move_compiler::{compiled_unit::AnnotatedCompiledUnit, diagnostics::FilesSourceText};
use move_compiler_v2::external_checks::ExternalChecks;
use move_model::model;
use petgraph::algo::toposort;
use std::{
    collections::{BTreeMap, BTreeSet},
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

    pub fn compile_with_driver<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        if self.resolution_graph.build_options.modular_compilation {
            return self.compile_modular(writer, config, external_checks, driver);
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
    /// # Falling back
    ///
    /// A package that calls a *cross-package* `public inline` function cannot
    /// be compiled this way, and that is inherent rather than a gap in the
    /// implementation: inlining needs the callee's body, an interface carries
    /// no bodies, and `MODULAR_COMPILATION.md` §4 explains why carrying them
    /// would require a different artifact altogether. Such a package is
    /// recompiled against its dependencies' sources — which is exactly what the
    /// monolithic path does — and the result is indistinguishable from today.
    ///
    /// The decision is made on the *dependency* side, before anything is
    /// attempted: a package that exports a non-private `inline` function
    /// publishes no interface at all, so a dependent sees an incomplete set and
    /// compiles against sources instead.
    ///
    /// Deciding afterwards would be more precise — the question is really
    /// whether *this* package calls such a function, which is a property of its
    /// call graph and unknown until it is compiled — but it is not available:
    /// `build_and_report_v2_driver` exits the process on a compile error, so a
    /// failed attempt cannot be caught and retried. Trying and falling back
    /// would abort the build instead of falling back.
    ///
    /// The cost of deciding early is conservatism: a package falls back even if
    /// it calls none of its dependencies' inline functions. For the Aptos
    /// framework that is every package, since `move-stdlib` exports 36 such
    /// functions and `aptos-stdlib` 32. Caching still applies to a package that
    /// fell back — it is compiled separately and keyed on its dependencies'
    /// source digests — so the rebuild win survives even where interfaces do
    /// not engage.
    fn compile_modular<W: Write>(
        &self,
        writer: &mut W,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        mut driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<model::GlobalEnv>)> {
        let project_root = match &self.resolution_graph.build_options.install_dir {
            Some(under_path) => under_path.clone(),
            None => self.resolution_graph.root_package_path.clone(),
        };
        let build_root = project_root.join(CompiledPackageLayout::Root.path());
        let bytecode_version = config
            .language_version
            .unwrap_or_default()
            .infer_bytecode_version(config.bytecode_version);

        let mut interfaces: BTreeMap<PackageName, Vec<String>> = BTreeMap::new();
        let mut built: BTreeMap<PackageName, CompiledPackage> = BTreeMap::new();
        let mut reused: BTreeSet<PackageName> = BTreeSet::new();
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
            // contributes its interface hash where it has one, and its source
            // digest otherwise — the latter because a package without an
            // interface is consumed as *source*, so any edit to it matters.
            let mut dependency_keys = BTreeMap::new();
            for dep in &dependencies {
                let dep_package = &self.resolution_graph.package_table[dep];
                let key = built
                    .get(dep)
                    .and_then(|package| package.interface_hash.clone())
                    .unwrap_or_else(|| dep_package.source_digest.to_string());
                dependency_keys.insert(*dep, key);
            }

            // Reuse the previous build when nothing it depends on has moved.
            let cached = OnDiskCompiledPackage::from_path(
                &build_root
                    .join(package_name.as_str())
                    .join(CompiledPackageLayout::BuildInfo.path()),
            )
            .ok()
            .filter(|on_disk| {
                CompiledPackage::can_load_cached_modular(
                    on_disk,
                    &self.resolution_graph,
                    &package,
                    is_root,
                    &dependency_keys,
                )
            })
            .and_then(|on_disk| on_disk.into_compiled_package().ok());

            if let Some(compiled) = cached {
                writeln!(writer, "{} {}", "CACHED".bold().green(), package_name)?;
                if compiled.compiled_interfaces.is_some() {
                    interfaces.insert(
                        *package_name,
                        Self::interface_paths(&build_root, *package_name)?,
                    );
                }
                reused.insert(*package_name);
                built.insert(*package_name, compiled);
                continue;
            }

            let modular = if complete {
                CompiledPackage::build_all(
                    writer,
                    &project_root,
                    package.clone(),
                    vec![],
                    config,
                    external_checks.clone(),
                    &self.resolution_graph,
                    available,
                    &mut driver,
                )
                .ok()
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
            compiled.save_to_disk(build_root.clone(), bytecode_version)?;
            if compiled.compiled_interfaces.is_some() {
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
        for (name, package) in &built {
            root.deps_compiled_units.extend(
                package
                    .root_compiled_units
                    .iter()
                    .map(|unit| (*name, unit.clone())),
            );
        }

        if reused.contains(&self.root) {
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

    /// The interface files a package wrote, as compiler inputs.
    fn interface_paths(build_root: &Path, package_name: PackageName) -> Result<Vec<String>> {
        let dir = build_root
            .join(package_name.as_str())
            .join(CompiledPackageLayout::CompiledInterfaces.path());
        if !dir.is_dir() {
            return Ok(vec![]);
        }
        let mut paths = std::fs::read_dir(&dir)?
            .map(|entry| Ok(entry?.path().to_string_lossy().into_owned()))
            .collect::<Result<Vec<_>>>()?;
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
