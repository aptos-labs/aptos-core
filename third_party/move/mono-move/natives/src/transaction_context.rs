// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `transaction_context` module, plus the extension backing them.

use crate::{monomorphic_natives, NativeEntry};
use aptos_types::transaction::authenticator::AuthenticationKey;
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeExtension, NativeStatus, TableHandle},
    VMResult,
};
use move_core_types::account_address::AccountAddress;
use sha3::{Digest, Sha3_256};

/// Per-transaction context backing the `transaction_context` natives.
//
// TODO(completeness): the "no user transaction context at all" case (e.g. view
// functions) is not modeled; the legacy VM aborts those natives with
// ETRANSACTION_CONTEXT_NOT_AVAILABLE.
pub struct TransactionContextExtension {
    txn_hash: [u8; 32],
    /// Chain id of the network the transaction runs on.
    chain_id: u8,
    /// AUIDs issued so far in this transaction.
    auid_counter: u64,
    /// Per-session counter feeding the low bits of the monotonic counter.
    local_counter: u16,
    /// Identifies the session within the transaction.
    session_counter: u8,
    /// The transaction's index within its block plus the monotonic counter's
    /// top byte (0 for block execution, 1 for validation/simulation), or
    /// `None` when the execution context provides no index.
    transaction_index: Option<(u32, u8)>,
    /// Tables created so far in this transaction; seeds the next table handle.
    table_counter: u32,
}

impl TransactionContextExtension {
    pub fn new(
        txn_hash: [u8; 32],
        chain_id: u8,
        session_counter: u8,
        transaction_index: Option<(u32, u8)>,
    ) -> Self {
        Self {
            txn_hash,
            chain_id,
            auid_counter: 0,
            local_counter: 0,
            session_counter,
            transaction_index,
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

/// Table handle: `sha3_256(txn_hash || table_count_be_u32)`.
fn table_handle(txn_hash: &[u8], table_count: u32) -> TableHandle {
    let mut hasher = Sha3_256::new();
    hasher.update(txn_hash);
    hasher.update(table_count.to_be_bytes());
    TableHandle::new(AccountAddress::new(hasher.finalize().into()))
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

/// `error::invalid_state(ETRANSACTION_INDEX_NOT_AVAILABLE)` in
/// `0x1::transaction_context` (category 3, reason 5).
const INDEX_NOT_AVAILABLE_ABORT_CODE: u64 = (3 << 16) | 5;

/// `0x1::transaction_context::generate_unique_address(): address`
///
/// Returns a freshly derived address, which is guaranteed to be unique within
/// the transaction.
//
// TODO(metering): charge gas.
pub fn native_generate_unique_address<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let mut ext = ctx.get_extension::<TransactionContextExtension>()?;
    ext.auid_counter += 1;
    let address =
        AuthenticationKey::auid(ext.txn_hash.to_vec(), ext.auid_counter).account_address();
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

    // Like the legacy VM, abort when the execution context provides no
    // transaction index.
    let Some((transaction_index, reserved_byte)) = ext.transaction_index else {
        return Ok(NativeStatus::Abort {
            code: INDEX_NOT_AVAILABLE_ABORT_CODE,
            message: Some("transaction index is not available in this execution context".into()),
        });
    };

    let counter = ((reserved_byte as u128) << 120)
        | ((timestamp_us as u128) << 56)
        | ((transaction_index as u128) << 24)
        | ((ext.session_counter as u128) << 16)
        | (ext.local_counter as u128);
    // SAFETY: return 0 is `u128`.
    unsafe { ctx.set_return(0, counter)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::chain_id_internal(): u8` and the identical
/// `0x1::type_info::chain_id_internal(): u8`.
///
/// Returns the chain id of the network the transaction runs on.
//
// TODO(metering): charge gas.
pub fn native_chain_id<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ext = ctx.get_extension::<TransactionContextExtension>()?;
    // SAFETY: return 0 is `u8`.
    unsafe { ctx.set_return(0, ext.chain_id)? };
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
        (
            "0x1::transaction_context::chain_id_internal",
            native_chain_id
        ),
    ]
}
