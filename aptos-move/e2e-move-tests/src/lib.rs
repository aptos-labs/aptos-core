// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod aggregator;
pub mod harness;
pub mod stake;

use anyhow::bail;
use framework::UPGRADE_POLICY_CUSTOM_FIELD;
pub use harness::*;
use move_deps::move_package::package_hooks::PackageHooks;
use move_deps::move_package::source_package::parsed_manifest::CustomDepInfo;
use move_deps::move_symbol_pool::Symbol;
pub use stake::*;

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
