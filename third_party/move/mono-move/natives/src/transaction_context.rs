// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `transaction_context` module, plus the extension backing them.

use crate::{
    address_derivation::{auid_address, table_handle},
    monomorphic_natives, NativeEntry,
};
use aptos_types::{error, transaction::user_transaction_context::UserTransactionContext};
use mono_move_core::{
    native::{
        NativeContext, NativeContextFamily, NativeExtension, NativeStatus, TableHandle, VMValue,
    },
    VMResult,
};
use move_core_types::account_address::AccountAddress;

/// Carries transaction context information, such as transaction hashes or chain
/// ID.
pub struct TransactionContextExtension {
    txn_hash: [u8; 32],
    /// Hash of the script executed by the transaction, or empty when the payload
    /// is not a script.
    script_hash: Vec<u8>,
    /// Chain ID of the network the transaction runs on.
    network_chain_id: u8,
    /// AUIDs issued so far in this transaction.
    auid_counter: u64,
    /// Per-session counter feeding the low bits of the monotonic counter.
    local_counter: u16,
    /// Identifies the session within the transaction.
    session_counter: u8,
    /// Tables created so far in this transaction; seeds the next table handle.
    table_counter: u32,
    /// Set to [`None`] when not a user transaction, and set otherwise.
    user_transaction_context: Option<UserTransactionContext>,
}

impl TransactionContextExtension {
    pub fn new(
        txn_hash: [u8; 32],
        script_hash: Vec<u8>,
        network_chain_id: u8,
        session_counter: u8,
        user_transaction_context: Option<UserTransactionContext>,
    ) -> Self {
        Self {
            txn_hash,
            script_hash,
            network_chain_id,
            auid_counter: 0,
            local_counter: 0,
            session_counter,
            table_counter: 0,
            user_transaction_context,
        }
    }

    /// Chain ID of the network the transaction runs on.
    pub(crate) fn network_chain_id(&self) -> u8 {
        self.network_chain_id
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

const MONOTONICALLY_INCREASING_COUNTER_OVERFLOW: u64 = error::invalid_state(2);

const TRANSACTION_CONTEXT_NOT_AVAILABLE: u64 = error::invalid_state(1);

const TXN_INDEX_NOT_AVAILABLE: u64 = error::invalid_state(5);

fn txn_context_unavailable_msg(subject: &str) -> String {
    format!(
        "Transaction context is not available ({subject} can only be accessed during transaction execution)"
    )
}

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
            code: MONOTONICALLY_INCREASING_COUNTER_OVERFLOW,
            message: Some(
                "Monotonically increasing counter has reached maximum value (too many calls in this session)".into(),
            ),
        });
    }
    ext.local_counter += 1;

    let Some(user_transaction_context) = &ext.user_transaction_context else {
        return Ok(NativeStatus::Abort {
            code: TRANSACTION_CONTEXT_NOT_AVAILABLE,
            message: Some(
                "Transaction context is not available (must be called during transaction execution)"
                    .into(),
            ),
        });
    };

    let Some((reserved_byte, transaction_index)) = user_transaction_context
        .transaction_index_kind()
        .reserved_byte_and_transaction_index()
    else {
        return Ok(NativeStatus::Abort {
            code: TXN_INDEX_NOT_AVAILABLE,
            message: Some("Transaction index is not available in this execution context".into()),
        });
    };

    let counter = (reserved_byte << 120)
        | ((timestamp_us as u128) << 56)
        | ((transaction_index as u128) << 24)
        | ((ext.session_counter as u128) << 16)
        | (ext.local_counter as u128);
    // SAFETY: return 0 is `u128`.
    unsafe { ctx.set_return(0, counter)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::chain_id_internal(): u8`
///
/// Returns the chain ID specified for the current user transaction. This might
/// be different from the network chain ID.
pub fn native_transaction_context_chain_id<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `chain_id` returns `u8`, matching the native's `u8` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "chain ID",
            UserTransactionContext::user_txn_chain_id,
        )
    }
}

/// `0x1::transaction_context::sender_internal(): address`
pub fn native_sender<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `sender` returns `AccountAddress`, matching the native's `address` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "sender information",
            UserTransactionContext::sender,
        )
    }
}

/// `0x1::transaction_context::gas_payer_internal(): address`
pub fn native_gas_payer<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `gas_payer` returns `AccountAddress`, matching the native's `address` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "gas payer information",
            UserTransactionContext::gas_payer,
        )
    }
}

/// `0x1::transaction_context::max_gas_amount_internal(): u64`
pub fn native_max_gas_amount<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `max_gas_amount` returns `u64`, matching the native's `u64` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "max gas amount",
            UserTransactionContext::max_gas_amount,
        )
    }
}

/// `0x1::transaction_context::gas_unit_price_internal(): u64`
pub fn native_gas_unit_price<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `gas_unit_price` returns `u64`, matching the native's `u64` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "gas unit price",
            UserTransactionContext::gas_unit_price,
        )
    }
}

/// `0x1::transaction_context::is_encrypted_txn_internal(): bool`
pub fn native_is_encrypted_txn<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `is_encrypted_txn` returns `bool`, matching the native's `bool` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "is_encrypted_txn",
            UserTransactionContext::is_encrypted_txn,
        )
    }
}

/// `0x1::transaction_context::get_txn_hash(): vector<u8>`
//
// TODO(metering): charge gas.
pub fn native_get_txn_hash<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let txn_hash = ctx.get_extension::<TransactionContextExtension>()?.txn_hash;
    let bytes = ctx.new_byte_vector(&txn_hash)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::get_script_hash(): vector<u8>`
//
// TODO(metering): charge gas.
pub fn native_get_script_hash<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let script_hash = ctx
        .get_extension::<TransactionContextExtension>()?
        .script_hash
        .clone();
    let bytes = ctx.new_byte_vector(&script_hash)?;
    // SAFETY: return 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::secondary_signers_internal(): vector<address>`
//
// TODO(metering): charge gas.
pub fn native_secondary_signers<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let signers = {
        let ext = ctx.get_extension::<TransactionContextExtension>()?;
        let Some(user_transaction_context) = &ext.user_transaction_context else {
            return Ok(NativeStatus::Abort {
                code: TRANSACTION_CONTEXT_NOT_AVAILABLE,
                message: Some(txn_context_unavailable_msg("secondary signers")),
            });
        };
        user_transaction_context.secondary_signers()
    };
    let mut data = Vec::with_capacity(signers.len() * AccountAddress::LENGTH);
    for signer in &signers {
        data.extend_from_slice(signer.as_ref());
    }
    let vector =
        ctx.new_vector_no_pointers(AccountAddress::LENGTH as u32, signers.len() as u64, &data)?;
    // SAFETY: return 0 is `vector<address>`; addresses are pointer-free.
    unsafe { ctx.set_return(0, vector)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::is_orderless_txn_internal(): bool`
pub fn native_is_orderless_txn<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: `is_orderless_txn` returns `bool`, matching the native's `bool` return.
    unsafe {
        return_user_transaction_context_field(
            ctx,
            "is_orderless_txn",
            UserTransactionContext::is_orderless_txn,
        )
    }
}

/// `0x1::transaction_context::is_orderless_txn_internal_for_test_only(): bool`
///
/// This is a test-only function but not gated behind `#[test_only]`. So it has
/// to stay with other production natives.
//
// TODO(metering): charge gas.
pub fn native_is_orderless_txn_for_test_only<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ext = ctx.get_extension::<TransactionContextExtension>()?;
    let is_orderless = ext
        .user_transaction_context
        .as_ref()
        .is_some_and(UserTransactionContext::is_orderless_txn);
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, is_orderless)? };
    Ok(NativeStatus::Success)
}

