// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod aggregator;
pub mod aggregator_v2;
pub mod aptos_governance;
pub mod harness;
pub mod stake;
pub mod transaction_fee;

use anyhow::bail;
use aptos_framework::{BuildOptions, BuiltPackage, UPGRADE_POLICY_CUSTOM_FIELD};
pub use harness::*;
use move_command_line_common::{env::read_bool_env_var, testing::MOVE_COMPILER_V2};
use move_package::{
    package_hooks::PackageHooks, source_package::parsed_manifest::CustomDepInfo, CompilerVersion,
};
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
    if read_bool_env_var(MOVE_COMPILER_V2) {
        options.compiler_version = Some(CompilerVersion::V2);
    }
    BuiltPackage::build(package_path.to_owned(), options)
}
