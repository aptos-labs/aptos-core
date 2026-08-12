// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `randomness` module, plus the extension backing them.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeExtension, NativeStatus},
    VMResult,
};

/// Raised when a transaction that may be biased draws randomness.
const E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT: u64 = 1;

/// The randomness state of one transaction: the counter each draw consumes, and whether the
/// transaction may draw at all.
///
/// A transaction that can inspect a random value and then abort can retry until the value suits
/// it, which is a bias the chain cannot detect. The framework's defence is that only an entry
/// function annotated `#[randomness]` may draw, because such a call is charged its whole gas budget
/// up front; the executor decides that per transaction and this carries the answer.
pub struct RandomnessContext {
    /// The 8-byte counter that distinguishes a transaction's draws from each other. Little-endian
    /// increment, matching what the legacy VM hands to the same framework code.
    txn_local_state: [u8; 8],
    unbiasable: bool,
}

impl RandomnessContext {
    pub fn new(unbiasable: bool) -> Self {
        Self {
            txn_local_state: [0; 8],
            unbiasable,
        }
    }

    fn increment(&mut self) {
        for byte in self.txn_local_state.iter_mut() {
            if *byte < 255 {
                *byte += 1;
                break;
            }
            *byte = 0;
        }
    }
}

impl NativeExtension for RandomnessContext {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {}

    fn on_checkpoint(&mut self) {}

    /// The counter is not restored. Two draws in one transaction must differ whatever happened
    /// between them, and a rolled-back transaction's draws are not observable anyway.
    fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
        Ok(())
    }
}

/// `0x1::randomness::fetch_and_increment_txn_counter(): vector<u8>`
///
/// The current counter, after which the next call sees a different one. Aborts if the transaction
/// may be biased; see [`RandomnessContext`].
//
// TODO(metering): charge gas.
pub fn native_fetch_and_increment_txn_counter<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let counter = {
        let mut randomness = ctx.get_extension::<RandomnessContext>()?;
        if !randomness.unbiasable {
            return Ok(NativeStatus::Abort {
                code: E_API_USE_SUSCEPTIBLE_TO_TEST_AND_ABORT,
                message: None,
            });
        }
        let counter = randomness.txn_local_state;
        randomness.increment();
        counter
    };
    let out = ctx.new_byte_vector(&counter)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::randomness::is_unbiasable(): bool`
///
/// Whether this transaction may draw randomness; see [`RandomnessContext`].
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
