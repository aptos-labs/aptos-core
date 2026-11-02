// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `randomness` module, plus the extension backing them.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeExtension, NativeStatus},
    VMResult,
};

/// Raised when a payload that may be biased asks for randomness.
const E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT: u64 = 1;

/// Per-transaction state backing the `0x1::randomness` natives.
pub struct RandomnessContext {
    counter: u64,
    unbiasable: bool,
}

impl RandomnessContext {
    pub fn new() -> Self {
        Self {
            counter: 0,
            unbiasable: false,
        }
    }

    /// Allows the payload to call the randomness API.
    pub fn mark_unbiasable(&mut self) {
        self.unbiasable = true;
    }
}

impl Default for RandomnessContext {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeExtension for RandomnessContext {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {}

    fn on_checkpoint(&mut self) {}

    // The counter is deliberately not rewound: a value handed out before a
    // rollback must not be handed out again. This matches
    // `TransactionContextExtension`.
    fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
        Ok(())
    }
}

/// `0x1::randomness::fetch_and_increment_txn_counter(): vector<u8>`
//
// TODO(metering): charge gas.
pub fn native_fetch_and_increment_txn_counter<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let mut ext = ctx.get_extension::<RandomnessContext>()?;
    if !ext.unbiasable {
        return Ok(NativeStatus::Abort {
            code: E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT,
            message: Some(
                "randomness is only available to an entry function annotated `#[randomness]`"
                    .into(),
            ),
        });
    }
    let counter = ext.counter.to_le_bytes();
    ext.counter = ext.counter.wrapping_add(1);
    // The borrow must end before allocating, which can trigger a collection.
    drop(ext);

    let bytes = ctx.new_byte_vector(&counter)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// `0x1::randomness::is_unbiasable(): bool`
//
// TODO(metering): charge gas.
pub fn native_is_unbiasable<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let unbiasable = ctx.get_extension::<RandomnessContext>()?.unbiasable;
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, unbiasable)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `randomness` module.
pub fn make_all_randomness_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::randomness::fetch_and_increment_txn_counter",
            native_fetch_and_increment_txn_counter
        ),
        ("0x1::randomness::is_unbiasable", native_is_unbiasable),
    ]
}
