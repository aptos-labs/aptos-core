// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `state_storage` module, plus the extension backing them.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{
        NativeContext, NativeContextFamily, NativeExtension, NativeStatus, RootPool, VMValue,
    },
    VMResult,
};

/// State-storage usage captured at the epoch boundary, served read-only to the
/// `state_storage` native.
///
/// This is different from the legacy VM, which reads this from a borrowed
/// host view, but the effects shall be the same.
pub struct StorageUsageAtEpochBoundary {
    items: u64,
    bytes: u64,
}

impl StorageUsageAtEpochBoundary {
    pub fn new(items: u64, bytes: u64) -> Self {
        Self { items, bytes }
    }
}

impl NativeExtension for StorageUsageAtEpochBoundary {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {}

    fn on_checkpoint(&mut self) {}

    fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
        Ok(())
    }
}

const USAGE_BYTES_OFFSET: usize = 8;

/// Rust mirror of `0x1::state_storage::Usage`.
struct Usage {
    items: u64,
    bytes: u64,
}

impl<'a> VMValue<'a> for Usage {
    const FRAME_SLOT_SIZE: usize = USAGE_BYTES_OFFSET + 8;

    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        unsafe {
            Usage {
                items: u64::read_from_frame(pool, frame_ptr, offset),
                bytes: u64::read_from_frame(pool, frame_ptr, offset + USAGE_BYTES_OFFSET),
            }
        }
    }

    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            self.items.write_to_frame(frame_ptr, offset);
            self.bytes
                .write_to_frame(frame_ptr, offset + USAGE_BYTES_OFFSET);
        }
    }
}

/// `0x1::state_storage::get_state_storage_usage_only_at_epoch_beginning(): Usage`
//
// TODO(metering): charge gas (constant cost) once the gas API lands.
pub fn native_get_state_storage_usage_only_at_epoch_beginning<C: NativeContext>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    let ext = ctx.get_extension::<StorageUsageAtEpochBoundary>()?;
    let usage = Usage {
        items: ext.items,
        bytes: ext.bytes,
    };
    // SAFETY: return 0 is `Usage { items: u64, bytes: u64 }`.
    unsafe { ctx.set_return(0, usage)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `state_storage` module.
pub fn make_all_state_storage_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![(
        "0x1::state_storage::get_state_storage_usage_only_at_epoch_beginning",
        native_get_state_storage_usage_only_at_epoch_beginning
    ),]
}
