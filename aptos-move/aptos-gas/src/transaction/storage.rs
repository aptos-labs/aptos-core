// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{
    on_chain_config::StorageGasSchedule, state_store::state_key::StateKey, write_set::WriteOp,
};
use move_core_types::gas_algebra::{
    InternalGas, InternalGasPerArg, InternalGasPerByte, NumArgs, NumBytes,
};

#[derive(Clone, Debug)]
pub struct StorageGasParameters {
    pub per_item_read: InternalGasPerArg,
    pub per_item_create: InternalGasPerArg,
    pub per_item_write: InternalGasPerArg,
    pub per_byte_read: InternalGasPerByte,
    pub per_byte_create: InternalGasPerByte,
    pub per_byte_write: InternalGasPerByte,
}

impl From<StorageGasSchedule> for StorageGasParameters {
    fn from(gas_schedule: StorageGasSchedule) -> Self {
        Self {
            per_item_read: gas_schedule.per_item_read.into(),
            per_item_create: gas_schedule.per_item_create.into(),
            per_item_write: gas_schedule.per_item_write.into(),
            per_byte_read: gas_schedule.per_byte_read.into(),
            per_byte_create: gas_schedule.per_byte_create.into(),
            per_byte_write: gas_schedule.per_byte_write.into(),
        }
    }
}

impl StorageGasParameters {
    pub fn zeros() -> Self {
        Self {
            per_item_read: 0.into(),
            per_item_create: 0.into(),
            per_item_write: 0.into(),
            per_byte_read: 0.into(),
            per_byte_create: 0.into(),
            per_byte_write: 0.into(),
        }
    }
}

impl StorageGasParameters {
    pub fn calculate_write_set_gas<'a>(
        &self,
        ops: impl IntoIterator<Item = (&'a StateKey, &'a WriteOp)>,
        feature_version: u64,
    ) -> InternalGas {
        use aptos_types::write_set::WriteOp::*;

        let mut num_items_create = NumArgs::zero();
        let mut num_items_write = NumArgs::zero();
        let mut num_bytes_create = NumBytes::zero();
        let mut num_bytes_write = NumBytes::zero();

        for (key, op) in ops.into_iter() {
            match &op {
                Creation(data) => {
                    num_items_create += 1.into();
                    num_bytes_create += Self::write_op_size(key, data, feature_version);
                }
                Modification(data) => {
                    num_items_write += 1.into();
                    num_bytes_write += Self::write_op_size(key, data, feature_version);
                }
                Deletion => (),
            }
        }

        num_items_create * self.per_item_create
            + num_items_write * self.per_item_write
            + num_bytes_create * self.per_byte_create
            + num_bytes_write * self.per_byte_write
    }

    fn write_op_size(key: &StateKey, value: &[u8], feature_version: u64) -> NumBytes {
        let value_size = NumBytes::new(value.len() as u64);

        if feature_version > 2 {
            let key_size = NumBytes::new(key.size() as u64);
            let kb = NumBytes::new(1024);
            (key_size + value_size)
                .checked_sub(kb)
                .unwrap_or(NumBytes::zero())
        } else {
            let key_size = NumBytes::new(
                key.encode()
                    .expect("Should be able to serialize state key")
                    .len() as u64,
            );
            key_size + value_size
        }
    }
}
