// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compilation::package_layout::CompiledPackageLayout,
    resolution::resolution_graph::{Renaming, ResolvedGraph, ResolvedPackage, ResolvedTable},
    source_package::{
        layout::{SourcePackageLayout, REFERENCE_TEMPLATE_FILENAME},
        parsed_manifest::{FileName, NamedAddress, PackageDigest, PackageName},
    },
    BuildConfig,
};
use anyhow::Result;
use colored::Colorize;
use move_abigen::{Abigen, AbigenOptions};
use move_binary_format::file_format::{CompiledModule, CompiledScript};
use move_bytecode_source_map::utils::source_map_from_file;
use move_bytecode_utils::Modules;
use move_command_line_common::files::{
    extension_equals, find_filenames, MOVE_COMPILED_EXTENSION, MOVE_EXTENSION, SOURCE_MAP_EXTENSION,
};
use move_core_types::language_storage::ModuleId;
use move_docgen::{Docgen, DocgenOptions};
use move_lang::{
    compiled_unit::{
        AnnotatedCompiledUnit, CompiledUnit, NamedCompiledModule, NamedCompiledScript,
    },
    diagnostics::FilesSourceText,
    shared::{Flags, NumericalAddress},
    Compiler,
};
use move_model::{model::GlobalEnv, options::ModelBuilderOptions, run_model_builder_with_options};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

/// Module resolution data
pub type ModuleResolutionMetadata = BTreeMap<ModuleId, NamedAddress>;

#[derive(Debug, Clone)]
pub enum CompilationCachingStatus {
    /// The package and all if its dependencies were cached
    Cached,
    /// At least this package and/or one of its dependencies needed to be rebuilt
    Recompiled,
}

#[derive(Debug, Clone)]
pub struct CompiledUnitWithSource {
    pub unit: CompiledUnit,
    pub source_path: PathBuf,
}

/// Represents meta information about a package and the information it was compiled with. Shared
/// across both the `CompiledPackage` and `OnDiskCompiledPackage` structs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPackageInfo {
    /// The name of the compiled package
    pub package_name: PackageName,
    /// The instantiations for all named addresses that were used for compilation
    pub address_alias_instantiation: ResolvedTable,
    /// The module resolution {<addr>::<module_name> |-> <named_address>} mapping used for
    /// compiling against bytecode using named addresses and to allow address renaming.
    pub module_resolution_metadata: ModuleResolutionMetadata,
    /// The hash of the source directory at the time of compilation. `None` if the source for this
    /// package is not available/this package was not compiled.
    pub source_digest: Option<PackageDigest>,
    /// The build flags that were used when compiling this package.
    pub build_flags: BuildConfig,
}

/// Represents a compiled package in memory.
#[derive(Debug, Clone)]
pub struct CompiledPackage {
    /// Meta information about the compilation of this `CompiledPackage`
    pub compiled_package_info: CompiledPackageInfo,
    /// The output compiled bytecode (both module, and scripts) along with its source file
    pub compiled_units: Vec<CompiledUnitWithSource>,
    /// Packages that this package depends on. Non-transitive dependencies.
    pub dependencies: Vec<CompiledPackage>,

    // Optional artifacts from compilation
    //
    /// filename -> doctext
    pub compiled_docs: Option<Vec<(String, String)>>,
    /// filename -> json bytes for ScriptABI. Can then be used to generate transaction builders in
    /// various languages.
    pub compiled_abis: Option<Vec<(String, Vec<u8>)>>,
}

/// Represents a compiled package that has been saved to disk. This holds only the minimal metadata
/// needed to reconstruct a `CompiledPackage` package from it and to determine whether or not a
/// recompilation of the package needs to be performed or not.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnDiskPackage {
    /// Information about the package and the specific compilation that was done.
    pub compiled_package_info: CompiledPackageInfo,
    /// Dependency names for this package.
    pub dependencies: Vec<FileName>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnDiskCompiledPackage {
    /// Path to the root of the package and its data on disk. Relative to/rooted at the directory
    /// containing the `Move.toml` file for this package.
    pub root_path: PathBuf,
    pub package: OnDiskPackage,
}

