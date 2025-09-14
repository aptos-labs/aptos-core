// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    compilation::{build_plan::CompilerDriverResult, package_layout::CompiledPackageLayout},
    resolution::resolution_graph::{Renaming, ResolvedGraph, ResolvedPackage, ResolvedTable},
    source_package::{
        layout::{SourcePackageLayout, REFERENCE_TEMPLATE_FILENAME},
        parsed_manifest::{FileName, PackageDigest, PackageName},
    },
    BuildConfig, CompilerConfig, CompilerVersion,
};
use anyhow::{bail, ensure, Context, Result};
use colored::Colorize;
use itertools::{Either, Itertools};
use legacy_move_compiler::{
    compiled_unit::{self, CompiledUnit, NamedCompiledModule, NamedCompiledScript},
    shared::{
        known_attributes::{AttributeKind, KnownAttribute},
        Flags, NamedAddressMap, NumericalAddress, PackagePaths,
    },
};
use move_abigen::{Abigen, AbigenOptions};
use move_binary_format::file_format::{CompiledModule, CompiledScript};
use move_bytecode_source_map::utils::source_map_from_file;
use move_bytecode_utils::Modules;
use move_command_line_common::files::{
    extension_equals, find_filenames, MOVE_COMPILED_EXTENSION, MOVE_EXTENSION, SOURCE_MAP_EXTENSION,
};
use move_compiler_v2::{external_checks::ExternalChecks, Experiment};
use move_docgen::{Docgen, DocgenOptions};
use move_model::model::GlobalEnv;
use move_symbol_pool::Symbol;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
};
use termcolor::{ColorChoice, StandardStream};

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
    /// The hash of the source directory at the time of compilation. `None` if the source for this
    /// package is not available/this package was not compiled.
    pub source_digest: Option<PackageDigest>,
    /// The build flags that were used when compiling this package.
    pub build_flags: BuildConfig,
}

