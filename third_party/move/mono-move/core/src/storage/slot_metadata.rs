// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! State value metadata of the storage slots a transaction writes.

use aptos_types::{
    on_chain_config::CurrentTimeMicroseconds, state_store::state_value::StateValueMetadata,
};

/// Deposit prices for the storage slots transactions write.
//
// TODO(metering): the prices are placeholders. Take them from the gas
// schedule.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlotMetadataContext {
    /// Deposit for a new slot.
    pub slot_deposit_price: u64,
    /// Deposit for each byte of a slot's key and value.
    pub byte_deposit_price: u64,
}

impl Default for SlotMetadataContext {
    fn default() -> Self {
        Self {
            slot_deposit_price: 400_000,
            byte_deposit_price: 400,
        }
    }
}

impl SlotMetadataContext {
    /// The metadata of a slot written with a value of `new_size` bytes. `old`
    /// is the slot's metadata and value size as read, or [`None`] if the slot
    /// did not exist. A new slot takes a deposit for itself and for its bytes.
    /// An existing slot keeps its metadata, and its bytes deposit grows with
    /// the value, up to what its new size costs.
    pub fn written_metadata(
        &self,
        old: Option<(&StateValueMetadata, u64)>,
        new_size: u64,
        key_size: u64,
        creation_time_usecs: u64,
    ) -> StateValueMetadata {
        let bytes_deposit = key_size
            .saturating_add(new_size)
            .saturating_mul(self.byte_deposit_price);
        let Some((old, old_size)) = old else {
            return StateValueMetadata::new(
                self.slot_deposit_price,
                bytes_deposit,
                &CurrentTimeMicroseconds {
                    microseconds: creation_time_usecs,
                },
            );
        };
        let mut metadata = old.clone();
        let growth = new_size
            .saturating_sub(old_size)
            .saturating_mul(self.byte_deposit_price);
        let top_up = growth.min(bytes_deposit.saturating_sub(old.bytes_deposit()));
        if top_up > 0 {
            metadata
                .maybe_upgrade()
                .set_bytes_deposit(old.bytes_deposit().saturating_add(top_up));
        }
        metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY_SIZE: u64 = 10;

    fn context() -> SlotMetadataContext {
        SlotMetadataContext {
            slot_deposit_price: 1000,
            byte_deposit_price: 3,
        }
    }

    fn stored(
        slot_deposit: u64,
        bytes_deposit: u64,
        creation_time_usecs: u64,
    ) -> StateValueMetadata {
        StateValueMetadata::new(slot_deposit, bytes_deposit, &CurrentTimeMicroseconds {
            microseconds: creation_time_usecs,
        })
    }

    #[test]
    fn new_slot_takes_slot_and_bytes_deposits() {
        let metadata = context().written_metadata(None, 20, KEY_SIZE, 77);
        assert_eq!(metadata, stored(1000, 90, 77));
    }

    #[test]
    fn existing_slot_deposits_only_for_growth() {
        let written = |old: &StateValueMetadata, old_size, new_size| {
            context().written_metadata(Some((old, old_size)), new_size, KEY_SIZE, 77)
        };
        let before = stored(1000, 90, 5);

        // Grows by 4 bytes.
        assert_eq!(written(&before, 20, 24), stored(1000, 102, 5));
        // Shrinks: the deposit stays.
        assert_eq!(written(&before, 20, 8), before);
        // Grows, but the deposit already covers the new size.
        let covered = stored(1000, 500, 5);
        assert_eq!(written(&covered, 20, 24), covered);
        // Grows past a deposit that covers part of the growth.
        assert_eq!(written(&stored(1000, 100, 5), 20, 24), stored(1000, 102, 5));
        // A slot without metadata gets it once it takes a deposit.
        let none = StateValueMetadata::none();
        assert_eq!(written(&none, 20, 24), stored(0, 12, 0));
        assert!(written(&none, 20, 20).is_none());
    }
}
