// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::change_set::WriteOpInfo;
use velor_gas_algebra::{Fee, NumSlots};
use velor_gas_schedule::TransactionGasParameters;
use velor_types::{
    account_config::AccountResource, contract_event::ContractEvent, on_chain_config::Features,
    state_store::state_key::StateKey, write_set::WriteOpSize,
};
use move_core_types::gas_algebra::NumBytes;
use std::fmt::Debug;

pub struct ChargeAndRefund {
    pub charge: Fee,
    pub refund: Fee,
}

impl ChargeAndRefund {
    pub fn zero() -> Self {
        Self {
            charge: 0.into(),
            refund: 0.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum DiskSpacePricing {
    /// With per state slot free write quota
    V1,
    /// With per txn ephemeral storage fee discount
    V2,
}

impl DiskSpacePricing {
    pub fn new(gas_feature_version: u64, features: &Features) -> Self {
        if gas_feature_version >= 13 && features.is_refundable_bytes_enabled() {
            Self::V2
        } else {
            Self::V1
        }
    }

    /// Calculates the storage fee for a state slot allocation.
    pub fn charge_refund_write_op(
        &self,
        params: &TransactionGasParameters,
        write_op_info: WriteOpInfo,
    ) -> ChargeAndRefund {
        match self {
            Self::V1 => Self::charge_refund_write_op_v1(params, write_op_info),
            Self::V2 => Self::charge_refund_write_op_v2(params, write_op_info),
        }
    }

    /// Calculates the storage fee for an event.
    pub fn legacy_storage_fee_per_event(
        &self,
        params: &TransactionGasParameters,
        event: &ContractEvent,
    ) -> Fee {
        match self {
            Self::V1 => {
                NumBytes::new(event.size() as u64) * params.legacy_storage_fee_per_event_byte
            },
            Self::V2 => 0.into(),
        }
    }

    /// Calculates the discount applied to the event storage fees, based on a free quota.
    ///
    /// This is specific to DiskSpacePricingV1, and applicable to only event bytes.
    pub fn legacy_storage_discount_for_events(
        &self,
        params: &TransactionGasParameters,
        total_cost: Fee,
    ) -> Fee {
        match self {
            Self::V1 => std::cmp::min(
                total_cost,
                params.legacy_free_event_bytes_quota * params.legacy_storage_fee_per_event_byte,
            ),
            Self::V2 => 0.into(),
        }
    }

    /// Calculates the storage fee for the transaction.
    pub fn legacy_storage_fee_for_transaction_storage(
        &self,
        params: &TransactionGasParameters,
        txn_size: NumBytes,
    ) -> Fee {
        match self {
            Self::V1 => {
                txn_size
                    .checked_sub(params.large_transaction_cutoff)
                    .unwrap_or(NumBytes::zero())
                    * params.legacy_storage_fee_per_transaction_byte
            },
            Self::V2 => 0.into(),
        }
    }

    // ----- private methods -----

    fn discounted_write_op_size_for_v1(
        params: &TransactionGasParameters,
        key: &StateKey,
        value_size: u64,
    ) -> NumBytes {
        let size = NumBytes::new(key.size() as u64) + NumBytes::new(value_size);
        size.checked_sub(params.legacy_free_write_bytes_quota)
            .unwrap_or(NumBytes::zero())
    }

    fn charge_refund_write_op_v1(
        params: &TransactionGasParameters,
        op: WriteOpInfo,
    ) -> ChargeAndRefund {
        use WriteOpSize::*;

        match op.op_size {
            Creation { write_len } => {
                let slot_fee = params.legacy_storage_fee_per_state_slot_create * NumSlots::new(1);
                let bytes_fee = Self::discounted_write_op_size_for_v1(params, op.key, write_len)
                    * params.legacy_storage_fee_per_excess_state_byte;

                if !op.metadata_mut.is_none() {
                    op.metadata_mut.set_slot_deposit(slot_fee.into())
                }

                ChargeAndRefund {
                    charge: slot_fee + bytes_fee,
                    refund: 0.into(),
                }
            },
            Modification { write_len } => {
                let bytes_fee = Self::discounted_write_op_size_for_v1(params, op.key, write_len)
                    * params.legacy_storage_fee_per_excess_state_byte;

                ChargeAndRefund {
                    charge: bytes_fee,
                    refund: 0.into(),
                }
            },
            Deletion => ChargeAndRefund {
                charge: 0.into(),
                refund: op.metadata_mut.total_deposit().into(),
            },
        }
    }

    /// n.b. logcic for bytes fee:
    /// * When slot increase in size on modification, charge additionally into the deposit.
    ///     * legacy slots that didn't pay bytes deposits won't get charged for the bytes allocated for free.
    ///     * Considering pricing change, charge only to the point where the total deposit for bytes don't go
    ///       beyond `current_price_per_byte * num_current_bytes`
    /// * When slot decrease in size, don't refund, to simplify implementation.
    /// * If slot doesn't change in size on modification, no charging even if pricing changes.
    /// * Refund only on deletion.
    /// * There's no longer non-refundable penalty when a slot larger than 1KB gets touched.
    fn charge_refund_write_op_v2(
        params: &TransactionGasParameters,
        op: WriteOpInfo,
    ) -> ChargeAndRefund {
        use WriteOpSize::*;

        let key_size = op.key.size() as u64;
        let num_bytes = key_size + op.op_size.write_len().unwrap_or(0);
        let target_bytes_deposit: u64 = num_bytes * u64::from(params.storage_fee_per_state_byte);

        match op.op_size {
            Creation { .. } => {
                // permanent storage fee
                let slot_deposit = u64::from(params.storage_fee_per_state_slot);

                op.metadata_mut.maybe_upgrade();
                op.metadata_mut.set_slot_deposit(slot_deposit);
                op.metadata_mut.set_bytes_deposit(target_bytes_deposit);

                ChargeAndRefund {
                    charge: (slot_deposit + target_bytes_deposit).into(),
                    refund: 0.into(),
                }
            },
            Modification { write_len } => {
                // Change of slot size or per byte price can result in a charge or refund of the bytes fee.
                let old_bytes_deposit = op.metadata_mut.bytes_deposit();
                let state_bytes_charge =
                    if write_len > op.prev_size && target_bytes_deposit > old_bytes_deposit {
                        let charge_by_increase: u64 = (write_len - op.prev_size)
                            * u64::from(params.storage_fee_per_state_byte);
                        let gap_from_target = target_bytes_deposit - old_bytes_deposit;
                        std::cmp::min(charge_by_increase, gap_from_target)
                    } else {
                        0
                    };
                op.metadata_mut.maybe_upgrade();
                op.metadata_mut
                    .set_bytes_deposit(old_bytes_deposit + state_bytes_charge);

                ChargeAndRefund {
                    charge: state_bytes_charge.into(),
                    refund: 0.into(),
                }
            },
            Deletion => ChargeAndRefund {
                charge: 0.into(),
                refund: op.metadata_mut.total_deposit().into(),
            },
        }
    }

    pub fn hack_estimated_fee_for_account_creation(
        &self,
        params: &TransactionGasParameters,
    ) -> Fee {
        match self {
            Self::V1 => params.legacy_storage_fee_per_state_slot_create * NumSlots::new(1),
            Self::V2 => {
                params.storage_fee_per_state_slot * NumSlots::new(1)
                    + NumBytes::new(ACCOUNT_RESOURCE_BYTES_OVER_ESTIMATE)
                        * params.storage_fee_per_state_byte
            },
        }
    }

    pub fn hack_account_creation_fee_lower_bound(&self, params: &TransactionGasParameters) -> Fee {
        match self {
            Self::V1 => params.legacy_storage_fee_per_state_slot_create * NumSlots::new(1),
            Self::V2 => {
                // This is an underestimation of the fee for account creation, because AccountResource has a
                // vector and two optional addresses in it which will expand to more bytes on-chain
                params.storage_fee_per_state_slot * NumSlots::new(1)
                    + params.storage_fee_per_state_byte
                        * NumBytes::new(std::mem::size_of::<AccountResource>() as u64)
            },
        }
    }
}

const ACCOUNT_RESOURCE_BYTES_OVER_ESTIMATE: u64 = 300;

#[cfg(test)]
mod tests {
    use super::*;
    use velor_types::{
        on_chain_config::CurrentTimeMicroseconds, state_store::state_value::StateValueMetadata,
    };

    /// to make sure hack_estimated_fee_for_account_creation() is safe
    #[test]
    fn account_resource_bytes_over_estimate() {
        let lower_bound = std::mem::size_of::<AccountResource>() as u64;
        println!("lower_bound: {}", lower_bound);
        assert!(ACCOUNT_RESOURCE_BYTES_OVER_ESTIMATE > lower_bound);
    }

    #[test]
    fn test_bytes_deposit() {
        let pricing = DiskSpacePricing::V2;
        let mut params = TransactionGasParameters::random();
        params.storage_fee_per_state_byte = 5.into();
        params.storage_fee_per_state_slot = 1000.into();
        let key = StateKey::raw(&[1, 2, 3]);
        assert_eq!(key.size(), 3); // to make sure our assumptions on the numbers in the assertions below are correct
        let ts = CurrentTimeMicroseconds { microseconds: 0 };
        let mut meta = StateValueMetadata::new(0, 0, &ts);

        // create new
        let ChargeAndRefund { charge: _, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Creation { write_len: 2 },
                prev_size: 0,
                metadata_mut: &mut meta,
            });
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 25);
        assert_eq!(meta.slot_deposit(), 1000);

        // legacy slots without bytes deposit recorded doesn't get charged if size doesn't increase
        meta.set_bytes_deposit(0); // marks it paid 0 bytes deposit
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Modification { write_len: 2 },
                prev_size: 2,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 0.into());
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 0);