/// Represents a compiled package in memory.
#[derive(Debug)]
pub struct CompiledPackage {
    /// Meta information about the compilation of this `CompiledPackage`
    pub compiled_package_info: CompiledPackageInfo,
    /// The output compiled bytecode in the root package (both module, and scripts) along with its
    /// source file
    pub root_compiled_units: Vec<CompiledUnitWithSource>,
    /// The output compiled bytecode for dependencies
    pub deps_compiled_units: Vec<(PackageName, CompiledUnitWithSource)>,
    /// Bytecode dependencies of this compiled package
    pub bytecode_deps: BTreeMap<PackageName, CompiledModule>,

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
    pub dependencies: Vec<PackageName>,
    pub bytecode_deps: Vec<PackageName>,
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
        let (buf, build_path) = if p.exists() && extension_equals(p, "yaml") {
            (std::fs::read(p)?, p.parent().unwrap().parent().unwrap())
        } else {
            (
                std::fs::read(p.join(CompiledPackageLayout::BuildInfo.path()))?,
                p.parent().unwrap(),
            )
        };
        let package = serde_yaml::from_slice::<OnDiskPackage>(&buf)?;
        assert!(build_path.ends_with(CompiledPackageLayout::Root.path()));
        let root_path = build_path.join(package.compiled_package_info.package_name.as_str());
        Ok(Self { root_path, package })
    }

    pub fn into_compiled_package(&self) -> Result<CompiledPackage> {
        let root_name = self.package.compiled_package_info.package_name;
        assert!(self.root_path.ends_with(root_name.as_str()));
        let root_compiled_units = self.get_compiled_units_paths(root_name)?;
        let root_compiled_units = root_compiled_units
            .into_iter()
            .map(|bytecode_path| self.decode_unit(root_name, &bytecode_path))
            .collect::<Result<Vec<_>>>()?;
        let mut deps_compiled_units = vec![];
        for dep_name in self.package.dependencies.iter().copied() {
            let compiled_units = self.get_compiled_units_paths(dep_name)?;
            for bytecode_path in compiled_units {
                deps_compiled_units.push((dep_name, self.decode_unit(dep_name, &bytecode_path)?))
            }
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
            root_compiled_units,
            deps_compiled_units,
            // TODO: support bytecode deps
            bytecode_deps: BTreeMap::new(),
            compiled_docs,
            compiled_abis,
        })
    }

    fn decode_unit(
        &self,
        package_name: Symbol,
        bytecode_path_str: &str,
    ) -> Result<CompiledUnitWithSource> {
        let package_name_opt = Some(package_name);
        let bytecode_path = Path::new(bytecode_path_str);
        let path_to_file = CompiledPackageLayout::path_to_file_after_category(bytecode_path);
        let bytecode_bytes = std::fs::read(bytecode_path)?;
        let source_map = source_map_from_file(
            &self
                .root_path
                .join(CompiledPackageLayout::SourceMaps.path())
                .join(&path_to_file)
                .with_extension(SOURCE_MAP_EXTENSION),
        )?;
        let source_path = self
            .root_path
            .join(CompiledPackageLayout::Sources.path())
            .join(path_to_file)
            .with_extension(MOVE_EXTENSION);
        ensure!(
            source_path.is_file(),
            "Error decoding package: {}. \
            Unable to find corresponding source file for '{}' in package {}",
            self.package.compiled_package_info.package_name,
            bytecode_path_str,
            package_name
        );
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
                    package_name: package_name_opt,
                    name,
                    script,
                    source_map,
                });
                Ok(CompiledUnitWithSource { unit, source_path })
            },
            Err(_) => {
                let module = CompiledModule::deserialize(&bytecode_bytes)?;
                let (address_bytes, module_name) = {
                    let id = module.self_id();
                    let parsed_addr = NumericalAddress::new(
                        id.address().into_bytes(),
                        legacy_move_compiler::shared::NumberFormat::Hex,
                    );
                    let module_name = FileName::from(id.name().as_str());
                    (parsed_addr, module_name)
                };
                let unit = CompiledUnit::Module(NamedCompiledModule {
                    package_name: package_name_opt,
                    address: address_bytes,
                    name: module_name,
                    module,
                    source_map,
                });
                Ok(CompiledUnitWithSource { unit, source_path })
            },
        }
    }

    /// Save `bytes` under `path_under` relative to the package on disk
    pub(crate) fn save_under(&self, file: impl AsRef<Path>, bytes: &[u8]) -> Result<()> {
        let path_to_save = self.root_path.join(file);
        let parent = path_to_save.parent().unwrap();
        std::fs::create_dir_all(parent)?;
        std::fs::write(path_to_save, bytes).map_err(|err| err.into())
    }

    #[allow(unused)]
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

    #[allow(unused)]
    pub(crate) fn are_build_flags_different(&self, build_config: &BuildConfig) -> bool {
        build_config != &self.package.compiled_package_info.build_flags
    }

    fn get_compiled_units_paths(&self, package_name: Symbol) -> Result<Vec<String>> {
        let package_dir = if self.package.compiled_package_info.package_name == package_name {
            self.root_path.clone()
        } else {
            self.root_path
                .join(CompiledPackageLayout::Dependencies.path())
                .join(package_name.as_str())
        };
        let mut compiled_unit_paths = vec![];
        let module_path = package_dir.join(CompiledPackageLayout::CompiledModules.path());
        if module_path.exists() {
            compiled_unit_paths.push(module_path);
        }
        let script_path = package_dir.join(CompiledPackageLayout::CompiledScripts.path());
        if script_path.exists() {
            compiled_unit_paths.push(script_path);
        }
        find_filenames(&compiled_unit_paths, |path| {
            extension_equals(path, MOVE_COMPILED_EXTENSION)
        })
    }

    fn save_compiled_unit(
        &self,
        package_name: Symbol,
        compiled_unit: &CompiledUnitWithSource,
        bytecode_version: u32,
    ) -> Result<()> {
        let root_package = self.package.compiled_package_info.package_name;
        assert!(self.root_path.ends_with(root_package.as_str()));
        let category_dir = match &compiled_unit.unit {
            CompiledUnit::Script(_) => CompiledPackageLayout::CompiledScripts.path(),
            CompiledUnit::Module(_) => CompiledPackageLayout::CompiledModules.path(),
        };
        let file_path = if root_package == package_name {
            PathBuf::new()
        } else {
            CompiledPackageLayout::Dependencies
                .path()
                .join(package_name.as_str())
        }
        .join(match &compiled_unit.unit {
            CompiledUnit::Script(named) => named.name.as_str(),
            CompiledUnit::Module(named) => named.name.as_str(),
        });

        self.save_under(
            category_dir
                .join(&file_path)
                .with_extension(MOVE_COMPILED_EXTENSION),
            compiled_unit
                .unit
                .serialize(Some(bytecode_version))
                .as_slice(),
        )?;
        self.save_under(
            CompiledPackageLayout::SourceMaps
                .path()
                .join(&file_path)
                .with_extension(SOURCE_MAP_EXTENSION),
            compiled_unit.unit.serialize_source_map().as_slice(),
        )?;
        self.save_under(
            CompiledPackageLayout::Sources
                .path()
                .join(&file_path)
                .with_extension(MOVE_EXTENSION),
            std::fs::read_to_string(&compiled_unit.source_path)?.as_bytes(),
        )
    }
}

