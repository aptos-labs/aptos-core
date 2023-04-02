// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeResult},
    safely_pop_arg,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, sync::Arc};
use std::any::Any;
use std::ops::Deref;
use std::fmt::Write;
use move_binary_format::errors::PartialVMError;
use move_core_types::u256;
use move_core_types::{value::{MoveStructLayout, MoveTypeLayout}, gas_algebra::InternalGas};
use move_core_types::account_address::AccountAddress;
use move_core_types::language_storage::TypeTag;
use move_core_types::vm_status::StatusCode;
use move_vm_types::loaded_data::runtime_types::StructType;
use move_vm_types::values::{Reference, Struct};
use crate::natives::helpers::SafeNativeError;

pub fn native_format_impl(
    context: &mut SafeNativeContext,
    ty: &MoveTypeLayout,
    val: Value,
    out: &mut String
) -> SafeNativeResult<()> {
    match ty {
        MoveTypeLayout::Bool => {
            let b = val.value_as::<bool>()?;
            write!(out, "{}", b).unwrap();
        }
        MoveTypeLayout::U8 => {
            let u = val.value_as::<u8>()?;
            write!(out, "{}", u).unwrap();
        }
        MoveTypeLayout::U64 => {
            let u = val.value_as::<u64>()?;
            write!(out, "{}", u).unwrap();
        }
        MoveTypeLayout::U128 => {
            let u = val.value_as::<u128>()?;
            write!(out, "{}", u).unwrap();
        }
        MoveTypeLayout::U16 => {
            let u = val.value_as::<u16>()?;
            write!(out, "{}", u).unwrap();
        }
        MoveTypeLayout::U32 => {
            let u = val.value_as::<u32>()?;
            write!(out, "{}", u).unwrap();
        }
        MoveTypeLayout::U256 => {
            let u = val.value_as::<u256::U256>()?;
            write!(out, "{}", u).unwrap();
        }
        MoveTypeLayout::Address => {
            let addr = val.value_as::<move_core_types::account_address::AccountAddress>()?;
            write!(out, "{}", addr).unwrap();
        }
        MoveTypeLayout::Signer => {
            let signer = val.value_as::<move_core_types::account_address::AccountAddress>()?;
            write!(out, "{}", signer).unwrap();
        }
        MoveTypeLayout::Vector(ty) => {
            let v = val.value_as::<Vec<Value>>()?;
            out.push('[');
            for (i, x) in v.into_iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                native_format_impl(context, ty.as_ref(), x, out)?;
            }
            out.push(']');
        }
        MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, fields, .. }) => {
            let strct = val.value_as::<Struct>()?;
            out.push_str(type_.name.as_str());
            out.push_str(" {");
            for (i, (ty, x)) in fields.iter().zip(strct.unpack()?).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write!(out, "{}: ", ty.name).unwrap();
                native_format_impl(context, &ty.layout, x, out)?;
            }
            out.push('}');
        }
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            for (i, (ty, x)) in fields.iter().zip(strct.unpack()?).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write!(out, "{}: ", ty.name).unwrap();
                native_format_impl(context, &ty.layout, x, out)?;
            }
            out.push('}');
        }
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('(');
            for (i, (ty, x)) in fields.iter().zip(strct.unpack()?).enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                native_format_impl(context, ty, x, out)?;
            }
            out.push(')');
        }
    };
    Ok(())
}

pub fn native_format(
    _gas_params: &FormatGasParams,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
    ) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    let ty = context.deref().type_to_fully_annotated_layout(&ty_args[0])?.unwrap();
    let x = safely_pop_arg!(arguments, Reference);
    let v = x.read_ref().map_err(|e| SafeNativeError::InvariantViolation(e))?;
    let mut out = String::new();
    native_format_impl(context, &ty, v, &mut out)?;
    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

pub fn native_format_list(
    _gas_params: &FormatGasParams,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    let mut list_ty = &ty_args[0];

    let val = safely_pop_arg!(arguments, Reference);
    let mut val = val.read_ref().map_err(|e| SafeNativeError::InvariantViolation(e))?;

    let fmt = safely_pop_arg!(arguments, Reference);
    let fmt = fmt.read_ref().map_err(|e| SafeNativeError::InvariantViolation(e))?;
    let fmt = fmt.value_as::<Struct>()?.unpack()?.next().unwrap().value_as::<Vec<u8>>()?;
    let fmt = std::str::from_utf8(&fmt).unwrap();


    let mut out = String::new();
    let mut in_braces = false;
    let mut in_escape = false;
    let arg_mismatch = 1;
    for c in fmt.chars() {
        if !in_escape && c == '\\' {
            in_escape = true;
            continue;
        } else if !in_escape && c == '{' {
            in_braces = true;
        } else if !in_escape && c == '}' {
            in_braces = false;
            if let Type::StructInstantiation(idx, ty_args) = list_ty {
                // verify`that the type is a list
                if let TypeTag::Struct(struct_tag) = context.type_to_type_tag(list_ty).map_err(|e| SafeNativeError::InvariantViolation(e))? {
                    if !(struct_tag.address == AccountAddress::ONE && struct_tag.module.as_str() == "string_utils" && struct_tag.name.as_str() == "List") {
                        return Err(SafeNativeError::Abort { abort_code: arg_mismatch });
                    }
                } else {
                    return Err(SafeNativeError::Abort { abort_code: arg_mismatch });
                }
                let mut it = val.value_as::<Struct>()?.unpack()?;
                let car = it.next().unwrap();
                val = it.next().unwrap();
                list_ty = &ty_args[1];

                let ty = context.deref().type_to_fully_annotated_layout(&ty_args[0])?.unwrap();
                native_format_impl(context, &ty, car, &mut out)?;
            } else {
                return Err(SafeNativeError::Abort { abort_code: arg_mismatch });
            }
        } else if !in_braces {
            out.push(c);
        }
        in_escape = false;
    }
    if let TypeTag::Struct(struct_tag) = context.type_to_type_tag(list_ty).map_err(|e| SafeNativeError::InvariantViolation(e))? {
        if !(struct_tag.address == AccountAddress::ONE && struct_tag.module.as_str() == "string_utils" && struct_tag.name.as_str() == "NIL") {
            return Err(SafeNativeError::Abort { abort_code: arg_mismatch });
        }
    } else {
        return Err(SafeNativeError::Abort { abort_code: arg_mismatch });
    }
    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

#[derive(Debug, Clone)]
pub struct FormatGasParams {
    //pub base: InternalGas,
}

pub fn make_all(
    gas_param: FormatGasParams,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [(
            "format",
            make_safe_native(gas_param.clone(), timed_features.clone(), features.clone(), native_format),
        ),
        (
            "format_list",
            make_safe_native(gas_param, timed_features, features, native_format_list),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}

