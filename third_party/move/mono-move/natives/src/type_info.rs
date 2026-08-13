// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `type_info` module.

use crate::{polymorphic_natives, NativeEntry};
use mono_move_core::{
    native::{NativeContext, NativeContextFamily, NativeStatus, RootPool, VMValue, Vector},
    types::{type_to_string, view_name, view_type, view_type_list, Type},
    VMResult,
};
use move_core_types::account_address::AccountAddress;

/// `0x1::type_info::type_name<T>(): String`
///
/// Returns the fully-qualified name of `T` as a string.
//
// TODO(metering): charge gas for the (currently unbounded) type traversal.
//
// TODO(correctness, metering): `type_to_string` is a placeholder — check it against the canonical
// string format the legacy VM uses.
//
// TODO(completeness): with monomorphization the name is known at specialization time, so the
// specializer could write it directly rather than going through a native.
pub fn native_type_name<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let name = type_to_string(ctx.ty_arg(0)?);
    let bytes = ctx.new_byte_vector(name.as_bytes())?;
    // SAFETY: structs are flattened inline rather than heap-boxed, so the
    // single-field `String { bytes: vector<u8> }` has the same representation as
    // a bare `vector<u8>` — an 8-byte pointer to the byte vector. There is no
    // separate struct header.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// Abort code raised when `type_of` is given a non-struct type. Matches the
/// code the legacy VM uses for this native.
const EXPECTED_STRUCT_ABORT_CODE: u64 = 1;

const TYPE_INFO_MODULE_NAME_OFFSET: usize = 32;
const TYPE_INFO_STRUCT_NAME_OFFSET: usize = 40;

/// Rust representation of `aptos_std::type_info::TypeInfo`, returned by [`native_type_of`].
struct TypeInfo<'a> {
    account_address: AccountAddress,
    module_name: Vector<'a, u8>,
    struct_name: Vector<'a, u8>,
}

impl<'a> VMValue<'a> for TypeInfo<'a> {
    const FRAME_SLOT_SIZE: usize = TYPE_INFO_STRUCT_NAME_OFFSET + 8;

    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        unsafe {
            let account_address = AccountAddress::read_from_frame(pool, frame_ptr, offset);
            let module_name =
                Vector::read_from_frame(pool, frame_ptr, offset + TYPE_INFO_MODULE_NAME_OFFSET);
            let struct_name =
                Vector::read_from_frame(pool, frame_ptr, offset + TYPE_INFO_STRUCT_NAME_OFFSET);
            TypeInfo {
                account_address,
                module_name,
                struct_name,
            }
        }
    }

    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            self.account_address.write_to_frame(frame_ptr, offset);
            self.module_name
                .write_to_frame(frame_ptr, offset + TYPE_INFO_MODULE_NAME_OFFSET);
            self.struct_name
                .write_to_frame(frame_ptr, offset + TYPE_INFO_STRUCT_NAME_OFFSET);
        }
    }
}

/// `0x1::type_info::type_of<T>(): TypeInfo`
///
/// Reflection API that gives `T`'s defining address, module name, and type name.
/// Aborts if `T` is not a struct.
//
// TODO(completeness): with monomorphization `T` is fully known at specialization time, so the
// specializer could synthesize this `TypeInfo` directly rather than via a native.
//
// TODO(correctness): double check that the result matches the legacy VM's completely.
pub fn native_type_of<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let (address, module_name, struct_name) = match view_type(ctx.ty_arg(0)?) {
        Type::Nominal {
            module_id,
            name,
            ty_args,
            ..
        } => {
            // SAFETY: interned ids are valid for the executable's lifetime.
            let module_id = unsafe { module_id.as_ref_unchecked() };
            let mut struct_name = view_name(*name).to_string();
            let ty_args = view_type_list(*ty_args);
            if !ty_args.is_empty() {
                struct_name.push('<');
                for (i, arg) in ty_args.iter().enumerate() {
                    if i > 0 {
                        struct_name.push_str(", ");
                    }
                    struct_name.push_str(&type_to_string(*arg));
                }
                struct_name.push('>');
            }
            (
                *module_id.address(),
                view_name(module_id.name()),
                struct_name,
            )
        },
        other => {
            return Ok(NativeStatus::Abort {
                code: EXPECTED_STRUCT_ABORT_CODE,
                message: Some(format!(
                    "Expected a struct type, found: {}",
                    other.short_name()
                )),
            })
        },
    };
    let module_name = ctx.new_byte_vector(module_name.as_bytes())?;
    let struct_name = ctx.new_byte_vector(struct_name.as_bytes())?;
    let info = TypeInfo {
        account_address: address,
        module_name,
        struct_name,
    };
    // SAFETY: return 0 is `TypeInfo`.
    unsafe { ctx.set_return(0, info)? };
    Ok(NativeStatus::Success)
}

/// Natives for the `type_info` module.
pub fn make_all_type_info_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::type_info::type_name", native_type_name),
        ("0x1::type_info::type_of", native_type_of),
    ]
}