/// `0x1::transaction_context::monotonically_increasing_counter_internal_for_test_only(): u128`
///
/// This is a test-only function but not gated behind `#[test_only]`. So it has
/// to stay with other production natives.
//
// TODO(metering): charge gas.
pub fn native_monotonically_increasing_counter_for_test_only<C: NativeContext>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    let mut ext = ctx.get_extension::<TransactionContextExtension>()?;
    if ext.local_counter == u16::MAX {
        return Ok(NativeStatus::Abort {
            code: MONOTONICALLY_INCREASING_COUNTER_OVERFLOW,
            message: Some(
                "Monotonically increasing counter has reached maximum value (too many calls in this session)".into(),
            ),
        });
    }
    ext.local_counter += 1;
    let counter = ext.local_counter as u128;
    // SAFETY: return 0 is `u128`.
    unsafe { ctx.set_return(0, counter)? };
    Ok(NativeStatus::Success)
}

/// Reads one field from the user transaction context and returns it. Aborts if
/// user transaction context is not available.
///
/// # Safety
///
/// Caller should guarantee the return type is correct.
//
// TODO(metering): charge gas.
unsafe fn return_user_transaction_context_field<'a, C, T>(
    ctx: &'a C,
    subject: &str,
    read: impl FnOnce(&UserTransactionContext) -> T,
) -> VMResult<NativeStatus>
where
    C: NativeContext,
    T: VMValue<'a>,
{
    let ext = ctx.get_extension::<TransactionContextExtension>()?;
    let Some(user_transaction_context) = &ext.user_transaction_context else {
        return Ok(NativeStatus::Abort {
            code: TRANSACTION_CONTEXT_NOT_AVAILABLE,
            message: Some(txn_context_unavailable_msg(subject)),
        });
    };
    let value = read(user_transaction_context);
    // SAFETY: the caller ensures return type and field type match.
    unsafe { ctx.set_return(0, value)? };
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
            native_transaction_context_chain_id
        ),
        ("0x1::transaction_context::sender_internal", native_sender),
        (
            "0x1::transaction_context::gas_payer_internal",
            native_gas_payer
        ),
        (
            "0x1::transaction_context::max_gas_amount_internal",
            native_max_gas_amount
        ),
        (
            "0x1::transaction_context::gas_unit_price_internal",
            native_gas_unit_price
        ),
        (
            "0x1::transaction_context::is_encrypted_txn_internal",
            native_is_encrypted_txn
        ),
        (
            "0x1::transaction_context::get_txn_hash",
            native_get_txn_hash
        ),
        (
            "0x1::transaction_context::get_script_hash",
            native_get_script_hash
        ),
        (
            "0x1::transaction_context::secondary_signers_internal",
            native_secondary_signers
        ),
        (
            "0x1::transaction_context::is_orderless_txn_internal",
            native_is_orderless_txn
        ),
        (
            "0x1::transaction_context::is_orderless_txn_internal_for_test_only",
            native_is_orderless_txn_for_test_only
        ),
        (
            "0x1::transaction_context::monotonically_increasing_counter_internal_for_test_only",
            native_monotonically_increasing_counter_for_test_only
        ),
    ]
}
