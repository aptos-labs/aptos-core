// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::{Fee, NumSlots};
use aptos_gas_schedule::TransactionGasParameters;
use aptos_types::{
    contract_event::ContractEvent,
    state_store::{state_key::StateKey, state_value::StateValueMetadata},
    write_set::WriteOpSize,
};
use move_core_types::gas_algebra::NumBytes;
use std::fmt::Debug;

pub struct ChargeAndRefund {
    pub charge: Fee,
    pub refund: Fee,
}

#[derive(Clone, Debug)]
pub enum DiskSpacePricing {
    /// With per state slot free write quota
    V1,
}

impl DiskSpacePricing {
    pub fn v1() -> Self {
        Self::V1
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
        }
    }

    /// Calculates the storage fee for an event.
    pub fn storage_fee_per_event(
        &self,
        params: &TransactionGasParameters,
        event: &ContractEvent,
    ) -> Fee {
        match self {
            Self::V1 => NumBytes::new(event.size() as u64) * params.storage_fee_per_event_byte,
        }
    }

    /// Calculates the discount applied to the event storage fees, based on a free quota.
    pub fn storage_discount_for_events(
        &self,
        params: &TransactionGasParameters,
        total_cost: Fee,
    ) -> Fee {
        match self {
            Self::V1 => std::cmp::min(
                total_cost,
                params.free_event_bytes_quota * params.storage_fee_per_event_byte,
            ),
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
                    * params.storage_fee_per_transaction_byte
            },
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
                let slot_fee = params.storage_fee_per_state_slot_create * NumSlots::new(1);
                let bytes_fee = Self::discounted_write_op_size_for_v1(params, key, *write_len)
                    * params.storage_fee_per_excess_state_byte;

                if !metadata.is_none() {
                    metadata.set_slot_deposit(slot_fee.into())
                }

                ChargeAndRefund {
                    charge: slot_fee + bytes_fee,
                    refund: 0.into(),
                }
            },
            Modification { write_len } => {
                let bytes_fee = Self::discounted_write_op_size_for_v1(params, key, *write_len)
                    * params.storage_fee_per_excess_state_byte;

                ChargeAndRefund {
                    charge: bytes_fee,
                    refund: 0.into(),
                }
            },
            Deletion => ChargeAndRefund {
                charge: 0.into(),
                refund: metadata.total_deposit().into(),
            },
        }
    }
}
