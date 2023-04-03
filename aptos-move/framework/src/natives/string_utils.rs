// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    natives::helpers::{make_safe_native, SafeNativeContext, SafeNativeError, SafeNativeResult},
    safely_pop_arg,
};
use aptos_types::on_chain_config::{Features, TimedFeatures};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{GasQuantity, InternalGas},
    language_storage::TypeTag,
    u256,
    value::{MoveStructLayout, MoveTypeLayout},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Reference, Struct, Value},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, fmt::Write, ops::Deref, sync::Arc};
use move_core_types::value::MoveFieldLayout;

pub fn format_vector(
    context: &mut SafeNativeContext,
    base_gas: InternalGas,
    v: Vec<(&MoveTypeLayout, Value)>,
    out: &mut String,
) -> SafeNativeResult<()> {
    for (i, (ty, val)) in v.into_iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        native_format_impl(context, base_gas, ty, val, out)?;
    };
    Ok(())
}

pub fn format_vector_with_fields(
    context: &mut SafeNativeContext,
    base_gas: InternalGas,
    fields: &[MoveFieldLayout],
    strct: Struct,
    out: &mut String,
) -> SafeNativeResult<()> {
    for (i, (ty, x)) in fields.iter().zip(strct.unpack()?).enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "{}: ", ty.name).unwrap();
        native_format_impl(context, base_gas, &ty.layout, x, out)?;
    }
    Ok(())
}

pub fn native_format_impl(
    context: &mut SafeNativeContext,
    base_gas: InternalGas,
    ty: &MoveTypeLayout,
    val: Value,
    out: &mut String,
) -> SafeNativeResult<()> {
    context.charge(base_gas)?;
    match ty {
        MoveTypeLayout::Bool => {
            let b = val.value_as::<bool>()?;
            write!(out, "{}", b).unwrap();
        },
        MoveTypeLayout::U8 => {
            let u = val.value_as::<u8>()?;
            write!(out, "{}", u).unwrap();
        },
        MoveTypeLayout::U64 => {
            let u = val.value_as::<u64>()?;
            write!(out, "{}", u).unwrap();
        },
        MoveTypeLayout::U128 => {
            let u = val.value_as::<u128>()?;
            write!(out, "{}", u).unwrap();
        },
        MoveTypeLayout::U16 => {
            let u = val.value_as::<u16>()?;
            write!(out, "{}", u).unwrap();
        },
        MoveTypeLayout::U32 => {
            let u = val.value_as::<u32>()?;
            write!(out, "{}", u).unwrap();
        },
        MoveTypeLayout::U256 => {
            let u = val.value_as::<u256::U256>()?;
            write!(out, "{}", u).unwrap();
        },
        MoveTypeLayout::Address => {
            let addr = val.value_as::<move_core_types::account_address::AccountAddress>()?;
            write!(out, "{}", addr).unwrap();
        },
        MoveTypeLayout::Signer => {
            let signer = val.value_as::<move_core_types::account_address::AccountAddress>()?;
            write!(out, "{}", signer).unwrap();
        },
        MoveTypeLayout::Vector(ty) => {
            let v = val.value_as::<Vec<Value>>()?;
            out.push('[');
            format_vector(context, base_gas, v.into_iter().map(|x| (ty.as_ref(), x)).collect(), out)?;
            out.push(']');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, fields, .. }) => {
            if type_.name.as_str() == "String"
                && type_.module.as_str() == "string"
                && type_.address == AccountAddress::ONE
            {
                let v = val
                    .value_as::<Struct>()?
                    .unpack()?
                    .next()
                    .unwrap()
                    .value_as::<Vec<u8>>()?;
                write!(out, "\"{}\"", std::str::from_utf8(&v).unwrap()).unwrap();
                return Ok(());
            }
            let strct = val.value_as::<Struct>()?;
            write!(out, "{} {{", type_.name.as_str()).unwrap();
            format_vector_with_fields(context, base_gas, &fields, strct, out)?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            format_vector_with_fields(context, base_gas, &fields, strct, out)?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            format_vector(context, base_gas, fields.iter().zip(strct.unpack()?).collect(), out)?;
            out.push('}');
        },
    };
    Ok(())
}

