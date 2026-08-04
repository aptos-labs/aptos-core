// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `table` module.
//!
//! Table entries are global-storage items keyed by `(handle, serialized_key)`,
//! so they flow through the same read-write set as resources — no table-specific
//! extension is needed. `new_table_handle`'s per-transaction counter lives on
//! the transaction-context extension.

use crate::{polymorphic_natives, transaction_context::TransactionContextExtension, NativeEntry};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Ref,
        TableHandle,
    },
    VMResult,
};

/// Table entry already exists (`error::invalid_argument(100)`).
const ALREADY_EXISTS: u64 = (100 << 8) + 7;
/// Table entry not found (`error::invalid_argument(101)`).
const NOT_FOUND: u64 = (101 << 8) + 7;

// TODO(cleanup): revisit these abort codes alongside the other native abort codes -- they
// are unhelpful as-is. In particular, a missing entry on borrow could surface a
// runtime error (as resource borrows do) rather than an abort that hides the
// reason.

/// `0x1::table::new_table_handle<K, V>(): address`
//
// TODO(metering): charge gas.
pub fn native_new_table_handle<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let handle = ctx
        .get_extension::<TransactionContextExtension>()?
        .next_table_handle();
    // SAFETY: return 0 is `address`, and `TableHandle` has the same layout as address.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// Shared helper that
/// - Reads the table handle (arg 0, a `&[mut] Table`)
/// - Reads the key (arg 1) and serializes it
fn handle_and_key<C: NativeContext>(ctx: &C) -> VMResult<(Ref<'_, TableHandle>, Vec<u8>)> {
    // SAFETY: arg 0 is `&[mut] Table<K, V>`, which has the same representation
    // as `&TableHandle` — its single `handle` field.
    let handle: Ref<TableHandle> = unsafe { ctx.arg(0)? };
    let key = ctx.bcs_serialize_arg(1, ctx.ty_arg(0)?)?;
    Ok((handle, key))
}

/// `0x1::table::add_box<K, V, B>(table: &mut Table<K, V>, key: K, val: Box<V>)`
//
// TODO(metering): charge gas.
pub fn native_add_box<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let (handle, key) = handle_and_key(ctx)?;
    let descriptor = ctx
        .required_descriptor(0)
        .ok_or_else(|| native_invariant_violation("add_box: missing value descriptor".into()))?;
    // Arg 2 is the `Box<V>` value; box it onto the heap before storing it.
    let value = ctx.box_arg(2, descriptor)?;
    if ctx.table_add(handle.get(), &key, value, ctx.ty_arg(2)?)? {
        Ok(NativeStatus::Success)
    } else {
        Ok(NativeStatus::Abort {
            code: ALREADY_EXISTS,
            message: None,
        })
    }
}

/// Borrows the entry, writing the reference into return slot 0, or aborts if
/// the entry is missing.
fn borrow_box<C: NativeContext>(ctx: &C, mutable: bool) -> VMResult<NativeStatus> {
    let (handle, key) = handle_and_key(ctx)?;
    match ctx.table_borrow(handle.get(), &key, mutable, ctx.ty_arg(2)?)? {
        // SAFETY: return 0 is the `&[mut] Box<V>` reference.
        Some(r) => unsafe { ctx.set_return(0, r) }.map(|()| NativeStatus::Success),
        None => Ok(NativeStatus::Abort {
            code: NOT_FOUND,
            message: None,
        }),
    }
}

/// `0x1::table::borrow_box<K, V, B>(table: &Table<K, V>, key: K): &Box<V>`
//
// TODO(metering): charge gas.
pub fn native_borrow_box<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    borrow_box(ctx, false)
}

/// `0x1::table::borrow_box_mut<K, V, B>(table: &mut Table<K, V>, key: K): &mut Box<V>`
//
// TODO(metering): charge gas.
pub fn native_borrow_box_mut<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    borrow_box(ctx, true)
}

/// `0x1::table::contains_box<K, V, B>(table: &Table<K, V>, key: K): bool`
//
// TODO(metering): charge gas.
pub fn native_contains_box<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let (handle, key) = handle_and_key(ctx)?;
    let exists = ctx.table_contains(handle.get(), &key, ctx.ty_arg(2)?)?;
    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, exists)? };
    Ok(NativeStatus::Success)
}

/// `0x1::table::remove_box<K, V, B>(table: &mut Table<K, V>, key: K): Box<V>`
//
// TODO(metering): charge gas.
pub fn native_remove_box<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let (handle, key) = handle_and_key(ctx)?;
    match ctx.table_remove(handle.get(), &key, ctx.ty_arg(2)?)? {
        Some(value) => {
            let size = ctx.return_size(0)?;
            // SAFETY: the entry was boxed from a `Box<V>` value, so its payload
            // begins with that value's in-frame bytes, and `size` is the return
            // slot's size.
            let bytes = unsafe { std::slice::from_raw_parts(value.ptr(), size) };
            // SAFETY: return 0 is `Box<V>`, which `bytes` is the representation of.
            unsafe { ctx.set_return_raw(0, bytes)? };
            Ok(NativeStatus::Success)
        },
        None => Ok(NativeStatus::Abort {
            code: NOT_FOUND,
            message: None,
        }),
    }
}

/// `0x1::table::destroy_empty_box<K, V, B>(table: &Table<K, V>)`
//
// TODO(metering): charge gas.
pub fn native_destroy_empty_box<C: NativeContext>(_ctx: &C) -> VMResult<NativeStatus> {
    // Table entries are ordinary read-write-set items, so an empty table owns no
    // state to reclaim.
    Ok(NativeStatus::Success)
}

/// `0x1::table::drop_unchecked_box<K, V, B>(table: Table<K, V>)`
//
// TODO(metering): charge gas.
pub fn native_drop_unchecked_box<C: NativeContext>(_ctx: &C) -> VMResult<NativeStatus> {
    // A table handle owns no heap state, so dropping it is a no-op.
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
        ("0x1::table::remove_box", native_remove_box),
        ("0x1::table::destroy_empty_box", native_destroy_empty_box),
        ("0x1::table::drop_unchecked_box", native_drop_unchecked_box),
    ]
}
