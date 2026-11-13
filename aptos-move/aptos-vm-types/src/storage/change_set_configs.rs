// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::change_set::ChangeSetInterface;
use aptos_gas_schedule::AptosGasParameters;
use move_binary_format::errors::{Location, PartialVMError};
use move_core_types::vm_status::{StatusCode, VMStatus};

/// Cap on distinct positions a single transaction may write. Position writes
/// are not part of the gas-metered write set, so this is a hard bound rather
/// than a gas-derived one, and applies even to otherwise unlimited configs.
/// A `NativePosition` is fixed-size, so bounding the count bounds the bytes;
/// a separate byte limit would be redundant.
///
/// TODO[native_position](metering): derive this from gas parameters like the
/// other limits once position writes are gas-metered, instead of a const.
pub const MAX_POSITION_WRITE_OPS_PER_TRANSACTION: usize = 1000;

/// Cap on distinct orders a single transaction may write, a hard bound like
/// [`MAX_POSITION_WRITE_OPS_PER_TRANSACTION`].
///
/// TODO[native_order](metering): derive from gas parameters once order writes
/// are gas-metered.
pub const MAX_ORDER_WRITE_OPS_PER_TRANSACTION: usize = 1000;

/// Cap on the total bytes (key + value) of a transaction's order writes.
/// Order values vary in size, so the count cap alone does not bound them.
/// Separate from the regular write set's byte limits, which order writes do
/// not count against.
///
/// TODO[native_order](metering): derive from gas parameters once order writes
/// are gas-metered.
pub const MAX_ORDER_WRITE_BYTES_PER_TRANSACTION: u64 = 1 << 20;

/// Cap on distinct collateral entries a single transaction may write, a hard
/// bound like [`MAX_ORDER_WRITE_OPS_PER_TRANSACTION`].
///
/// TODO[native_collateral](metering): derive from gas parameters once
/// collateral writes are gas-metered.
pub const MAX_COLLATERAL_WRITE_OPS_PER_TRANSACTION: usize = 1000;

/// Cap on the total bytes (key + value) of a transaction's collateral writes,
/// separate from the regular write set's byte limits, as for orders.
///
/// TODO[native_collateral](metering): derive from gas parameters once
/// collateral writes are gas-metered.
pub const MAX_COLLATERAL_WRITE_BYTES_PER_TRANSACTION: u64 = 1 << 20;

#[derive(Clone, Debug)]
pub struct ChangeSetConfigs {
    gas_feature_version: u64,
    max_bytes_per_write_op: u64,
    max_bytes_all_write_ops_per_transaction: u64,
    max_bytes_per_event: u64,
    max_bytes_all_events_per_transaction: u64,
    max_write_ops_per_transaction: u64,
}

impl ChangeSetConfigs {
    pub fn unlimited_at_gas_feature_version(gas_feature_version: u64) -> Self {
        Self::new_impl(
            gas_feature_version,
            u64::MAX,
            u64::MAX,
            u64::MAX,
            u64::MAX,
            u64::MAX,
        )
    }

    pub fn new(feature_version: u64, gas_params: &AptosGasParameters) -> Self {
        if feature_version >= 5 {
            Self::from_gas_params(feature_version, gas_params)
        } else if feature_version >= 3 {
            Self::for_feature_version_3()
        } else {
            Self::unlimited_at_gas_feature_version(feature_version)
        }
    }

    fn new_impl(
        gas_feature_version: u64,
        max_bytes_per_write_op: u64,
        max_bytes_all_write_ops_per_transaction: u64,
        max_bytes_per_event: u64,
        max_bytes_all_events_per_transaction: u64,
        max_write_ops_per_transaction: u64,
    ) -> Self {
        Self {
            gas_feature_version,
            max_bytes_per_write_op,
            max_bytes_all_write_ops_per_transaction,
            max_bytes_per_event,
            max_bytes_all_events_per_transaction,
            max_write_ops_per_transaction,
        }
    }

    pub fn legacy_resource_creation_as_modification(&self) -> bool {
        // Bug fixed at gas_feature_version 3 where (non-group) resource creation was converted to
        // modification.
        // Modules and table items were not affected (https://github.com/aptos-labs/aptos-core/pull/4722/commits/7c5e52297e8d1a6eac67a68a804ab1ca2a0b0f37).
        // Resource groups and state values with metadata were not affected because they were
        // introduced later than feature_version 3 on all networks.
        self.gas_feature_version < 3
    }