impl CompiledPackage {
    /// Returns all compiled units with sources for this package in transitive dependencies. Order
    /// is not guaranteed.
    pub fn all_compiled_units_with_source(&self) -> impl Iterator<Item = &CompiledUnitWithSource> {
        self.root_compiled_units
            .iter()
            .chain(self.deps_compiled_units.iter().map(|(_, unit)| unit))
    }

    /// Returns all compiled units for this package in transitive dependencies. Order is not
    /// guaranteed.
    pub fn all_compiled_units(&self) -> impl Iterator<Item = &CompiledUnit> {
        self.all_compiled_units_with_source().map(|unit| &unit.unit)
    }

    /// Returns compiled modules for this package and its transitive dependencies
    pub fn all_modules_map(&self) -> Modules<'_> {
        Modules::new(
            self.all_compiled_units()
                .filter_map(|unit| match unit {
                    CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                    CompiledUnit::Script(_) => None,
                })
                .collect::<Vec<_>>(),
        )
    }

    pub fn root_modules_map(&self) -> Modules<'_> {
        Modules::new(
            self.root_compiled_units
                .iter()
                .filter_map(|unit| match &unit.unit {
                    CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                    CompiledUnit::Script(_) => None,
                }),
        )
    }

    /// `all_compiled_units_with_source` filtered over `CompiledUnit::Module`
    pub fn all_modules(&self) -> impl Iterator<Item = &CompiledUnitWithSource> {
        self.all_compiled_units_with_source()
            .filter(|unit| matches!(unit.unit, CompiledUnit::Module(_)))
    }

    /// `root_compiled_units` filtered over `CompiledUnit::Module`
    pub fn root_modules(&self) -> impl Iterator<Item = &CompiledUnitWithSource> {
        self.root_compiled_units
            .iter()
            .filter(|unit| matches!(unit.unit, CompiledUnit::Module(_)))
    }

    pub fn get_module_by_name(
        &self,
        package_name: &str,
        module_name: &str,
    ) -> Result<&CompiledUnitWithSource> {
        if self.compiled_package_info.package_name.as_str() == package_name {
            return self.get_module_by_name_from_root(module_name);
        }

        self.deps_compiled_units
            .iter()
            .filter(|(dep_package, unit)| {
                dep_package.as_str() == package_name && matches!(unit.unit, CompiledUnit::Module(_))
            })
            .map(|(_, unit)| unit)
            .find(|unit| unit.unit.name().as_str() == module_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find module with name '{}' in package {}",
                    module_name,
                    self.compiled_package_info.package_name
                )
            })
    }

    pub fn get_script_by_name(
        &self,
        package_name: &str,
        script_name: &str,
    ) -> Result<&CompiledUnitWithSource> {
        if self.compiled_package_info.package_name.as_str() == package_name {
            return self.get_script_by_name_from_root(script_name);
        }

        self.deps_compiled_units
            .iter()
            .filter(|(dep_package, unit)| {
                dep_package.as_str() == package_name && matches!(unit.unit, CompiledUnit::Script(_))
            })
            .map(|(_, unit)| unit)
            .find(|unit| unit.unit.name().as_str() == script_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find script with name '{}' in package {}",
                    script_name,
                    self.compiled_package_info.package_name
                )
            })
    }

    pub fn get_module_by_name_from_root(
        &self,
        module_name: &str,
    ) -> Result<&CompiledUnitWithSource> {
        self.root_modules()
            .find(|unit| unit.unit.name().as_str() == module_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find module with name '{}' in package {}",
                    module_name,
                    self.compiled_package_info.package_name
                )
            })
    }

    pub fn get_script_by_name_from_root(
        &self,
        script_name: &str,
    ) -> Result<&CompiledUnitWithSource> {
        self.scripts()
            .find(|unit| unit.unit.name().as_str() == script_name)
            .ok_or_else(|| {
                anyhow::format_err!(
                    "Unable to find script with name '{}' in package {}",
                    script_name,
                    self.compiled_package_info.package_name
                )
            })
    }

    pub fn scripts(&self) -> impl Iterator<Item = &CompiledUnitWithSource> {
        self.root_compiled_units
            .iter()
            .filter(|unit| matches!(unit.unit, CompiledUnit::Script(_)))
    }

    #[allow(unused)]
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

    pub(crate) fn build_all<W: Write>(
        w: &mut W,
        project_root: &Path,
        resolved_package: ResolvedPackage,
        transitive_dependencies: Vec<(
            /* name */ Symbol,
            /* is immediate */ bool,
            /* source paths */ Vec<Symbol>,
            /* address mapping */ &ResolvedTable,
            /* whether source is available */ bool,
        )>,
        config: &CompilerConfig,
        external_checks: Vec<Arc<dyn ExternalChecks>>,
        resolution_graph: &ResolvedGraph,
        mut compiler_driver: impl FnMut(move_compiler_v2::Options) -> CompilerDriverResult,
    ) -> Result<(CompiledPackage, Option<GlobalEnv>)> {
        let immediate_dependencies = transitive_dependencies
            .iter()
            .filter(|(_, is_immediate, _, _, _)| *is_immediate)
            .map(|(name, _, _, _, _)| *name)
            .collect::<Vec<_>>();
        let transitive_dependencies = transitive_dependencies
            .into_iter()
            .map(
                |(name, _is_immediate, source_paths, address_mapping, src_flag)| {
                    (name, source_paths, address_mapping, src_flag)
                },
            )
            .collect::<Vec<_>>();
        let mut source_package_map: BTreeMap<String, Symbol> = BTreeMap::new();
        for (dep_package_name, source_paths, _, _) in &transitive_dependencies {
            for dep_path in source_paths.clone() {
                source_package_map.insert(dep_path.as_str().to_string(), *dep_package_name);
            }
            writeln!(
                w,
                "{} {}",
                "INCLUDING DEPENDENCY".bold().green(),
                dep_package_name
            )?;
        }
        let root_package_name = resolved_package.source_package.package.name;
        writeln!(w, "{} {}", "BUILDING".bold().green(), root_package_name)?;
        // gather source/dep files with their address mappings
        let (sources_package_paths, deps_package_paths) = make_source_and_deps_for_compiler(
            resolution_graph,
            &resolved_package,
            transitive_dependencies,
        )?;
        for source_path in &sources_package_paths.paths {
            source_package_map.insert(source_path.as_str().to_string(), root_package_name);
        }
        let mut flags = if resolution_graph.build_options.test_mode {
            Flags::testing()
        } else {
            Flags::empty()
        };
        let skip_attribute_checks = resolution_graph
            .build_options
            .compiler_config
            .skip_attribute_checks;
        flags = flags.set_skip_attribute_checks(skip_attribute_checks);
        let mut known_attributes = resolution_graph
            .build_options
            .compiler_config
            .known_attributes
            .clone();
        KnownAttribute::add_attribute_names(&mut known_attributes);

        // Partition deps_package according whether src is available
        let (src_deps, bytecode_deps): (Vec<_>, Vec<_>) = deps_package_paths
            .clone()
            .into_iter()
            .partition_map(|(p, b)| if b { Either::Left(p) } else { Either::Right(p) });
        // If bytecode dependency is not empty, do not allow renaming
        if !bytecode_deps.is_empty() {
            if let Some(pkg_name) = resolution_graph.contains_renaming() {
                bail!(
                    "Found address renaming in package '{}' when \
                    building with bytecode dependencies -- this is currently not supported",
                    pkg_name
                )
            }
        }

        // invoke the compiler
        let effective_compiler_version = config.compiler_version.unwrap_or_default();
        let effective_language_version = config.language_version.unwrap_or_default();
        effective_compiler_version.check_language_support(effective_language_version)?;

        let (file_map, all_compiled_units, model) =
            match config.compiler_version.unwrap_or_default() {
                CompilerVersion::V1 => anyhow::bail!("Compiler v1 is no longer supported"),
                version @ CompilerVersion::V2_0 | version @ CompilerVersion::V2_1 => {
                    let to_str_vec = |ps: &[Symbol]| {
                        ps.iter()
                            .map(move |s| s.as_str().to_owned())
                            .collect::<Vec<_>>()
                    };
                    let mut global_address_map = BTreeMap::new();
                    for pack in std::iter::once(&sources_package_paths)
                        .chain(src_deps.iter())
                        .chain(bytecode_deps.iter())
                    {
                        for (name, val) in &pack.named_address_map {
                            if let Some(old) =
                                global_address_map.insert(name.as_str().to_owned(), *val)
                            {
                                if old != *val {
                                    let pack_name = pack
                                        .name
                                        .map(|s| s.as_str().to_owned())
                                        .unwrap_or_else(|| "<unnamed>".to_owned());
                                    bail!(
                                    "found remapped address alias `{}` (`{} != {}`) in package `{}`\
                                    , please use unique address aliases across dependencies",
                                    name, old, val, pack_name
                                )
                                }
                            }
                        }
                    }
                    let mut options = move_compiler_v2::Options {
                        sources: sources_package_paths
                            .paths
                            .iter()
                            .map(|path| path.as_str().to_owned())
                            .collect(),
                        sources_deps: src_deps.iter().flat_map(|x| to_str_vec(&x.paths)).collect(),
                        dependencies: bytecode_deps
                            .iter()
                            .flat_map(|x| to_str_vec(&x.paths))
                            .collect(),
                        named_address_mapping: global_address_map
                            .into_iter()
                            .map(|(k, v)| format!("{}={}", k, v))
                            .collect(),
                        skip_attribute_checks,
                        known_attributes: known_attributes.clone(),
                        language_version: Some(effective_language_version),
                        compiler_version: Some(version),
                        compile_test_code: flags.keep_testing_functions(),
                        experiments: config.experiments.clone(),
                        external_checks,
                        ..Default::default()
                    };
                    options = options.set_experiment(Experiment::ATTACH_COMPILED_MODULE, true);
                    compiler_driver(options)?
                },
            };
        let mut root_compiled_units = vec![];
        let mut deps_compiled_units = vec![];
        let obtain_package_name =
            |default_opt: Option<Symbol>, source_path_str: &str| -> Result<Symbol> {
                if let Some(default) = default_opt {
                    Ok(default)
                } else if source_package_map.contains_key(source_path_str) {
                    Ok(*source_package_map.get(source_path_str).unwrap())
                } else {
                    Err(anyhow::anyhow!("package name is none"))
                }
            };
        for annot_unit in all_compiled_units {
            let source_path_str = file_map
                .get(&annot_unit.loc().file_hash())
                .ok_or_else(|| anyhow::anyhow!("invalid transaction script bytecode"))?
                .0
                .as_str();
            let source_path = PathBuf::from(source_path_str);
            let package_name = match &annot_unit {
                compiled_unit::CompiledUnitEnum::Module(m) => {
                    obtain_package_name(m.named_module.package_name, source_path_str)?
                },
                compiled_unit::CompiledUnitEnum::Script(s) => {
                    obtain_package_name(s.named_script.package_name, source_path_str)?
                },
            };
            let unit = CompiledUnitWithSource {
                unit: annot_unit.into_compiled_unit(),
                source_path,
            };
            if package_name == root_package_name {
                root_compiled_units.push(unit)
            } else {
                deps_compiled_units.push((package_name, unit))
            }
        }
        let bytecode_version = config
            .language_version
            .unwrap_or_default()
            .infer_bytecode_version(config.bytecode_version);
        let mut compiled_docs = None;
        let mut compiled_abis = None;
        let mut move_model = None;
        if resolution_graph.build_options.generate_docs
            || resolution_graph.build_options.generate_abis
            || resolution_graph.build_options.generate_move_model
        {
            if resolution_graph.build_options.generate_docs {
                compiled_docs = Some(Self::build_docs(
                    resolved_package.source_package.package.name,
                    &model,
                    &resolved_package.package_path,
                    &immediate_dependencies,
                    &resolution_graph.build_options.install_dir,
                ));
            }

            if resolution_graph.build_options.generate_abis {
                compiled_abis = Some(Self::build_abis(
                    bytecode_version,
                    &model,
                    &root_compiled_units,
                ));
            }

            if resolution_graph.build_options.generate_move_model {
                move_model = Some(model)
            }
        };

        Self::check_duplicate_script_function_names(&root_compiled_units)?;

        let compiled_package = CompiledPackage {
            root_compiled_units,
            deps_compiled_units,
            bytecode_deps: bytecode_deps
                .iter()
                .flat_map(|package| {
                    let name = package.name.unwrap();
                    package.paths.iter().map(move |pkg_path| {
                        get_module_in_package(name, pkg_path.as_str()).map(|module| (name, module))
                    })
                })
                .try_collect()?,
            compiled_package_info: CompiledPackageInfo {
                package_name: resolved_package.source_package.package.name,
                address_alias_instantiation: resolved_package.resolution_table,
                source_digest: Some(resolved_package.source_digest),
                build_flags: resolution_graph.build_options.clone(),
            },
            compiled_docs,
            compiled_abis,
        };

        compiled_package.save_to_disk(
            project_root.join(CompiledPackageLayout::Root.path()),
            bytecode_version,
        )?;

        Ok((compiled_package, move_model))
    }

    // We take the (restrictive) view that all filesystems are case insensitive to maximize
    // portability of packages.
    fn check_filepaths_ok(&self) -> Result<()> {
        // A mapping of (lowercase_name => [info_for_each_occurence]
        let mut insensitive_mapping = BTreeMap::new();
        for compiled_unit in &self.root_compiled_units {
            let is_module = matches!(&compiled_unit.unit, CompiledUnit::Module(_));
            let name = match &compiled_unit.unit {
                CompiledUnit::Script(named) => named.name.as_str(),
                CompiledUnit::Module(named) => named.name.as_str(),
            };
            let entry = insensitive_mapping
                .entry(name.to_lowercase())
                .or_insert_with(Vec::new);
            entry.push((
                name,
                is_module,
                compiled_unit.source_path.to_string_lossy().to_string(),
            ));
        }
        let errs = insensitive_mapping
            .into_iter()
            .filter_map(|(insensitive_name, occurence_infos)| {
                if occurence_infos.len() > 1 {
                    let name_conflict_error_msg = occurence_infos
                        .into_iter()
                        .map(|(name, is_module, fpath)| {
                            format!(
                                "\t{} '{}' at path '{}'",
                                if is_module { "Module" } else { "Script" },
                                name,
                                fpath
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    Some(format!(
                        "The following modules and/or scripts would collide as '{}' on the file system:\n{}",
                        insensitive_name, name_conflict_error_msg
                    ))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        if !errs.is_empty() {
            anyhow::bail!("Module and/or script names found that would cause failures on case insensitive \
                file systems when compiling package '{}':\n{}\nPlease rename these scripts and/or modules to resolve these conflicts.",
                self.compiled_package_info.package_name,
                errs.join("\n"),
            )
        }
        Ok(())
    }

    pub(crate) fn save_to_disk(
        &self,
        under_path: PathBuf,
        bytecode_version: u32,
    ) -> Result<OnDiskCompiledPackage> {
        self.check_filepaths_ok()?;
        assert!(under_path.ends_with(CompiledPackageLayout::Root.path()));
        let root_package = self.compiled_package_info.package_name;
        let on_disk_package = OnDiskCompiledPackage {
            root_path: under_path.join(root_package.as_str()),
            package: OnDiskPackage {
                compiled_package_info: self.compiled_package_info.clone(),
                dependencies: self
                    .deps_compiled_units
                    .iter()
                    .map(|(package_name, _)| *package_name)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                bytecode_deps: self
                    .bytecode_deps
                    .keys()
                    .copied()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            },
        };

        // Clear out the build dir for this package so we don't keep artifacts from previous
        // compilations
        if on_disk_package.root_path.is_dir() {
            std::fs::remove_dir_all(&on_disk_package.root_path)?;
        }

        std::fs::create_dir_all(&on_disk_package.root_path)?;

        for compiled_unit in &self.root_compiled_units {
            on_disk_package.save_compiled_unit(root_package, compiled_unit, bytecode_version)?;
        }
        for (dep_name, compiled_unit) in &self.deps_compiled_units {
            on_disk_package.save_compiled_unit(*dep_name, compiled_unit, bytecode_version)?;
        }

        if let Some(docs) = &self.compiled_docs {
            for (doc_filename, doc_contents) in docs {
                on_disk_package.save_under(
                    CompiledPackageLayout::CompiledDocs
                        .path()
                        .join(doc_filename)
                        .with_extension("md"),
                    doc_contents.clone().as_bytes(),
                )?;
            }
        }

        if let Some(abis) = &self.compiled_abis {
            for (filename, abi_bytes) in abis {
                on_disk_package.save_under(
                    CompiledPackageLayout::CompiledABIs
                        .path()
                        .join(filename)
                        .with_extension("abi"),
                    abi_bytes,
                )?;
            }
        }

        on_disk_package.save_under(
            CompiledPackageLayout::BuildInfo.path(),
            serde_yaml::to_string(&on_disk_package.package)?.as_bytes(),
        )?;

        Ok(on_disk_package)
    }

    fn check_duplicate_script_function_names(units: &[CompiledUnitWithSource]) -> Result<()> {
        let mut seen_names: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut seen_error = false;
        for unit in units {
            if let CompiledUnit::Script(named) = &unit.unit {
                let name = named.name.as_str().to_owned();
                let script_path = unit.source_path.to_string_lossy().to_string();
                let entry = seen_names.entry(name).or_default();
                entry.push(script_path);
                if entry.len() > 1 {
                    seen_error = true;
                }
            }
        }
        if !seen_error {
            return Ok(());
        }
        let mut error_strs = vec![];
        for (script_name, paths) in seen_names.into_iter() {
            if paths.len() > 1 {
                error_strs.push(format!(
                    "Script function name `{}` duplicated in the following files:\n{}",
                    script_name,
                    paths
                        .iter()
                        .map(|path| format!("\t`{}`", path))
                        .collect::<Vec<_>>()
                        .join("\n")
                ));
            }
        }
        error_strs.push("Please rename script functions to remove duplication".to_string());
        bail!(error_strs.join("\n"));
    }

    fn build_abis(
        bytecode_version: u32,
        model: &GlobalEnv,
        compiled_units: &[CompiledUnitWithSource],
    ) -> Vec<(String, Vec<u8>)> {
        let bytecode_map: BTreeMap<_, _> = compiled_units
            .iter()
            .map(|unit| match &unit.unit {
                CompiledUnit::Script(script) => (
                    script.name.to_string(),
                    unit.unit.serialize(Some(bytecode_version)),
                ),
                CompiledUnit::Module(module) => (
                    module.name.to_string(),
                    unit.unit.serialize(Some(bytecode_version)),
                ),
            })
            .collect();
        let abi_options = AbigenOptions {
            in_memory_bytes: Some(bytecode_map),
            output_directory: "".to_string(),
            ..AbigenOptions::default()
        };
        let mut abigen = Abigen::new(model, &abi_options);
        abigen.r#gen();
        abigen.into_result()
    }

    fn build_docs(
        package_name: PackageName,
        model: &GlobalEnv,
        package_root: &Path,
        deps: &[PackageName],
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
            .map(|dep_name| {
                root_for_docs
                    .join(dep_name.as_str())
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
        docgen.generate()
    }
}

pub(crate) fn named_address_mapping_for_compiler(
    resolution_table: &ResolvedTable,
) -> BTreeMap<Symbol, NumericalAddress> {
    resolution_table
        .iter()
        .map(|(ident, addr)| {
            let parsed_addr = NumericalAddress::new(
                addr.into_bytes(),
                legacy_move_compiler::shared::NumberFormat::Hex,
            );
            (*ident, parsed_addr)
        })
        .collect::<BTreeMap<_, _>>()
}

pub(crate) fn apply_named_address_renaming(
    current_package_name: Symbol,
    address_resolution: BTreeMap<Symbol, NumericalAddress>,
    renaming: &Renaming,
) -> NamedAddressMap {
    let package_renamings = renaming
        .iter()
        .filter_map(|(rename_to, (package_name, from_name))| {
            if package_name == &current_package_name {
                Some((from_name, *rename_to))
            } else {
                None
            }
        })
        .collect::<BTreeMap<_, _>>();

    address_resolution
        .into_iter()
        .map(|(name, value)| {
            let new_name = package_renamings.get(&name).copied();
            (new_name.unwrap_or(name), value)
        })
        .collect()
}

/// Collects source and dependency files with their address mappings.
pub fn make_source_and_deps_for_compiler(
    resolution_graph: &ResolvedGraph,
    root: &ResolvedPackage,
    deps: Vec<(
        /* name */ Symbol,
        /* source paths */ Vec<Symbol>,
        /* address mapping */ &ResolvedTable,
        /* whether src is available */ bool,
    )>,
) -> Result<(
    /* sources */ PackagePaths,
    /* deps */ Vec<(PackagePaths, bool)>,
)> {
    let deps_package_paths = deps
        .into_iter()
        .map(|(name, source_paths, resolved_table, src_flag)| {
            let paths = source_paths
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let named_address_map = named_address_mapping_for_compiler(resolved_table);
            Ok((
                PackagePaths {
                    name: Some(name),
                    paths,
                    named_address_map,
                },
                src_flag,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let root_named_addrs = apply_named_address_renaming(
        root.source_package.package.name,
        named_address_mapping_for_compiler(&root.resolution_table),
        &root.renaming,
    );
    let sources = root.get_sources(&resolution_graph.build_options)?;
    let source_package_paths = PackagePaths {
        name: Some(root.source_package.package.name),
        paths: sources,
        named_address_map: root_named_addrs,
    };
    Ok((source_package_paths, deps_package_paths))
}

/// Stub which can be used if the v2 compiler driver for a particular operation is not available.
pub fn unimplemented_v2_driver(_options: move_compiler_v2::Options) -> CompilerDriverResult {
    anyhow::bail!("v2 compiler requested but driver not available")
}

/// Runs the v2 compiler, exiting the process if any errors occurred.
pub fn build_and_report_v2_driver(options: move_compiler_v2::Options) -> CompilerDriverResult {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let mut emitter = options.error_emitter(&mut stderr);
    match move_compiler_v2::run_move_compiler(emitter.as_mut(), options) {
        Ok((env, units)) => Ok((move_compiler_v2::make_files_source_text(&env), units, env)),
        Err(_) => {
            // Error reported, exit
            std::process::exit(1);
        },
    }
}

/// Runs the v2 compiler, reporting errors and returning an error if compilation fails.
pub fn build_and_report_no_exit_v2_driver(
    options: move_compiler_v2::Options,
) -> CompilerDriverResult {
    let mut stderr = StandardStream::stderr(ColorChoice::Auto);
    let mut emitter = options.error_emitter(&mut stderr);
    let (env, units) = move_compiler_v2::run_move_compiler(emitter.as_mut(), options)?;
    Ok((move_compiler_v2::make_files_source_text(&env), units, env))
}

/// Returns the deserialized module from the bytecode file
fn get_module_in_package(pkg_name: Symbol, pkg_path: &str) -> Result<CompiledModule> {
    // Read the bytecode file
    let mut bytecode = Vec::new();
    std::fs::File::open(pkg_path)
        .context(format!("Failed to open bytecode file for {}", pkg_path))
        .and_then(|mut file| {
            // read contents of the file into bytecode
            std::io::Read::read_to_end(&mut file, &mut bytecode)
                .context(format!("Failed to read bytecode file {}", pkg_path))
        })
        .and_then(|_| {
            CompiledModule::deserialize(&bytecode).context(format!(
                "Failed to deserialize bytecode file for {}",
                pkg_name
            ))
        })
}