impl CompilationCachingStatus {
    /// Returns `true` if this package and all dependencies are cached
    pub fn is_cached(&self) -> bool {
        matches!(self, Self::Cached)
    }

    /// Returns `true` if this package or one of its dependencies was rebuilt
    pub fn is_rebuilt(&self) -> bool {
        !self.is_cached()
    }
}

impl OnDiskCompiledPackage {
    pub fn from_path(p: &Path) -> Result<Self> {
        let (buf, root_path) = if p.exists() && extension_equals(p, "yaml") {
            (std::fs::read(p)?, p.parent().unwrap().parent().unwrap())
        } else {
            (
                std::fs::read(p.join(CompiledPackageLayout::BuildInfo.path()))?,
                p.parent().unwrap(),
            )
        };
        let package = serde_yaml::from_slice::<OnDiskPackage>(&buf)?;
        Ok(Self {
            root_path: root_path.to_path_buf(),
            package,
        })
    }

    pub fn into_compiled_package(&self) -> Result<CompiledPackage> {
        let compiled_units = self.get_compiled_units_paths()?;
        let source_maps = find_filenames(
            &[self
                .root_path
                .join(self.package.compiled_package_info.package_name.as_str())
                .join(CompiledPackageLayout::SourceMaps.path())
                .to_string_lossy()
                .to_string()],
            |path| extension_equals(path, SOURCE_MAP_EXTENSION),
        )
        .unwrap_or_else(|_| vec![]);
        assert_eq!(
            compiled_units.len(),
            source_maps.len(),
            "number of compiled units and source maps differ, {} != {}",
            compiled_units.len(),
            source_maps.len()
        );
        let compiled_units = compiled_units
            .iter()
            .map(|bytecode_path| {
                let bytecode_path = Path::new(bytecode_path.as_str());
                let file_stem = bytecode_path.file_stem().unwrap();
                let bytecode_bytes = std::fs::read(&bytecode_path)?;
                let source_map = source_map_from_file(
                    &CompiledPackageLayout::SourceMaps
                        .from_sibling_path(bytecode_path)
                        .ok_or_else(|| anyhow::format_err!("Unable to find source map"))?
                        .join(file_stem)
                        .with_extension(SOURCE_MAP_EXTENSION),
                )?;
                let source_path = CompiledPackageLayout::Sources
                    .from_sibling_path(bytecode_path)
                    .ok_or_else(|| anyhow::format_err!("Unable to find source file"))?
                    .join(file_stem)
                    .with_extension(MOVE_EXTENSION);
                match CompiledScript::deserialize(&bytecode_bytes) {
                    Ok(script) => {
                        let name = FileName::from(
                            bytecode_path
                                .file_stem()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                        );
                        let unit = CompiledUnit::Script(NamedCompiledScript {
                            name,
                            script,
                            source_map,
                        });
                        Ok(CompiledUnitWithSource { unit, source_path })
                    }
                    Err(_) => {
                        let module = CompiledModule::deserialize(&bytecode_bytes)?;
                        let (address_bytes, module_name) = {
                            let id = module.self_id();
                            let parsed_addr = NumericalAddress::new(
                                id.address().into_bytes(),
                                move_lang::shared::NumberFormat::Hex,
                            );
                            let module_name = FileName::from(id.name().as_str());
                            (parsed_addr, module_name)
                        };
                        let unit = CompiledUnit::Module(NamedCompiledModule {
                            address: address_bytes,
                            name: module_name,
                            module,
                            source_map,
                        });
                        Ok(CompiledUnitWithSource { unit, source_path })
                    }
                }
            })
            .collect::<Result<Vec<_>>>()?;

        let mut dependencies = Vec::new();
        let self_path = self.package.compiled_package_info.package_name;
        for dep_path in &self.package.dependencies {
            if dep_path == &self_path {
                continue;
            }
            dependencies.push(
                Self::from_path(
                    &self
                        .root_path
                        .join(dep_path.as_str())
                        .join(CompiledPackageLayout::BuildInfo.path()),
                )?
                .into_compiled_package()?,
            )
        }

        let docs_path = self
            .root_path
            .join(self.package.compiled_package_info.package_name.as_str())
            .join(CompiledPackageLayout::CompiledDocs.path());
        let compiled_docs = if docs_path.is_dir() {
            Some(
                find_filenames(&[docs_path.to_string_lossy().to_string()], |path| {
                    extension_equals(path, "md")
                })?
                .into_iter()
                .map(|path| {
                    let contents = std::fs::read_to_string(&path).unwrap();
                    (path, contents)
                })
                .collect(),
            )
        } else {
            None
        };

        let abi_path = self
            .root_path
            .join(self.package.compiled_package_info.package_name.as_str())
            .join(CompiledPackageLayout::CompiledABIs.path());
        let compiled_abis = if abi_path.is_dir() {
            Some(
                find_filenames(&[abi_path.to_string_lossy().to_string()], |path| {
                    extension_equals(path, "abi")
                })?
                .into_iter()
                .map(|path| {
                    let contents = std::fs::read(&path).unwrap();
                    (path, contents)
                })
                .collect(),
            )
        } else {
            None
        };

        Ok(CompiledPackage {
            compiled_package_info: self.package.compiled_package_info.clone(),
            compiled_units,
            dependencies,
            compiled_docs,
            compiled_abis,
        })
    }

