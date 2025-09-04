// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::natives::transaction_context::NativeTransactionContext;
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_core_types::{
    gas_algebra::NumBytes,
    language_storage::{StructTag, TypeTag},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, fmt::Write};

fn type_of_internal(struct_tag: &StructTag) -> Result<SmallVec<[Value; 1]>, std::fmt::Error> {
    let mut name = struct_tag.name.to_string();
    if let Some(first_ty) = struct_tag.type_args.first() {
        write!(name, "<")?;
        write!(name, "{}", first_ty.to_canonical_string())?;
        for ty in struct_tag.type_args.iter().skip(1) {
            write!(name, ", {}", ty.to_canonical_string())?;
        }
        write!(name, ">")?;
    }

    let struct_value = Struct::pack(vec![
        Value::address(struct_tag.address),
        Value::vector_u8(struct_tag.module.as_bytes().to_vec()),
        Value::vector_u8(name.as_bytes().to_vec()),
    ]);
    Ok(smallvec![Value::struct_(struct_value)])
}

/***************************************************************************************************
 * native fun type_of
 *
 *   Returns the structs Module Address, Module Name and the Structs Name.
 *
 *   gas cost: base_cost + unit_cost * type_size
 *
 **************************************************************************************************/
fn native_type_of(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.is_empty());

    context.charge(TYPE_INFO_TYPE_OF_BASE)?;

    let type_tag = context.type_to_type_tag(&ty_args[0])?;

    if context.eval_gas(TYPE_INFO_TYPE_OF_PER_BYTE_IN_STR) > 0.into() {
        let type_tag_str = type_tag.to_canonical_string();
        // Ideally, we would charge *before* the `type_to_type_tag()` and `type_tag.to_string()` calls above.
        // But there are other limits in place that prevent this native from being called with too much work.
        context
            .charge(TYPE_INFO_TYPE_OF_PER_BYTE_IN_STR * NumBytes::new(type_tag_str.len() as u64))?;
    }

    if let TypeTag::Struct(struct_tag) = type_tag {
        Ok(type_of_internal(&struct_tag).expect("type_of should never fail."))
    } else {
        Err(SafeNativeError::Abort {
            abort_code: super::status::NFE_EXPECTED_STRUCT_TYPE_TAG,
        })
    }
}

/***************************************************************************************************
 * native fun type_name
 *
 *   Returns a string representing the TypeTag of the parameter.
 *
 *   gas cost: base_cost + unit_cost * type_size
 *
 **************************************************************************************************/
fn native_type_name(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.is_empty());

    context.charge(TYPE_INFO_TYPE_NAME_BASE)?;

    let type_tag = context.type_to_type_tag(&ty_args[0])?;
    let type_name = type_tag.to_canonical_string();

    // TODO: Ideally, we would charge *before* the `type_to_type_tag()` and `type_tag.to_string()` calls above.
    context.charge(TYPE_INFO_TYPE_NAME_PER_BYTE_IN_STR * NumBytes::new(type_name.len() as u64))?;

    Ok(smallvec![Value::struct_(Struct::pack(vec![
        Value::vector_u8(type_name.as_bytes().to_vec())
    ]))])
}

/***************************************************************************************************
 * native fun chain_id
 *
 *   Returns the chain ID
 *
 *   gas cost: base_cost
 *
 **************************************************************************************************/
fn native_chain_id(
    context: &mut SafeNativeContext,
    _ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(_ty_args.is_empty());
    debug_assert!(arguments.is_empty());

    context.charge(TYPE_INFO_CHAIN_ID_BASE)?;

    let chain_id = context
        .extensions()
        .get::<NativeTransactionContext>()
        .chain_id();

    Ok(smallvec![Value::u8(chain_id)])
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("type_of", native_type_of as RawSafeNative),
        ("type_name", native_type_name),
        ("chain_id_internal", native_chain_id),
    ];

    builder.make_named_natives(natives)
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::{account_address::AccountAddress, identifier::Identifier};
    use move_vm_types::values::VMValueCast;

    #[test]
    fn test_type_of_internal() {
        let dummy_st = StructTag {
            address: AccountAddress::random(),
            module: Identifier::new("DummyModule").unwrap(),
            name: Identifier::new("DummyStruct").unwrap(),
            type_args: vec![TypeTag::Vector(Box::new(TypeTag::U8))],
        };

        let dummy_as_strings = dummy_st.to_canonical_string();
        let mut dummy_as_strings = dummy_as_strings.split("::");
        let dummy_as_type_of = type_of_internal(&dummy_st).unwrap().pop().unwrap();
        let dummy_as_type_of: Struct = dummy_as_type_of.cast().unwrap();
        let mut dummy_as_type_of = dummy_as_type_of.unpack().unwrap();

        let account_addr =
            AccountAddress::from_hex_literal(dummy_as_strings.next().unwrap()).unwrap();
        assert!(Value::address(account_addr)
            .equals(&dummy_as_type_of.next().unwrap())
            .unwrap());
        let module = dummy_as_strings.next().unwrap().as_bytes().to_owned();
        assert!(Value::vector_u8(module)
            .equals(&dummy_as_type_of.next().unwrap())
            .unwrap());
        let name = dummy_as_strings.next().unwrap().as_bytes().to_owned();
        assert!(Value::vector_u8(name)
            .equals(&dummy_as_type_of.next().unwrap())
            .unwrap());
    }
}
