// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod aggregator;
pub mod aggregator_v2;
pub mod aptos_governance;
pub mod harness;
pub mod resource_groups;
pub mod stake;

use anyhow::bail;
use aptos_framework::{BuildOptions, BuiltPackage, UPGRADE_POLICY_CUSTOM_FIELD};
pub use harness::*;
use move_command_line_common::env::get_move_compiler_v2_from_env;
use move_model::metadata::CompilerVersion;
use move_package::{package_hooks::PackageHooks, source_package::parsed_manifest::CustomDepInfo};
use move_symbol_pool::Symbol;
pub use stake::*;
use std::path::PathBuf;

#[cfg(test)]
mod tests;

pub(crate) struct AptosPackageHooks {}

impl PackageHooks for AptosPackageHooks {
    fn custom_package_info_fields(&self) -> Vec<String> {
        vec![UPGRADE_POLICY_CUSTOM_FIELD.to_string()]
    }

    fn custom_dependency_key(&self) -> Option<String> {
        Some("aptos".to_string())
    }

    fn resolve_custom_dependency(
        &self,
        _dep_name: Symbol,
        _info: &CustomDepInfo,
    ) -> anyhow::Result<()> {
        bail!("not used")
    }
}

pub(crate) fn build_package(
    package_path: PathBuf,
    options: BuildOptions,
) -> anyhow::Result<BuiltPackage> {
    let mut options = options;
    if get_move_compiler_v2_from_env() {
        options.compiler_version = Some(CompilerVersion::latest_stable());
        options.language_version =
            Some(CompilerVersion::latest_stable().infer_stable_language_version());
    }
    BuiltPackage::build(package_path.to_owned(), options)
}

#[cfg(test)]
pub(crate) fn build_package_with_compiler_version(
    package_path: PathBuf,
    options: BuildOptions,
    compiler_version: CompilerVersion,
) -> anyhow::Result<BuiltPackage> {
    let mut options = options;
    options.language_version = Some(compiler_version.infer_stable_language_version());
    options.compiler_version = Some(compiler_version);
    BuiltPackage::build(package_path.to_owned(), options)
}
