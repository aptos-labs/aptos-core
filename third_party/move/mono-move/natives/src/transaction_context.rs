// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `transaction_context` module, plus the extension backing them.

use crate::{
    address_derivation::{auid_address, table_handle},
    monomorphic_natives, NativeEntry,
};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeExtension, NativeStatus, TableHandle},
    VMResult,
};

/// Per-transaction context backing the `transaction_context` natives.
//
// TODO(completeness): source `transaction_index` / `reserved_byte` from aptos-core's
// `UserTransactionContext` rather than carrying them as plain fields, and model
// the "transaction index not available" case.
pub struct TransactionContextExtension {
    txn_hash: Vec<u8>,
    /// AUIDs issued so far in this transaction.
    auid_counter: u64,
    /// Per-session counter feeding the low bits of the monotonic counter.
    local_counter: u16,
    /// Identifies the session within the transaction.
    session_counter: u8,
    /// Index of the transaction within its block/chunk.
    transaction_index: u32,
    /// Top byte of the monotonic counter (0 for block execution, 1 for
    /// validation/simulation).
    reserved_byte: u8,
    /// Tables created so far in this transaction; seeds the next table handle.
    table_counter: u32,
}

impl TransactionContextExtension {
    pub fn new(
        txn_hash: Vec<u8>,
        session_counter: u8,
        transaction_index: u32,
        reserved_byte: u8,
    ) -> Self {
        Self {
            txn_hash,
            auid_counter: 0,
            local_counter: 0,
            session_counter,
            transaction_index,
            reserved_byte,
            table_counter: 0,
        }
    }

    /// Mints the next table handle, advancing the per-transaction counter.
    pub fn next_table_handle(&mut self) -> TableHandle {
        let handle = table_handle(&self.txn_hash, self.table_counter);
        self.table_counter += 1;
        handle
    }
}

impl NativeExtension for TransactionContextExtension {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {}

    // TODO(completeness): In the legacy VM, the AUID and the monotonic counter reset at every session start.
    // However because each legacy VM session gets a fresh `session_hash` / `session_counter`,
    // the derived IDs and counters are still guaranteed to be unique across the whole transaction.
    // In some sense, this is a mechanism to accommodate the legacy VM's insufficient session model.
    //
    // Mono Move's session natively supports checkpoints and rollbacks, so there is no need to
    // create multiple sessions. AUID and the monotonic counter are naturally unique as long as we
    // do not reset them.
    //
    // This is likely fine for their intended use cases, but it does break binary compatibility with
    // the legacy implementation. We'll need to revisit this and see if it ends up being a real
    // problem. Perhaps a compatibility mode is needed if we want to replay existing transactions
    // using the new VM?
    fn on_checkpoint(&mut self) {}

    fn on_rollback(&mut self, _n: usize) -> VMResult<()> {
        Ok(())
    }
}

/// `error::invalid_state(EMONOTONICALLY_INCREASING_COUNTER_OVERFLOW)` in
/// `0x1::transaction_context` (category 3, reason 2).
const COUNTER_OVERFLOW_ABORT_CODE: u64 = (3 << 16) | 2;

/// `0x1::transaction_context::generate_unique_address(): address`
///
/// Returns a freshly derived address, which is guaranteed to be unique within
/// the transaction.
//
// TODO(metering): charge gas.
pub fn native_generate_unique_address<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let mut ext = ctx.get_extension::<TransactionContextExtension>()?;
    ext.auid_counter += 1;
    let address = auid_address(&ext.txn_hash, ext.auid_counter);
    // SAFETY: return 0 is `address`.
    unsafe { ctx.set_return(0, address)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::monotonically_increasing_counter_internal(timestamp_us: u64): u128`
///
/// Packs `reserved_byte || timestamp_us || transaction_index || session_counter
/// || local_counter` into a value that strictly increases within a session.
//
// TODO(metering): charge gas.
pub fn native_monotonically_increasing_counter_internal<C: NativeContext>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `timestamp_us: u64`.
    let timestamp_us = unsafe { ctx.arg::<u64>(0)? };

    let mut ext = ctx.get_extension::<TransactionContextExtension>()?;
    if ext.local_counter == u16::MAX {
        return Ok(NativeStatus::Abort {
            code: COUNTER_OVERFLOW_ABORT_CODE,
            message: Some("monotonically increasing counter overflow".into()),
        });
    }
    ext.local_counter += 1;

    let counter = ((ext.reserved_byte as u128) << 120)
        | ((timestamp_us as u128) << 56)
        | ((ext.transaction_index as u128) << 24)
        | ((ext.session_counter as u128) << 16)
        | (ext.local_counter as u128);
    // SAFETY: return 0 is `u128`.
    unsafe { ctx.set_return(0, counter)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `transaction_context` module.
pub fn make_all_transaction_context_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::transaction_context::generate_unique_address",
            native_generate_unique_address
        ),
        (
            "0x1::transaction_context::monotonically_increasing_counter_internal",
            native_monotonically_increasing_counter_internal
        ),
    ]
}
