// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Feature overrides: the on-chain `Features` config stored in the dump, and the
//! process-global timed-feature profile the VM environment reads.

use anyhow::{bail, Result};
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatureOverride};

/// Flags that must be on. The binary format flag caps the deserializer, and the
/// overridden modules are compiled at HEAD's bytecode version.
const FORCED_ON: &[FeatureFlag] = &[
    FeatureFlag::ENABLE_FUNCTION_REFLECTION,
    FeatureFlag::FUNCTION_VALUE_DISPATCH,
    FeatureFlag::VM_BINARY_FORMAT_V10,
];

/// Enables [`FORCED_ON`] in the dump's on-chain config.
///
/// Must run after the framework modules are replaced, because the `features`
/// module deployed on chain does not declare these flags.
pub fn enable_features(state: &InMemoryStateStore) -> Result<()> {
    state.modify_on_chain_config::<Features, _>(|features| {
        for flag in FORCED_ON {
            features.enable(*flag);
        }
        Ok(())
    })
}

/// Sets timed features to comparison mode, so that they can be enabled or
/// disabled as needed.
pub fn install_timed_feature_override() -> Result<()> {
    aptos_vm_environment::prod_configs::set_timed_feature_override(
        TimedFeatureOverride::VmComparison,
    );
    match aptos_vm_environment::prod_configs::get_timed_feature_override() {
        Some(TimedFeatureOverride::VmComparison) => Ok(()),
        other => bail!("A different timed feature override is already installed: {other:?}"),
    }
}
