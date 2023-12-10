// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::{Fee, NumSlots};
use aptos_gas_schedule::TransactionGasParameters;
use aptos_types::{
    contract_event::ContractEvent,
    on_chain_config::Features,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::WriteOpSize,
};
use move_core_types::gas_algebra::NumBytes;
use std::fmt::Debug;

pub struct ChargeAndRefund {
    // The amount not subject to the per txn discount, including all DiskSpacePricingV1 charges
    // and the refundable portion of DiskSpacePricingV2 charges (state slot and state bytes charges).
    pub non_discountable: Fee,
    // The amount subject to the per txn discounts, i.e. the "ephemeral bytes" charges by
    // DiskSpacePricingV2.
    pub discountable: Fee,
    pub refund: Fee,
}

impl ChargeAndRefund {
    pub fn zero() -> Self {
        Self {
            non_discountable: 0.into(),
            discountable: 0.into(),
            refund: 0.into(),
        }
    }

    pub fn combine(&mut self, other: Self) {
        let Self {
            non_discountable,
            discountable,
            refund,
        } = other;

        self.non_discountable += non_discountable;
        self.discountable += discountable;
        self.refund += refund;
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
        if gas_feature_version >= 12 && features.is_ephemeral_storage_fee_enabled() {
            Self::V2
        } else {
            Self::V1
        }
    }

    pub fn latest() -> Self {
        Self::V2
    }

    /// Calculates the storage fee for a state slot allocation.
    pub fn charge_refund_write_op(
        &self,
        params: &TransactionGasParameters,
        key: &StateKey,
        op_size: &WriteOpSize,
        metadata: &mut StateValueMetadata,
    ) -> ChargeAndRefund {
        match self {
            Self::V1 => Self::charge_refund_write_op_v1(params, key, op_size, metadata),
            Self::V2 => Self::charge_refund_write_op_v2(params, key, op_size, metadata),
        }
    }

    /// Calculates the storage fee for an event.
    pub fn storage_fee_per_event(
        &self,
        params: &TransactionGasParameters,
        event: &ContractEvent,
    ) -> Fee {
        match self {
            Self::V1 => {
                NumBytes::new(event.size() as u64) * params.legacy_storage_fee_per_event_byte
            },
            Self::V2 => NumBytes::new(event.size() as u64) * params.storage_fee_per_event_byte,
        }
    }

    /// Calculates the discount applied to the event storage fees, based on a free quota.
    ///
    /// This is specific to DiskSpacePricingV1, and applicable to only event bytes.
    pub fn storage_discount_for_events(
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
    pub fn storage_fee_for_transaction_storage(
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
            Self::V2 => txn_size * params.storage_fee_per_transaction_byte,
        }
    }

    /// Calculates the discount applied to the total of ephemeral storage fees, based on a free quota.
    ///
    /// This is specific to DiskSpacePricingV2, where the per state slot free write quota is removed.
    pub fn ephemeral_storage_fee_discount(
        &self,
        params: &TransactionGasParameters,
        total_ephemeral_fee: Fee,
    ) -> Fee {
        match self {
            DiskSpacePricing::V1 => 0.into(),
            DiskSpacePricing::V2 => std::cmp::min(
                total_ephemeral_fee,
                params.ephemeral_storage_fee_discount_per_transaction,
            ),
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
        key: &StateKey,
        op_size: &WriteOpSize,
        metadata: &mut StateValueMetadata,
    ) -> ChargeAndRefund {
        use WriteOpSize::*;

        match op_size {
            Creation { write_len } => {
                let slot_fee = params.legacy_storage_fee_per_state_slot_create * NumSlots::new(1);
                let bytes_fee = Self::discounted_write_op_size_for_v1(params, key, *write_len)
                    * params.legacy_storage_fee_per_excess_state_byte;

                if !metadata.is_none() {
                    metadata.set_slot_deposit(slot_fee.into())
                }

                ChargeAndRefund {
                    non_discountable: slot_fee + bytes_fee,
                    discountable: 0.into(),
                    refund: 0.into(),
                }
            },
            Modification { write_len } => {
                let bytes_fee = Self::discounted_write_op_size_for_v1(params, key, *write_len)
                    * params.legacy_storage_fee_per_excess_state_byte;

                ChargeAndRefund {
                    non_discountable: bytes_fee,
                    discountable: 0.into(),
                    refund: 0.into(),
                }
            },
            Deletion => ChargeAndRefund {
                non_discountable: 0.into(),
                discountable: 0.into(),
                refund: metadata.total_deposit().into(),
            },
        }
    }

    fn charge_refund_write_op_v2(
        params: &TransactionGasParameters,
        key: &StateKey,
        op_size: &WriteOpSize,
        metadata: &mut StateValueMetadata,
    ) -> ChargeAndRefund {
        use WriteOpSize::*;

        // ephemeral storage fee
        let write_op_fee = params.storage_fee_per_write_op * NumSlots::new(1);
        let num_bytes =
            NumBytes::new(key.size() as u64) + NumBytes::new(op_size.write_len().unwrap_or(0));
        let write_op_bytes_fee = params.storage_fee_per_write_set_byte * num_bytes;
        let discountable = write_op_fee + write_op_bytes_fee;

        match op_size {
            Creation { .. } => {
                // permanent storage fee
                let slot_deposit = params.storage_fee_per_state_slot_refundable * NumSlots::new(1);
                let bytes_deposit = num_bytes * params.storage_fee_per_state_byte_refundable;

                metadata.set_slot_deposit(slot_deposit.into());
                metadata.set_bytes_deposit(bytes_deposit.into());

                ChargeAndRefund {
                    non_discountable: slot_deposit + bytes_deposit,
                    discountable,
                    refund: 0.into(),
                }
            },
            Modification { write_len } => {
                // change of slot size or per byte price can result in a charge or refund of permanent bytes fee
                let num_bytes = NumBytes::new(key.size() as u64) + NumBytes::new(*write_len);
                let target_bytes_deposit = num_bytes * params.storage_fee_per_state_byte_refundable;
                let old_bytes_deposit = metadata.bytes_deposit().into();
                let (state_bytes_charge, state_bytes_refund) = if target_bytes_deposit
                    > old_bytes_deposit
                {
                    let bytes_deposit =
                        target_bytes_deposit.checked_sub(old_bytes_deposit).unwrap();
                    (bytes_deposit, 0.into())
                } else {
                    let bytes_refund = old_bytes_deposit.checked_sub(target_bytes_deposit).unwrap();
                    (0.into(), bytes_refund)
                };
                metadata.set_bytes_deposit(target_bytes_deposit.into());

                ChargeAndRefund {
                    non_discountable: state_bytes_charge,
                    discountable,
                    refund: state_bytes_refund,
                }
            },
            Deletion => ChargeAndRefund {
                non_discountable: 0.into(),
                discountable,
                refund: metadata.total_deposit().into(),
            },
        }
    }
}
