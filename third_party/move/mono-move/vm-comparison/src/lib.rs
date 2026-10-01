// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! State overrides that make the execution results of a transaction captured
//! from a real network replay comparable on the Move VM V1 and on MonoMove.
//!
//! Known overrides:
//! - Gas is different between the VMs. The gas schedule is changed to not
//!   charge anything, so write sets and events are byte-comparable.
//! - MonoMove does not support the `0x1::aggregator` natives used by the legacy
//!   V1 aggregator implementation. On mainnet, only a single instance of a V1
//!   aggregator exists, used for total supply, and it can be switched to a
//!   regular integer. Comparison testing does not care about read-write
//!   conflicts or gas, so switching to an integer avoids running
//!   `0x1::aggregator` natives.
//! - MonoMove supports fungible asset dispatch via reflection. Move VM V1 uses
//!   a hook in the VM which was enabled long before reflection code was added.
//!   To test both VMs, the modules needed to run via the newer reflection path
//!   override that state, and the appropriate features are enabled.

pub mod coin_supply;
pub mod features;
pub mod gas;
pub mod modules;

use anyhow::Result;
use aptos_transaction_simulation::InMemoryStateStore;

/// Applies every state-level override to a captured dump.
pub fn prepare_state(state: &InMemoryStateStore) -> Result<()> {
    modules::override_framework(state)?;
    // After the modules are replaced, because the framework deployed on chain
    // does not declare the flags MonoMove needs.
    features::enable_features(state)?;
    coin_supply::sequentialize_coin_supply(state)?;
    gas::make_gas_free(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_transaction_simulation::SimulationStateStore;
    use aptos_types::on_chain_config::{FeatureFlag, Features};
    use move_binary_format::access::ModuleAccess;

    #[test]
    fn test_overridden_modules_are_deserializable() {
        let state = InMemoryStateStore::from_head_genesis();
        // A captured dump predates HEAD's bytecode version.
        state
            .modify_on_chain_config::<Features, _>(|features| {
                features.disable(FeatureFlag::VM_BINARY_FORMAT_V10);
                Ok(())
            })
            .unwrap();

        prepare_state(&state).unwrap();

        let version = aptos_cached_packages::head_release_bundle()
            .compiled_modules()
            .iter()
            .map(ModuleAccess::version)
            .max()
            .unwrap();
        let features = state.get_on_chain_config::<Features>().unwrap();
        assert!(features.get_max_binary_format_version() >= version);
    }
}
