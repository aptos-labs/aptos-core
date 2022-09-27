// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error_map::generate_error_map;
use crate::natives::code::{
    ModuleMetadata, MoveOption, PackageDep, PackageMetadata, UpgradePolicy,
};
use crate::{zip_metadata, zip_metadata_str, RuntimeModuleMetadata, APTOS_METADATA_KEY};
use aptos_module_verifier::module_init::verify_module_init_function;
use aptos_types::account_address::AccountAddress;
use aptos_types::transaction::EntryABI;
use clap::Parser;
use move_deps::move_binary_format::CompiledModule;
use move_deps::move_command_line_common::files::MOVE_COMPILED_EXTENSION;
use move_deps::move_compiler::compiled_unit::{CompiledUnit, NamedCompiledModule};
use move_deps::move_core_types::errmap::ErrorMapping;
use move_deps::move_core_types::metadata::Metadata;
use move_deps::move_package::compilation::compiled_package::CompiledPackage;
use move_deps::move_package::compilation::package_layout::CompiledPackageLayout;
use move_deps::move_package::source_package::manifest_parser::{
    parse_move_manifest_string, parse_source_manifest,
};
use move_deps::move_package::BuildConfig;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const METADATA_FILE_NAME: &str = "package-metadata.bcs";
pub const UPGRADE_POLICY_CUSTOM_FIELD: &str = "upgrade_policy";

/// Represents a set of options for building artifacts from Move.
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct BuildOptions {
    #[clap(long)]
    pub with_srcs: bool,
    #[clap(long)]
    pub with_abis: bool,
    #[clap(long)]
    pub with_source_maps: bool,
    #[clap(long, default_value = "true")]
    pub with_error_map: bool,
    /// Installation directory for compiled artifacts. Defaults to <package>/build.
    #[clap(long, parse(from_os_str))]
    pub install_dir: Option<PathBuf>,
    #[clap(skip)] // TODO: have a parser for this; there is one in the CLI buts its  downstream
    pub named_addresses: BTreeMap<String, AccountAddress>,
}

// Because named_addresses as no parser, we can't use clap's default impl. This must be aligned
// with defaults above.
impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            with_srcs: false,
            with_abis: false,
            with_source_maps: false,
            with_error_map: true,
            install_dir: None,
            named_addresses: Default::default(),
        }
    }
}

/// Represents a built package.  It allows to extract `PackageMetadata`. Can also be used to
/// just build Move code.
pub struct BuiltPackage {
    options: BuildOptions,
    package_path: PathBuf,
    package: CompiledPackage,
}

impl BuiltPackage {
    /// Builds the package and on success delivers a `BuiltPackage`.
    ///
    /// This function currently reports all Move compilation errors and warnings to stdout,
    /// and is not `Ok` if there was an error among those.
    pub fn build(package_path: PathBuf, options: BuildOptions) -> anyhow::Result<Self> {
        let build_config = BuildConfig {
            dev_mode: false,
            additional_named_addresses: options.named_addresses.clone(),
            architecture: None,
            generate_abis: options.with_abis,
            generate_docs: false,
            install_dir: options.install_dir.clone(),
            test_mode: false,
            force_recompilation: false,
            fetch_deps_only: false,
            fetch_latest_git_deps: false,
        };
        let mut package = build_config.compile_package_no_exit(&package_path, &mut Vec::new())?;
        for module in package.root_modules_map().iter_modules().iter() {
            verify_module_init_function(module)?;
        }
        let error_map = if options.with_error_map {
            generate_error_map(&package_path, &options)
        } else {
            None
        };
        if let Some(map) = &error_map {
            inject_module_metadata(
                package_path
                    .join(CompiledPackageLayout::Root.path())
                    .join(package.compiled_package_info.package_name.as_str()),
                &mut package,
                map,
            )?
        }
        Ok(Self {
            options,
            package_path,
            package,
        })
    }

    /// Returns the name of this package.
    pub fn name(&self) -> &str {
        self.package.compiled_package_info.package_name.as_str()
    }

    pub fn package_path(&self) -> &Path {
        self.package_path.as_path()
    }

    pub fn package_artifacts_path(&self) -> PathBuf {
        self.package_path
            .join(CompiledPackageLayout::Root.path())
            .join(self.name())
    }

    /// Extracts the bytecode for the modules of the built package.
    pub fn extract_code(&self) -> Vec<Vec<u8>> {
        self.package
            .root_modules()
            .map(|unit_with_source| unit_with_source.unit.serialize(None))
            .collect()
    }

    /// Returns the abis for this package, if available.
    pub fn extract_abis(&self) -> Option<Vec<EntryABI>> {
        self.package.compiled_abis.as_ref().map(|abis| {
            abis.iter()
                .map(|(_, bytes)| bcs::from_bytes::<EntryABI>(bytes.as_slice()).unwrap())
                .collect()
        })
    }

