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
use aptos_types::on_chain_config::FeatureFlag;
pub use harness::*;
use move_package::{package_hooks::PackageHooks, source_package::parsed_manifest::CustomDepInfo};
use move_symbol_pool::Symbol;
pub use stake::*;

pub fn feature_flags_for_orderless(
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> Vec<FeatureFlag> {
    let mut flags = vec![];
    if use_txn_payload_v2_format {
        flags.push(FeatureFlag::TRANSACTION_PAYLOAD_V2);
    }
    if use_orderless_transactions {
        flags.push(FeatureFlag::ORDERLESS_TRANSACTIONS);
    }
    flags
}

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