    /// Save `bytes` under `path_under` relative to the package on disk
    pub(crate) fn save_under(
        &self,
        dir_or_file: &Path,
        path_under: Option<PathBuf>,
        bytes: &[u8],
    ) -> Result<()> {
        let mut path_to_save = self.root_path.join(dir_or_file);
        if let Some(under_path) = path_under {
            let parent = under_path.parent().unwrap();
            path_to_save.push(parent);
            std::fs::create_dir_all(&path_to_save)?;
            path_to_save.push(Path::new(under_path.file_name().unwrap()));
        }
        std::fs::write(path_to_save, bytes).map_err(|err| err.into())
    }

    pub(crate) fn has_source_changed_since_last_compile(
        &self,
        resolved_package: &ResolvedPackage,
    ) -> bool {
        match &self.package.compiled_package_info.source_digest {
            // Don't have source available to us
            None => false,
            Some(digest) => digest != &resolved_package.source_digest,
        }
    }
    pub(crate) fn are_build_flags_different(&self, build_config: &BuildConfig) -> bool {
        build_config != &self.package.compiled_package_info.build_flags
    }

    fn get_compiled_units_paths(&self) -> Result<Vec<String>> {
        let mut compiled_unit_paths = vec![self
            .root_path
            .join(self.package.compiled_package_info.package_name.as_str())
            .join(CompiledPackageLayout::CompiledModules.path())
            .to_string_lossy()
            .to_string()];
        let compiled_scripts_path = self
            .root_path
            .join(self.package.compiled_package_info.package_name.as_str())
            .join(CompiledPackageLayout::CompiledScripts.path());
        if compiled_scripts_path.exists() {
            compiled_unit_paths.push(compiled_scripts_path.to_string_lossy().to_string());
        }
        find_filenames(&compiled_unit_paths, |path| {
            extension_equals(path, MOVE_COMPILED_EXTENSION)
        })
    }
}

impl CompiledPackage {
    /// Returns all compiled units for this package in transitive dependencies. Order is not
    /// guaranteed.
    pub fn transitive_compiled_units(&self) -> Vec<CompiledUnit> {
        self.transitive_dependencies()
            .flat_map(|compiled_package| {
                compiled_package
                    .compiled_units
                    .iter()
                    .map(|unit| unit.unit.clone())
            })
            .collect()
    }

    /// Returns all compiled units with sources for this package in transitive dependencies. Order is not
    /// guaranteed.
    pub fn transitive_compiled_units_with_source(&self) -> Vec<CompiledUnitWithSource> {
        self.transitive_dependencies()
            .flat_map(|compiled_package| compiled_package.compiled_units.clone())
            .collect()
    }

