// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `table` module.
//!
//! Table entries are global-storage items keyed by `(handle, serialized_key)`,
//! so they flow through the same read-write set as resources — no table-specific
//! extension is needed. `new_table_handle`'s per-transaction counter lives on
//! the transaction-context extension.

use crate::{polymorphic_natives, transaction_context::TransactionContextExtension, NativeEntry};
use mono_move_core::native::{
    NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref, VMInternalError,
};
use move_core_types::account_address::AccountAddress;

/// Table entry already exists (`error::invalid_argument(100)`).
const ALREADY_EXISTS: u64 = (100 << 8) + 7;
/// Table entry not found (`error::invalid_argument(101)`).
const NOT_FOUND: u64 = (101 << 8) + 7;

/// `0x1::table::new_table_handle<K, V>(): address`
//
// TODO: charge gas.
pub fn native_new_table_handle<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let handle = ctx
        .get_extension::<TransactionContextExtension>()?
        .next_table_handle();
    // SAFETY: return 0 is `address`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// Reads the table handle (arg 0, `&[mut] Table`) and the serialized key (arg 1).
fn handle_and_key<C: NativeContext>(ctx: &C) -> Result<(AccountAddress, Vec<u8>), VMInternalError> {
    // SAFETY: arg 0 is `&[mut] Table<K, V>`, which has the same representation
    // as `&address` — its single `handle` field.
    let table: Ref<Opaque> = unsafe { ctx.arg(0)? };
    // SAFETY: the referent's first 32 bytes are the table handle.
    let handle = unsafe { core::ptr::read_unaligned(table.ptr() as *const AccountAddress) };
    let key = match ctx.bcs_serialize_arg(1, ctx.ty_arg(0)?)? {
        Ok(bytes) => bytes,
        // A key is a `copy + drop` value, so a failure here is not user-facing.
        Err(e) => {
            return Err(VMInternalError::invariant_violation(format!(
                "table key serialization failed: {e}"
            )))
        },
    };
    Ok((handle, key))
}

/// `0x1::table::add_box<K, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>)`
//
// TODO: charge gas.
pub fn native_add_box<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let (handle, key) = handle_and_key(ctx)?;
    if ctx.table_add(handle, &key, 2)? {
        Ok(NativeStatus::Success)
    } else {
        Ok(NativeStatus::Abort {
            code: ALREADY_EXISTS,
            message: None,
        })
    }
}

/// `0x1::table::borrow_box<K, V, B>(table: &Table<K, V>, key: K): &Box<V>`
//
// TODO: charge gas.
pub fn native_borrow_box<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let (handle, key) = handle_and_key(ctx)?;
    if ctx.table_borrow(handle, &key, 0, false)? {
        Ok(NativeStatus::Success)
    } else {
        Ok(NativeStatus::Abort {
            code: NOT_FOUND,
            message: None,
        })
    }
}

/// `0x1::table::borrow_box_mut<K, V, B>(table: &mut Table<K, V>, key: K): &mut Box<V>`
//
// TODO: charge gas.
pub fn native_borrow_box_mut<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let (handle, key) = handle_and_key(ctx)?;
    if ctx.table_borrow(handle, &key, 0, true)? {
        Ok(NativeStatus::Success)
    } else {
        Ok(NativeStatus::Abort {
            code: NOT_FOUND,
            message: None,
        })
    }
}

/// `0x1::table::contains_box<K, V, B>(table: &Table<K, V>, key: K): bool`
//
// TODO: charge gas.
pub fn native_contains_box<C: NativeContext>(ctx: &C) -> Result<NativeStatus, VMInternalError> {
    let (handle, key) = handle_and_key(ctx)?;
    let exists = ctx.table_contains(handle, &key)?;
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, exists)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `table` module.
pub fn make_all_table_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::table::new_table_handle", native_new_table_handle),
        ("0x1::table::add_box", native_add_box),
        ("0x1::table::borrow_box", native_borrow_box),
        ("0x1::table::borrow_box_mut", native_borrow_box_mut),
        ("0x1::table::contains_box", native_contains_box),
    ]
}
