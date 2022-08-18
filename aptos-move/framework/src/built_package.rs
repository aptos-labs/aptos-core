// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::error_map::generate_error_map;
use crate::natives::code::{ModuleMetadata, PackageMetadata, UpgradePolicy};
use crate::zip_metadata;
use aptos_types::account_address::AccountAddress;
use clap::Parser;
use move_deps::move_core_types::errmap::ErrorMapping;
use move_deps::move_package::compilation::compiled_package::CompiledPackage;
use move_deps::move_package::source_package::manifest_parser::{
    parse_move_manifest_string, parse_source_manifest,
};
use move_deps::move_package::BuildConfig;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use std::collections::BTreeMap;
use std::path::PathBuf;

pub const UPGRADE_POLICY_CUSTOM_FIELD: &str = "upgrade_policy";

/// Represents a set of options for building artifacts from Move.
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct BuildOptions {
    #[clap(long, default_value = "true")]
    pub with_srcs: bool,
    #[clap(long, default_value = "true")]
    pub with_abis: bool,
    #[clap(long, default_value = "true")]
    pub with_source_maps: bool,
    #[clap(long, default_value = "true")]
    pub with_error_map: bool,
    #[clap(skip)] // TODO: have a parser for this; there is one in the CLI buts its  downstream
    pub named_addresses: BTreeMap<String, AccountAddress>,
}

// Because named_addresses as no parser, we can't use clap's default impl. This must be aligned
// with defaults above.
impl Default for BuildOptions {
    fn default() -> Self {
        Self {
            with_srcs: true,
            with_abis: true,
            with_source_maps: true,
            with_error_map: true,
            named_addresses: Default::default(),
        }
    }
}

/// Represents a built package.  It allows to extract `PackageMetadata`. Can also be used to
/// just build Move code.
pub struct BuiltPackage {
    package_path: PathBuf,
    package: CompiledPackage,
    error_map: Option<ErrorMapping>,
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
            install_dir: None,
            test_mode: false,
            force_recompilation: false,
            fetch_deps_only: false,
        };
        let package = build_config.compile_package_no_exit(&package_path, &mut Vec::new())?;
        let error_map = if options.with_error_map {
            generate_error_map(&package_path, &options)
        } else {
            None
        };
        Ok(Self {
            package_path,
            package,
            error_map,
        })
    }

    /// Returns the name of this package.
    pub fn name(&self) -> &str {
        self.package.compiled_package_info.package_name.as_str()
    }

    /// Extracts the bytecode from the built package.
    pub fn extract_code(&self) -> Vec<Vec<u8>> {
        self.package
            .root_compiled_units
            .iter()
            .map(|unit_with_source| unit_with_source.unit.serialize(None))
            .collect()
    }

    /// Extracts metadata, as needed for releasing a package, from the built package.
    pub fn extract_metadata(&self) -> anyhow::Result<PackageMetadata> {
        let build_info = serde_yaml::to_string(&self.package.compiled_package_info)?;

        let manifest_file = self.package_path.join("Move.toml");
        let manifest = std::fs::read_to_string(&manifest_file)?;
        let custom_props = extract_custom_fields(&manifest)?;
        let upgrade_policy = if let Some(val) = custom_props.get(UPGRADE_POLICY_CUSTOM_FIELD) {
            str::parse::<UpgradePolicy>(val.as_ref())?
        } else {
            UpgradePolicy::compat()
        };
        let mut modules = vec![];
        for u in &self.package.root_compiled_units {
            let name = u.unit.name().to_string();
            let source = zip_metadata(std::fs::read_to_string(&u.source_path)?.as_bytes())?;
            let source_map = zip_metadata(&u.unit.serialize_source_map())?;
            modules.push(ModuleMetadata {
                name,
                source,
                source_map,
            })
        }
        let error_map = if let Some(map) = &self.error_map {
            bcs::to_bytes(map).expect("bcs for error map")
        } else {
            vec![]
        };
        let abis = if let Some(abis) = &self.package.compiled_abis {
            abis.iter().map(|(_, a)| ByteBuf::from(a.clone())).collect()
        } else {
            vec![]
        };

        Ok(PackageMetadata {
            name: self.name().to_string(),
            upgrade_policy,
            upgrade_number: 0,
            build_info,
            manifest,
            modules,
            error_map,
            abis,
        })
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