    /// Returns an iterator for all compiled proper (non-script) modules.
    pub fn modules(&self) -> impl Iterator<Item = &CompiledModule> {
        self.package
            .root_modules()
            .filter_map(|unit| match &unit.unit {
                CompiledUnit::Module(NamedCompiledModule { module, .. }) => Some(module),
                CompiledUnit::Script(_) => None,
            })
    }

    /// Returns the number of scripts in the package.
    pub fn script_count(&self) -> usize {
        self.package.scripts().count()
    }

    /// Returns the serialized bytecode of the scripts in the package.
    pub fn extract_script_code(&self) -> Vec<Vec<u8>> {
        self.package
            .scripts()
            .map(|unit_with_source| unit_with_source.unit.serialize(None))
            .collect()
    }

    /// Extracts metadata, as needed for releasing a package, from the built package.
    pub fn extract_metadata(&self) -> anyhow::Result<PackageMetadata> {
        let source_digest = self
            .package
            .compiled_package_info
            .source_digest
            .map(|s| s.to_string())
            .unwrap_or_default();
        let manifest_file = self.package_path.join("Move.toml");
        let manifest = std::fs::read_to_string(&manifest_file)?;
        let custom_props = extract_custom_fields(&manifest)?;
        let manifest = zip_metadata_str(&manifest)?;
        let upgrade_policy = if let Some(val) = custom_props.get(UPGRADE_POLICY_CUSTOM_FIELD) {
            str::parse::<UpgradePolicy>(val.as_ref())?
        } else {
            UpgradePolicy::compat()
        };
        let mut modules = vec![];
        for u in self.package.root_modules() {
            let name = u.unit.name().to_string();
            let source = if self.options.with_srcs {
                zip_metadata_str(&std::fs::read_to_string(&u.source_path)?)?
            } else {
                vec![]
            };
            let source_map = if self.options.with_source_maps {
                zip_metadata(&u.unit.serialize_source_map())?
            } else {
                vec![]
            };
            modules.push(ModuleMetadata {
                name,
                source,
                source_map,
                extension: MoveOption::default(),
            })
        }
        let deps = self
            .package
            .deps_compiled_units
            .iter()
            .map(|(name, unit)| {
                let package_name = name.as_str().to_string();
                let account = match &unit.unit {
                    CompiledUnit::Module(m) => AccountAddress::new(m.address.into_bytes()),
                    _ => panic!("script not a dependency"),
                };
                PackageDep {
                    account,
                    package_name,
                }
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(PackageMetadata {
            name: self.name().to_string(),
            upgrade_policy,
            upgrade_number: 0,
            source_digest,
            manifest,
            modules,
            deps,
            extension: MoveOption::none(),
        })
    }

    pub fn extract_metadata_and_save(&self) -> anyhow::Result<()> {
        let data = self.extract_metadata()?;
        let path = self.package_artifacts_path();
        std::fs::create_dir_all(&path)?;
        std::fs::write(path.join(METADATA_FILE_NAME), &bcs::to_bytes(&data)?)?;
        Ok(())
    }
}

fn extract_custom_fields(toml: &str) -> anyhow::Result<BTreeMap<String, String>> {
    let manifest = parse_source_manifest(parse_move_manifest_string(toml.to_owned())?)?;
    Ok(manifest
        .package
        .custom_properties
        .iter()
        .map(|(s, v)| (s.to_string(), v.to_string()))
        .collect())
}

fn inject_module_metadata(
    package_path: PathBuf,
    pack: &mut CompiledPackage,
    error_map: &ErrorMapping,
) -> anyhow::Result<()> {
    for unit_with_source in pack.root_compiled_units.iter_mut() {
        match &mut unit_with_source.unit {
            CompiledUnit::Module(named_module) => {
                if let Some(module_map) = error_map
                    .module_error_maps
                    .get(&named_module.module.self_id())
                {
                    if !module_map.is_empty() {
                        let serialized_metadata = bcs::to_bytes(&RuntimeModuleMetadata {
                            error_map: module_map.clone(),
                        })
                        .expect("BCS for RuntimeModuleMetadata");
                        named_module.module.metadata.push(Metadata {
                            key: APTOS_METADATA_KEY.clone(),
                            value: serialized_metadata,
                        });

                        // Also need to update the .mv file on disk.
                        let path = package_path
                            .join(CompiledPackageLayout::CompiledModules.path())
                            .join(named_module.name.as_str())
                            .with_extension(MOVE_COMPILED_EXTENSION);
                        if path.is_file() {
                            let bytes = unit_with_source.unit.serialize(None);
                            std::fs::write(path, &bytes)?;
                        }
                    }
                }
            }
            CompiledUnit::Script(_) => {}
        }
    }
    Ok(())
}