    /// Returns compiled modules for this package and its transitive dependencies in dependency
    /// order.
    pub fn transitive_compiled_modules(&self) -> Modules {
        Modules::new(
            self.transitive_dependencies()
                .flat_map(|compiled_package| &compiled_package.compiled_units)
                .filter_map(|unit| match &unit.unit {
                    CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                    CompiledUnit::Script(_) => None,
                }),
        )
    }

    /// Returns `CompiledPackage`s deduped
    pub fn transitive_dependencies(&self) -> impl Iterator<Item = &CompiledPackage> {
        self.dependencies.iter().chain(vec![self])
    }

    pub fn compiled_modules(&self) -> Modules {
        Modules::new(
            self.compiled_units
                .iter()
                .filter_map(|unit| match &unit.unit {
                    CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                    CompiledUnit::Script(_) => None,
                }),
        )
    }

    pub fn modules(&self) -> Result<impl Iterator<Item = &CompiledUnitWithSource>> {
        let mut lookup_modules: BTreeMap<_, _> = self
            .compiled_units
            .iter()
            .filter_map(|unit| match &unit.unit {
                CompiledUnit::Module(NamedCompiledModule { module, .. }) => {
                    Some((module.self_id(), unit))
                }
                CompiledUnit::Script(_) => None,
            })
            .collect();

        let dep_graph: Vec<_> = self
            .transitive_compiled_modules()
            .compute_dependency_graph()
            .compute_topological_order()?
            .cloned()
            .collect();

        Ok(dep_graph
            .into_iter()
            .map(|module| module.self_id())
            .flat_map(move |module_id| lookup_modules.remove(&module_id)))
    }