    /// Fail-closed window `[RELEASE_V1_46, RELEASE_V1_48)`. Within it, squashing a full write (an
    /// earlier session that wrote a value or a resource group) with a later *in-place delayed-field
    /// change* on the *same* key (e.g. the gas epilogue exchanging an aggregator in that
    /// value/group) is rejected outright. This covers both the resource-group arm
    /// (`WriteResourceGroup ⊕ ResourceGroupInPlaceDelayedFieldChange`) and the standalone-resource
    /// arm (`WriteWithDelayedFields ⊕ InPlaceDelayedFieldChange`).
    ///
    /// Before `RELEASE_V1_46`, the merge was permitted whenever the materialized sizes happened to
    /// match -- a matching size does not by itself prove the later session's delayed-field exchange
    /// reconciles with what the earlier session wrote, hence the defensive fail-closed interim.
    ///
    /// From `RELEASE_V1_48`, the view-layer reconciliation fix (which layers the earlier change set
    /// so the later session's in-place exchange reflects the earlier write) is active, so the
    /// materialized-size match is once again a sound proof and we revert to the legacy squash. See
    /// `VMChangeSet::squash_additional_resource_writes`.
    pub fn strict_delayed_field_squash(&self) -> bool {
        use aptos_gas_schedule::gas_feature_versions::{RELEASE_V1_46, RELEASE_V1_48};
        (RELEASE_V1_46..RELEASE_V1_48).contains(&self.gas_feature_version)
    }

    fn for_feature_version_3() -> Self {
        const MB: u64 = 1 << 20;

        Self::new_impl(3, MB, u64::MAX, MB, 10 * MB, u64::MAX)
    }

    fn from_gas_params(gas_feature_version: u64, gas_params: &AptosGasParameters) -> Self {
        let params = &gas_params.vm.txn;
        Self::new_impl(
            gas_feature_version,
            params.max_bytes_per_write_op.into(),
            params.max_bytes_all_write_ops_per_transaction.into(),
            params.max_bytes_per_event.into(),
            params.max_bytes_all_events_per_transaction.into(),
            params.max_write_ops_per_transaction.into(),
        )
    }

    pub fn check_change_set(&self, change_set: &impl ChangeSetInterface) -> Result<(), VMStatus> {
        let storage_write_limit_reached = |maybe_message: Option<&str>| {
            let mut err = PartialVMError::new(StatusCode::STORAGE_WRITE_LIMIT_REACHED);
            if let Some(message) = maybe_message {
                err = err.with_message(message.to_string())
            }
            Err(err.finish(Location::Undefined).into_vm_status())
        };

        if self.max_write_ops_per_transaction != 0
            && change_set.num_write_ops() as u64 > self.max_write_ops_per_transaction
        {
            return storage_write_limit_reached(Some("Too many write ops."));
        }

        if change_set.num_position_write_ops() > MAX_POSITION_WRITE_OPS_PER_TRANSACTION {
            return storage_write_limit_reached(Some("Too many position write ops."));
        }

        if change_set.num_order_write_ops() > MAX_ORDER_WRITE_OPS_PER_TRANSACTION {
            return storage_write_limit_reached(Some("Too many order write ops."));
        }

        let order_write_bytes: u64 = change_set
            .order_write_set_size_iter()
            .filter_map(|(key, op_size)| Some(op_size.write_len()? + key.size() as u64))
            .sum();
        if order_write_bytes > MAX_ORDER_WRITE_BYTES_PER_TRANSACTION {
            return storage_write_limit_reached(Some("Too many order write bytes."));
        }

        if change_set.num_collateral_write_ops() > MAX_COLLATERAL_WRITE_OPS_PER_TRANSACTION {
            return storage_write_limit_reached(Some("Too many collateral write ops."));
        }

        let collateral_write_bytes: u64 = change_set
            .collateral_write_set_size_iter()
            .filter_map(|(key, op_size)| Some(op_size.write_len()? + key.size() as u64))
            .sum();
        if collateral_write_bytes > MAX_COLLATERAL_WRITE_BYTES_PER_TRANSACTION {
            return storage_write_limit_reached(Some("Too many collateral write bytes."));
        }

        let mut write_set_size = 0;
        for (key, op_size) in change_set.write_set_size_iter() {
            if let Some(len) = op_size.write_len() {
                let write_op_size = len + (key.size() as u64);
                if write_op_size > self.max_bytes_per_write_op {
                    return storage_write_limit_reached(None);
                }
                write_set_size += write_op_size;
            }
            if write_set_size > self.max_bytes_all_write_ops_per_transaction {
                return storage_write_limit_reached(None);
            }
        }

        let mut total_event_size = 0;
        for event in change_set.events_iter() {
            let size = event.event_data().len() as u64;
            if size > self.max_bytes_per_event {
                return storage_write_limit_reached(None);
            }
            total_event_size += size;
            if total_event_size > self.max_bytes_all_events_per_transaction {
                return storage_write_limit_reached(None);
            }
        }

        Ok(())
    }
}
