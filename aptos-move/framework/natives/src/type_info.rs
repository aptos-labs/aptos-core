// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::transaction_context::NativeTransactionContext;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_binary_format::errors::PartialVMError;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::NumBytes, identifier::Identifier,
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

/// Builds the returned `TypeInfo` value from a pre-rendered canonical name.
fn type_of_internal(
    address: AccountAddress,
    module_name: &Identifier,
    canonical_name: &str,
) -> Result<SmallVec<[Value; 1]>, std::fmt::Error> {
    let struct_value = Struct::pack(vec![
        Value::address(address),
        Value::vector_u8(module_name.as_bytes().to_vec()),
        Value::vector_u8(canonical_name.as_bytes().to_vec()),
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
    ty_args: &[Type],
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.is_empty());

    context.charge(TYPE_INFO_TYPE_OF_BASE)?;

    let canonical_str = context.type_to_canonical_string(&ty_args[0])?;
    // A zero charge would still show up in gas-profiling traces.
    if context.eval_gas(TYPE_INFO_TYPE_OF_PER_BYTE_IN_STR) > 0.into() {
        context.charge(
            TYPE_INFO_TYPE_OF_PER_BYTE_IN_STR * NumBytes::new(canonical_str.len() as u64),
        )?;
    }

    match context.struct_address_and_name(&ty_args[0])? {
        Some((address, module_name)) => {
            // The name is everything after the second `::`.
            let canonical_name = canonical_str.splitn(3, "::").nth(2).ok_or_else(|| {
                SafeNativeError::InvariantViolation(PartialVMError::new_invariant_violation(
                    format!("Malformed canonical struct type name: {}", canonical_str),
                ))
            })?;
            Ok(type_of_internal(address, &module_name, canonical_name)
                .expect("type_of should never fail."))
        },
        None => Err(SafeNativeError::abort_with_message(
            super::status::NFE_EXPECTED_STRUCT_TYPE_TAG,
            format!(
                "Expected a struct type, found: {}",
                ty_args[0].to_short_string()
            ),
        )),
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
    ty_args: &[Type],
    arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    debug_assert!(arguments.is_empty());

    context.charge(TYPE_INFO_TYPE_NAME_BASE)?;

    let type_name = context.type_to_canonical_string(&ty_args[0])?;

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
    _ty_args: &[Type],
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
    use move_core_types::{
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
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
        // The name part: everything after the second `::`, as the native passes it.
        let dummy_as_name = {
            let mut parts = dummy_as_strings.clone();
            parts.next();
            parts.next();
            parts.next().unwrap().to_owned()
        };
        let dummy_as_type_of = type_of_internal(dummy_st.address, &dummy_st.module, &dummy_as_name)
            .unwrap()
            .pop()
            .unwrap();
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