        // but if it does increase in size, new bytes gets charged, at the latest rate
        params.storage_fee_per_state_byte = 20.into();
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Modification { write_len: 4 },
                prev_size: 2,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 40.into());
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 40);

        // price lowered, adding a new byte, the target deposit is (3 + 5) * 10 = 80
        params.storage_fee_per_state_byte = 10.into();
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Modification { write_len: 5 },
                prev_size: 4,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 10.into());
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 50);

        // price lowered, adding a new byte, the target deposit is (3 + 6) * 6 = 54
        // the charge is lower than one byte according to the current pricing so the
        // deposit won't go beyond the target deposit
        params.storage_fee_per_state_byte = 6.into();
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Modification { write_len: 6 },
                prev_size: 5,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 4.into());
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 54);

        // price lowered, adding a new byte, the target deposit is (3 + 7) * 5 = 50
        // no new charge is incurred
        params.storage_fee_per_state_byte = 5.into();
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Modification { write_len: 7 },
                prev_size: 6,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 0.into());
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 54);

        // no refund for reducing size
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Modification { write_len: 2 },
                prev_size: 7,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 0.into());
        assert_eq!(refund, 0.into());
        assert_eq!(meta.bytes_deposit(), 54);

        // refund all on deletion
        let ChargeAndRefund { charge, refund } =
            pricing.charge_refund_write_op(&params, WriteOpInfo {
                key: &key,
                op_size: WriteOpSize::Deletion,
                prev_size: 2,
                metadata_mut: &mut meta,
            });
        assert_eq!(charge, 0.into());
        assert_eq!(refund, 1054.into());
        // no need to clear up the metadata for deletions
        assert_eq!(meta.slot_deposit(), 1000);
        assert_eq!(meta.bytes_deposit(), 54);
    }
}
