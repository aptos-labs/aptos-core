// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `type_info` module.

use crate::{
    monomorphic_natives, polymorphic_natives, transaction_context::TransactionContextExtension,
    NativeEntry,
};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, RootPool,
        VMValue, Vector,
    },
    type_tag_of,
    types::type_to_string,
    VMResult,
};
use move_core_types::{account_address::AccountAddress, language_storage::TypeTag};

/// `0x1::type_info::type_name<T>(): String`
///
/// Returns the canonical type name of `T`.
//
// TODO(metering): charge gas for the (currently unbounded) type traversal.
//
// TODO(completeness): with monomorphization the name is known at specialization time, so the
// specializer could write it directly rather than going through a native.
pub fn native_type_name<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let name = type_tag_of_arg(ctx, 0)?.to_canonical_string();
    let bytes = ctx.new_byte_vector(name.as_bytes())?;
    // SAFETY: structs are flattened inline rather than heap-boxed, so the
    // single-field `String { bytes: vector<u8> }` has the same representation as
    // a bare `vector<u8>` — an 8-byte pointer to the byte vector. There is no
    // separate struct header.
    unsafe { ctx.set_return(0, bytes)? };
    Ok(NativeStatus::Success)
}

/// The [`TypeTag`] of the native's type argument at `index`.
fn type_tag_of_arg<C: NativeContext>(ctx: &C, index: usize) -> VMResult<TypeTag> {
    let ty = ctx.ty_arg(index)?;
    type_tag_of(ty).ok_or_else(|| {
        native_invariant_violation(format!(
            "type argument has no type tag: {}",
            type_to_string(ty)
        ))
    })
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
pub fn native_type_of<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let type_tag = type_tag_of_arg(ctx, 0)?;
    let TypeTag::Struct(struct_tag) = type_tag else {
        return Ok(NativeStatus::Abort {
            code: EXPECTED_STRUCT_ABORT_CODE,
            message: Some(format!(
                "Expected a struct type, found: {}",
                type_tag.to_short_string()
            )),
        });
    };
    let struct_name = if struct_tag.type_args.is_empty() {
        struct_tag.name.to_string()
    } else {
        let type_args = struct_tag
            .type_args
            .iter()
            .map(TypeTag::to_canonical_string)
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}<{}>", struct_tag.name, type_args)
    };
    let module_name = ctx.new_byte_vector(struct_tag.module.as_bytes())?;
    let struct_name = ctx.new_byte_vector(struct_name.as_bytes())?;
    let info = TypeInfo {
        account_address: struct_tag.address,
        module_name,
        struct_name,
    };
    // SAFETY: return 0 is `TypeInfo`.
    unsafe { ctx.set_return(0, info)? };
    Ok(NativeStatus::Success)
}

/// `0x1::type_info::chain_id_internal(): u8`
///
/// Returns the chain ID of the network, and is therefore always available. This
/// is NOT the chain ID of the user transaction, which may be different and not
/// even available for some transaction types.
//
// TODO(metering): charge gas.
pub fn native_type_info_chain_id<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let ext = ctx.get_extension::<TransactionContextExtension>()?;
    // SAFETY: return 0 is `u8`.
    unsafe { ctx.set_return(0, ext.network_chain_id())? };
    Ok(NativeStatus::Success)
}

/// Natives for the `type_info` module.
pub fn make_all_type_info_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    let mut natives = polymorphic_natives![
        ("0x1::type_info::type_name", native_type_name),
        ("0x1::type_info::type_of", native_type_of),
    ];
    // `chain_id_internal` is non-generic, so it registers as a monomorphic entry
    // with empty type arguments. Unlike `transaction_context::chain_id_internal`,
    // it is always available and never aborts on a missing user transaction
    // context.
    natives.extend(monomorphic_natives![(
        "0x1::type_info::chain_id_internal",
        native_type_info_chain_id
    )]);
    natives
}