pub fn native_format(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    let ty = context
        .deref()
        .type_to_fully_annotated_layout(&ty_args[0])?
        .unwrap();
    let x = safely_pop_arg!(arguments, Reference);
    let v = x.read_ref().map_err(SafeNativeError::InvariantViolation)?;
    let mut out = String::new();
    native_format_impl(context, gas_params.base, &ty, v, &mut out)?;
    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

pub fn native_format_list(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);
    let mut list_ty = &ty_args[0];

    let val = safely_pop_arg!(arguments, Reference);
    let mut val = val
        .read_ref()
        .map_err(SafeNativeError::InvariantViolation)?;

    let fmt = safely_pop_arg!(arguments, Reference);
    let fmt = fmt
        .read_ref()
        .map_err(SafeNativeError::InvariantViolation)?;
    let fmt = fmt
        .value_as::<Struct>()?
        .unpack()?
        .next()
        .unwrap()
        .value_as::<Vec<u8>>()?;
    let fmt = std::str::from_utf8(&fmt).unwrap();

    context.charge(gas_params.per_byte * GasQuantity::from(fmt.len() as u64))?;

    let arg_mismatch = 1;
    let invalid_fmt = 2;

    let match_list_ty = |context: &mut SafeNativeContext, list_ty, name| {
        if let TypeTag::Struct(struct_tag) = context
            .type_to_type_tag(list_ty)
            .map_err(SafeNativeError::InvariantViolation)?
        {
            if !(struct_tag.address == AccountAddress::ONE
                && struct_tag.module.as_str() == "string_utils"
                && struct_tag.name.as_str() == name)
            {
                return Err(SafeNativeError::Abort {
                    abort_code: arg_mismatch,
                });
            }
            Ok(())
        } else {
            Err(SafeNativeError::Abort {
                abort_code: arg_mismatch,
            })
        }
    };

    let mut out = String::new();
    let mut in_braces = false;
    let mut in_escape = false;
    for c in fmt.chars() {
        if !in_escape && c == '\\' {
            in_escape = true;
            continue;
        } else if !in_escape && c == '{' {
            if in_braces {
                return Err(SafeNativeError::Abort {
                    abort_code: invalid_fmt,
                });
            }
            in_braces = true;
        } else if !in_escape && c == '}' {
            if !in_braces {
                return Err(SafeNativeError::Abort {
                    abort_code: invalid_fmt,
                });
            }
            in_braces = false;
            // verify`that the type is a list
            match_list_ty(context, list_ty, "Cons")?;

            // We know that the type is a list, so we can safely unwrap
            let ty_args = if let Type::StructInstantiation(_, ty_args) = list_ty {
                ty_args
            } else {
                unreachable!()
            };
            let mut it = val.value_as::<Struct>()?.unpack()?;
            let car = it.next().unwrap();
            val = it.next().unwrap();
            list_ty = &ty_args[1];

            let ty = context
                .deref()
                .type_to_fully_annotated_layout(&ty_args[0])?
                .unwrap();
            native_format_impl(context, gas_params.base, &ty, car, &mut out)?;
        } else if !in_braces {
            out.push(c);
        }
        in_escape = false;
    }
    if in_escape || in_braces {
        return Err(SafeNativeError::Abort {
            abort_code: invalid_fmt,
        });
    }
    match_list_ty(context, list_ty, "NIL")?;

    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGas,
}

pub fn make_all(
    gas_param: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let natives = [
        (
            "format",
            make_safe_native(
                gas_param.clone(),
                timed_features.clone(),
                features.clone(),
                native_format,
            ),
        ),
        (
            "format_list",
            make_safe_native(gas_param, timed_features, features, native_format_list),
        ),
    ];

    crate::natives::helpers::make_module_natives(natives)
}
