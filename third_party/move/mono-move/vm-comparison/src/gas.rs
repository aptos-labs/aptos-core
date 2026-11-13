// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Helpers to provide gas-free replay support.

use anyhow::Result;
use aptos_transaction_simulation::{InMemoryStateStore, SimulationStateStore};
use aptos_types::{on_chain_config::GasScheduleV2, state_store::state_value::StateValue};

/// The cost entries of the `txn.` namespace. Everything else under `txn.` is
/// structure (bounds, limits, quotas, unit scaling) and should NOT be zeroed.
const TXN_COST_ENTRIES: &[&str] = &[
    "txn.min_transaction_gas_units",
    "txn.intrinsic_gas_per_byte",
    "txn.load_data.base",
    "txn.load_data.failure",
    "txn.load_data.per_byte",
    "txn.write_data.new_item",
    "txn.write_data.per_op",
    "txn.write_data.per_byte_in_key",
    "txn.write_data.per_byte_in_val",
    "txn.legacy_storage_fee_per_event_byte",
    "txn.legacy_storage_fee_per_excess_state_byte",
    "txn.legacy_storage_fee_per_state_slot_create",
    "txn.legacy_storage_fee_per_transaction_byte",
    "txn.storage_fee_per_event_byte",
    "txn.storage_fee_per_excess_state_byte",
    "txn.storage_fee_per_state_byte",
    "txn.storage_fee_per_state_slot",
    "txn.storage_fee_per_state_slot_create",
    "txn.storage_fee_per_transaction_byte",
    "txn.storage_io_per_event_byte_write",
    "txn.storage_io_per_state_byte_read",
    "txn.storage_io_per_state_byte_write",
    "txn.storage_io_per_state_slot_read",
    "txn.storage_io_per_state_slot_write",
    "txn.storage_io_per_transaction_byte_write",
    "txn.dependency_per_byte",
    "txn.dependency_per_module",
    "txn.keyless.base",
    "txn.slh_dsa_sha2_128s.base",
    "txn.encrypted_txn_decryption.base",
];

/// Scales internal gas down to external gas, replacing the production value of
/// 1,000,000. A safety net for any cost this module failed to zero: external gas
/// is rounded up, so a missed charge is reported as a single unit instead of a
/// number large enough to change balances or to run the transaction out of gas.
const GAS_UNIT_SCALING_FACTOR: u64 = 1_000_000_000;

/// Whether a gas schedule entry is a cost (which should be zeroed).
fn is_cost(name: &str) -> bool {
    // Abstract value sizes are used by cost formulas but are not costs; zeroing
    // them would hide the charges the formulas are supposed to produce.
    if name.starts_with("misc.abs_val.") {
        return false;
    }
    if name.starts_with("txn.") {
        return TXN_COST_ENTRIES.contains(&name);
    }
    true
}

/// Rewrites the gas schedule so all costs are set to zero.
/// State-value metadata is also stripped so slot deletions cannot result in refunds.
///
/// Both are required for a byte-for-byte comparison between V1 and V2.
pub fn make_gas_free(state: &InMemoryStateStore) -> Result<()> {
    for (key, value) in state.to_btree_map() {
        state.set_state_value(key, StateValue::new_legacy(value.bytes().clone()))?;
    }

    state.modify_on_chain_config::<GasScheduleV2, _>(|schedule| {
        for (name, value) in &mut schedule.entries {
            if is_cost(name) {
                *value = 0;
            } else if name == "txn.gas_unit_scaling_factor" {
                *value = GAS_UNIT_SCALING_FACTOR;
            }
        }
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abstract_value_sizes_are_not_costs() {
        assert!(!is_cost("misc.abs_val.u8"));
        assert!(!is_cost("misc.abs_val.per_address_packed"));
    }

    #[test]
    fn test_value_traversal_is_a_cost() {
        assert!(is_cost("misc.value_traversal.base"));
        assert!(is_cost("misc.value_traversal.per_abs_val_unit"));
    }

    #[test]
    fn test_txn_structure_is_not_a_cost() {
        assert!(!is_cost("txn.gas_unit_scaling_factor"));
        assert!(!is_cost("txn.maximum_number_of_gas_units"));
        assert!(is_cost("txn.intrinsic_gas_per_byte"));
    }
}