    pub fn get_module_by_name(&self, module_name: &str) -> Result<&CompiledUnitWithSource> {
        self.modules()?
            .find(|unit| unit.unit.name().as_str() == module_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find module with name '{}' in package {}",
                    module_name,
                    self.compiled_package_info.package_name
                )
            })
    }

    pub fn get_dependency_by_name(&self, package_name: &str) -> Result<&Self> {
        self.dependencies
            .iter()
            .find(|pkg| pkg.compiled_package_info.package_name.as_str() == package_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find dependency for package '{}' with name '{}'",
                    self.compiled_package_info.package_name,
                    package_name
                )
            })
    }

    pub fn scripts(&self) -> impl Iterator<Item = &CompiledUnitWithSource> {
        self.compiled_units
            .iter()
            .filter(|unit| matches!(unit.unit, CompiledUnit::Script(_)))
    }

    fn can_load_cached(
        package: &OnDiskCompiledPackage,
        resolution_graph: &ResolvedGraph,
        resolved_package: &ResolvedPackage,
        is_root_package: bool,
    ) -> bool {
        // TODO: add more tests for the different caching cases
        !(package.has_source_changed_since_last_compile(resolved_package) // recompile if source has changed
            // Recompile if the flags are different
                || package.are_build_flags_different(&resolution_graph.build_options)
                // Force root package recompilation in test mode
                || resolution_graph.build_options.test_mode && is_root_package
                // Recompile if force recompilation is set
                || resolution_graph.build_options.force_recompilation) &&
                // Dive deeper to make sure that instantiations haven't changed since that
                // can be changed by other packages above us in the dependency graph possibly
                package.package.compiled_package_info.address_alias_instantiation
                    == resolved_package.resolution_table
    }

    pub(crate) fn build<W: Write>(
        w: &mut W,
        project_root: &Path,
        resolved_package: ResolvedPackage,
        dependencies: Vec<(CompiledPackage, CompilationCachingStatus)>,
        resolution_graph: &ResolvedGraph,
        is_root_package: bool,
        mut compiler_driver: impl FnMut(
            Compiler,
            bool,
        )
            -> Result<(FilesSourceText, Vec<AnnotatedCompiledUnit>)>,
    ) -> Result<(CompiledPackage, CompilationCachingStatus)> {
        let mut module_resolution_metadata = BTreeMap::new();

        // NB: This renaming needs to be applied in the topological order of dependencies
        for package in &dependencies {
            package
                .0
                .apply_renaming(&mut module_resolution_metadata, &resolved_package.renaming)
        }

        let build_root_path = project_root.join(CompiledPackageLayout::Root.path());
        let path = build_root_path
            .join(resolved_package.source_package.package.name.as_str())
            .join(CompiledPackageLayout::BuildInfo.path());

        // Compare the digest of the package being compiled against the digest of the package at the time
        // of the last compilation to determine if we can reuse the already-compiled package or
        // not. If any dependency has changed we need to recompile.
        if let Ok(package) = OnDiskCompiledPackage::from_path(&path) {
            if dependencies
                .iter()
                .all(|(_, cache_status)| cache_status.is_cached())
                && Self::can_load_cached(
                    &package,
                    resolution_graph,
                    &resolved_package,
                    is_root_package,
                )
            {
                writeln!(
                    w,
                    "{} {}",
                    "CACHED".bold().green(),
                    resolved_package.source_package.package.name,
                )?;
                return package
                    .into_compiled_package()
                    .map(|x| (x, CompilationCachingStatus::Cached));
            }
        }
        writeln!(
            w,
            "{} {}",
            "BUILDING".bold().green(),
            resolved_package.source_package.package.name,
        )?;
        let dependencies: Vec<_> = dependencies.into_iter().map(|x| x.0).collect();

        let dep_paths = dependencies
            .iter()
            .map(|compiled_package| {
                build_root_path
                    .join(
                        compiled_package
                            .compiled_package_info
                            .package_name
                            .to_string(),
                    )
                    .join(CompiledPackageLayout::CompiledModules.path())
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();

        let tmp_interface_dir = tempfile::tempdir()?;
        let in_scope_named_addrs = resolved_package
            .resolution_table
            .iter()
            .map(|(ident, addr)| {
                let parsed_addr =
                    NumericalAddress::new(addr.into_bytes(), move_lang::shared::NumberFormat::Hex);
                (ident.to_string(), parsed_addr)
            })
            .collect::<BTreeMap<_, _>>();
        let sources: Vec<_> = resolved_package
            .get_sources(&resolution_graph.build_options)?
            .into_iter()
            .map(|symbol| symbol.as_str().to_string())
            .collect();
        let flags = if resolution_graph.build_options.test_mode {
            Flags::testing()
        } else {
            Flags::empty()
        };

        let compiler = Compiler::new(&sources, &dep_paths)
            .set_compiled_module_named_address_mapping(
                module_resolution_metadata
                    .clone()
                    .into_iter()
                    .map(|(x, ident)| (x, ident.to_string()))
                    .collect::<BTreeMap<_, _>>(),
            )
            .set_named_address_values(in_scope_named_addrs.clone())
            .set_interface_files_dir(tmp_interface_dir.path().to_string_lossy().to_string())
            .set_flags(flags);
        let (file_map, compiled_units) = compiler_driver(compiler, is_root_package)?;

        let (compiled_units, resolutions): (Vec<_>, Vec<_>) = compiled_units
            .into_iter()
            .map(|annot_unit| {
                let originating_file =
                    PathBuf::from(file_map[&annot_unit.loc().file_hash()].0.as_str());
                match &annot_unit {
                    AnnotatedCompiledUnit::Module(annot_module) => {
                        // Only return resolutions for modules that have named addresses
                        let resolution = match annot_module.module_id() {
                            (Some(str_name), module_id) => Some((module_id, str_name.value)),
                            _ => None,
                        };
                        (
                            CompiledUnitWithSource {
                                unit: annot_unit.into_compiled_unit(),
                                source_path: originating_file,
                            },
                            resolution,
                        )
                    }
                    AnnotatedCompiledUnit::Script(_) => (
                        CompiledUnitWithSource {
                            unit: annot_unit.into_compiled_unit(),
                            source_path: originating_file,
                        },
                        None,
                    ),
                }
            })
            .unzip();

        for (mod_id, name) in resolutions.into_iter().flatten() {
            module_resolution_metadata.insert(mod_id, name);
        }

        let mut compiled_docs = None;
        let mut compiled_abis = None;
        if resolution_graph.build_options.generate_docs
            || resolution_graph.build_options.generate_abis
        {
            let model = run_model_builder_with_options(
                &sources,
                &[tmp_interface_dir.path().to_string_lossy().to_string()],
                ModelBuilderOptions::default(),
                in_scope_named_addrs,
            )?;

            if resolution_graph.build_options.generate_docs {
                compiled_docs = Some(Self::build_docs(
                    resolved_package.source_package.package.name,
                    &model,
                    &resolved_package.package_path,
                    &dependencies,
                    &resolution_graph.build_options.install_dir,
                ));
            }

            if resolution_graph.build_options.generate_abis {
                compiled_abis = Some(Self::build_abis(&model, &compiled_units));
            }
        };

        let compiled_package = CompiledPackage {
            compiled_package_info: CompiledPackageInfo {
                package_name: resolved_package.source_package.package.name,
                address_alias_instantiation: resolved_package.resolution_table,
                module_resolution_metadata,
                source_digest: Some(resolved_package.source_digest),
                build_flags: resolution_graph.build_options.clone(),
            },
            compiled_units,
            compiled_docs,
            compiled_abis,
            dependencies,
        };

        compiled_package.save_to_disk(build_root_path)?;

        Ok((compiled_package, CompilationCachingStatus::Recompiled))
    }

    pub(crate) fn save_to_disk(&self, under_path: PathBuf) -> Result<OnDiskCompiledPackage> {
        let on_disk_package = OnDiskCompiledPackage {
            root_path: under_path.join(&self.compiled_package_info.package_name.to_string()),
            package: OnDiskPackage {
                compiled_package_info: self.compiled_package_info.clone(),
                dependencies: self
                    .dependencies
                    .iter()
                    .map(|dep| dep.compiled_package_info.package_name)
                    .collect(),
            },
        };

        // Clear out the build dir for this package so we don't keep artifacts from previous
        // compilations
        if on_disk_package.root_path.is_dir() {
            std::fs::remove_dir_all(&on_disk_package.root_path)?;
        }

        std::fs::create_dir_all(&on_disk_package.root_path)?;

        std::fs::create_dir_all(
            on_disk_package
                .root_path
                .join(CompiledPackageLayout::Sources.path()),
        )?;
        for compiled_unit in &self.compiled_units {
            on_disk_package.save_under(
                CompiledPackageLayout::Sources.path(),
                Some(PathBuf::from(
                    compiled_unit.source_path.file_name().unwrap(),
                )),
                std::fs::read_to_string(&compiled_unit.source_path)?.as_bytes(),
            )?;
        }

        std::fs::create_dir_all(
            on_disk_package
                .root_path
                .join(CompiledPackageLayout::CompiledScripts.path()),
        )?;
        std::fs::create_dir_all(
            on_disk_package
                .root_path
                .join(CompiledPackageLayout::CompiledModules.path()),
        )?;
        for compiled_unit in &self.compiled_units {
            let under_path = match &compiled_unit.unit {
                CompiledUnit::Script(_) => CompiledPackageLayout::CompiledScripts.path(),
                CompiledUnit::Module(_) => CompiledPackageLayout::CompiledModules.path(),
            };
            let path = match &compiled_unit.unit {
                CompiledUnit::Script(named) => named.name.as_str(),
                CompiledUnit::Module(named) => named.name.as_str(),
            };
            on_disk_package.save_under(
                under_path,
                Some(Path::new(path).with_extension(MOVE_COMPILED_EXTENSION)),
                compiled_unit.unit.serialize().as_slice(),
            )?;

            on_disk_package.save_under(
                CompiledPackageLayout::SourceMaps.path(),
                Some(Path::new(path).with_extension(SOURCE_MAP_EXTENSION)),
                compiled_unit.unit.serialize_source_map().as_slice(),
            )?;
        }

        if let Some(docs) = &self.compiled_docs {
            for (doc_filename, doc_contents) in docs {
                on_disk_package.save_under(
                    CompiledPackageLayout::CompiledDocs.path(),
                    Some(Path::new(&doc_filename).with_extension("md")),
                    doc_contents.clone().as_bytes(),
                )?;
            }
        }

        if let Some(abis) = &self.compiled_abis {
            for (filename, abi_bytes) in abis {
                on_disk_package.save_under(
                    CompiledPackageLayout::CompiledABIs.path(),
                    Some(Path::new(&filename).with_extension("abi")),
                    abi_bytes,
                )?;
            }
        }

        on_disk_package.save_under(
            CompiledPackageLayout::BuildInfo.path(),
            None,
            serde_yaml::to_string(&on_disk_package.package)?.as_bytes(),
        )?;

        Ok(on_disk_package)
    }

    fn apply_renaming(
        &self,
        module_resolution: &mut ModuleResolutionMetadata,
        renaming: &Renaming,
    ) {
        let package_renamings = renaming
            .iter()
            .filter_map(|(rename_to, (package_name, from_name))| {
                if package_name == &self.compiled_package_info.package_name {
                    Some((from_name, *rename_to))
                } else {
                    None
                }
            })
            .collect::<BTreeMap<_, _>>();

        for (module_id, ident) in self
            .compiled_package_info
            .module_resolution_metadata
            .clone()
            .into_iter()
        {
            match package_renamings.get(&ident) {
                Some(rename) => module_resolution.insert(module_id, *rename),
                None => module_resolution.insert(module_id, ident),
            };
        }
    }

    fn build_abis(
        model: &GlobalEnv,
        compiled_units: &[CompiledUnitWithSource],
    ) -> Vec<(String, Vec<u8>)> {
        let bytecode_map: BTreeMap<_, _> = compiled_units
            .iter()
            .map(|unit| match &unit.unit {
                CompiledUnit::Script(script) => (script.name.to_string(), unit.unit.serialize()),
                CompiledUnit::Module(module) => (module.name.to_string(), unit.unit.serialize()),
            })
            .collect();
        let abi_options = AbigenOptions {
            in_memory_bytes: Some(bytecode_map),
            output_directory: "".to_string(),
            ..AbigenOptions::default()
        };
        let mut abigen = Abigen::new(model, &abi_options);
        abigen.gen();
        abigen.into_result()
    }

    fn build_docs(
        package_name: PackageName,
        model: &GlobalEnv,
        package_root: &Path,
        deps: &[CompiledPackage],
        install_dir: &Option<PathBuf>,
    ) -> Vec<(String, String)> {
        let root_doc_templates = find_filenames(
            &[package_root
                .join(SourcePackageLayout::DocTemplates.path())
                .to_string_lossy()
                .to_string()],
            |path| extension_equals(path, "md"),
        )
        .unwrap_or_else(|_| vec![]);
        let root_for_docs = if let Some(install_dir) = install_dir {
            install_dir.join(CompiledPackageLayout::Root.path())
        } else {
            CompiledPackageLayout::Root.path().to_path_buf()
        };
        let dep_paths = deps
            .iter()
            .map(|dep| {
                root_for_docs
                    .join(dep.compiled_package_info.package_name.as_str())
                    .join(CompiledPackageLayout::CompiledDocs.path())
                    .to_string_lossy()
                    .to_string()
            })
            .collect();
        let in_pkg_doc_path = root_for_docs
            .join(package_name.as_str())
            .join(CompiledPackageLayout::CompiledDocs.path());
        let references_path = package_root
            .join(SourcePackageLayout::DocTemplates.path())
            .join(REFERENCE_TEMPLATE_FILENAME);
        let references_file = if references_path.exists() {
            Some(references_path.to_string_lossy().to_string())
        } else {
            None
        };
        let doc_options = DocgenOptions {
            doc_path: dep_paths,
            output_directory: in_pkg_doc_path.to_string_lossy().to_string(),
            root_doc_templates,
            compile_relative_to_output_dir: true,
            references_file,
            ..DocgenOptions::default()
        };
        let docgen = Docgen::new(model, &doc_options);
        docgen.gen()
    }
}
