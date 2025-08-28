// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod aggregator;
pub mod aggregator_v2;
pub mod aptos_governance;
pub mod harness;
pub mod resource_groups;
pub mod stake;

use anyhow::bail;
use aptos_framework::UPGRADE_POLICY_CUSTOM_FIELD;
pub use harness::*;
use move_package::{package_hooks::PackageHooks, source_package::parsed_manifest::CustomDepInfo};
use move_symbol_pool::Symbol;
pub use stake::*;

#[cfg(test)]
pub mod tests;

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
