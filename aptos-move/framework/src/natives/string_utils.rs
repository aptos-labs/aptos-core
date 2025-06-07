// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_gas_algebra::NumBytes;
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_std::iterable::Iterable;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::TypeTag,
    u256,
    value::{MoveFieldLayout, MoveStructLayout, MoveTypeLayout, MASTER_ADDRESS_FIELD_OFFSET},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    value_serde::FunctionValueExtension,
    values::{Closure, Reference, Struct, Value, Vector, VectorRef},
};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, fmt::Write, ops::Deref};

// Error codes from Move contracts:
const EARGS_MISMATCH: u64 = 1;
const EINVALID_FORMAT: u64 = 2;
const EUNABLE_TO_FORMAT_DELAYED_FIELD: u64 = 3;

struct FormatContext<'a, 'b, 'c, 'd, 'e> {
    context: &'e mut SafeNativeContext<'a, 'b, 'c, 'd>,
    should_charge_gas: bool,
    max_depth: usize,
    max_len: usize,
    type_tag: bool,
    canonicalize: bool,
    single_line: bool,
    include_int_type: bool,
}

/// Converts a `MoveValue::Vector` of `u8`'s to a `String` by wrapping it in double quotes and
/// escaping double quotes and backslashes.
///
/// Examples:
///  - 'Hello' returns "Hello"
///  - '"Hello?" What are you saying?' returns "\"Hello?\" What are you saying?"
///  - '\ and " are escaped' returns "\\ and \" are escaped"
fn bytes_as_escaped_string(buf: &str) -> String {
    let str = String::from(buf);

    // We need to escape displayed double quotes " as \" and, as a result, also escape
    // displayed \ as \\.
    str.replace('\\', "\\\\").replace('"', "\\\"")
}

fn print_space_or_newline(newline: bool, out: &mut String, depth: usize) {
    if newline {
        out.push('\n');
        for _ in 0..depth {
            // add 2 spaces
            write!(out, "  ").unwrap();
        }
    } else {
        out.push(' ');
    }
}

fn primitive_type(ty: &MoveTypeLayout) -> bool {
    !matches!(ty, MoveTypeLayout::Vector(_) | MoveTypeLayout::Struct(_))
}

trait MoveLayout {
    fn write_name(&self, out: &mut String);
    fn get_layout(&self) -> &MoveTypeLayout;
}

impl MoveLayout for MoveFieldLayout {
    fn write_name(&self, out: &mut String) {
        write!(out, "{}: ", self.name).unwrap();
    }

    fn get_layout(&self) -> &MoveTypeLayout {
        &self.layout
    }
}

impl MoveLayout for MoveTypeLayout {
    fn write_name(&self, _out: &mut String) {}

    fn get_layout(&self) -> &MoveTypeLayout {
        self
    }
}

fn format_vector<'a>(
    context: &mut FormatContext,
    fields: impl Iterator<Item = &'a (impl MoveLayout + 'a)>,
    values: Vec<Value>,
    depth: usize,
    newline: bool,
    out: &mut String,
) -> SafeNativeResult<()> {
    if values.is_empty() {
        return Ok(());
    }
    if depth >= context.max_depth {
        write!(out, " .. ").unwrap();
        return Ok(());
    }
    print_space_or_newline(newline, out, depth + 1);
    for (i, (ty, val)) in fields.zip(values.into_iter()).enumerate() {
        if i > 0 {
            out.push(',');
            print_space_or_newline(newline, out, depth + 1);
        }
        if i >= context.max_len {
            write!(out, "..").unwrap();
            break;
        }
        ty.write_name(out);
        native_format_impl(context, ty.get_layout(), val, depth + 1, out)?;
    }
    print_space_or_newline(newline, out, depth);
    Ok(())
}

fn native_format_impl(
    context: &mut FormatContext,
    layout: &MoveTypeLayout,
    val: Value,
    depth: usize,
    out: &mut String,
) -> SafeNativeResult<()> {
    if context.should_charge_gas {
        context.context.charge(STRING_UTILS_BASE)?;
    }
    let mut suffix = "";
    match layout {
        MoveTypeLayout::Bool => {
            let b = val.value_as::<bool>()?;
            write!(out, "{}", b).unwrap();
        },
        MoveTypeLayout::U8 => {
            let u = val.value_as::<u8>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u8";
        },
        MoveTypeLayout::U64 => {
            let u = val.value_as::<u64>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u64";
        },
        MoveTypeLayout::U128 => {
            let u = val.value_as::<u128>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u128";
        },
        MoveTypeLayout::U16 => {
            let u = val.value_as::<u16>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u16";
        },
        MoveTypeLayout::U32 => {
            let u = val.value_as::<u32>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u32";
        },
        MoveTypeLayout::U256 => {
            let u = val.value_as::<u256::U256>()?;
            write!(out, "{}", u).unwrap();
            suffix = "u256";
        },
        MoveTypeLayout::Address => {
            let addr = val.value_as::<AccountAddress>()?;
            let str = if context.canonicalize {
                addr.to_canonical_string()
            } else {
                addr.to_hex_literal()
            };
            write!(out, "@{}", str).unwrap();
        },
        MoveTypeLayout::Signer => {
            let fix_enabled = context
                .context
                .get_feature_flags()
                .is_enabled(FeatureFlag::SIGNER_NATIVE_FORMAT_FIX);
            let addr = if fix_enabled {
                val.value_as::<Struct>()?
                    .unpack()?
                    // The second field of a signer is always the master address regardless of which variants.
                    .nth(MASTER_ADDRESS_FIELD_OFFSET)
                    .ok_or_else(|| SafeNativeError::Abort {
                        abort_code: EINVALID_FORMAT,
                    })?
                    .value_as::<AccountAddress>()?
            } else {
                val.value_as::<AccountAddress>()?
            };

            let str = if context.canonicalize {
                addr.to_canonical_string()
            } else {
                addr.to_hex_literal()
            };
            if fix_enabled {
                write!(out, "signer(@{})", str).unwrap();
            } else {
                write!(out, "signer({})", str).unwrap();
            }
        },
        MoveTypeLayout::Vector(ty) => {
            if let MoveTypeLayout::U8 = ty.as_ref() {
                let bytes = val.value_as::<Vec<u8>>()?;
                if context.context.timed_feature_enabled(
                    aptos_types::on_chain_config::TimedFeatureFlag::ChargeBytesForPrints,
                ) {
                    context
                        .context
                        .charge(STRING_UTILS_PER_BYTE * NumBytes::new(bytes.len() as u64))?;
                }
                write!(out, "0x{}", hex::encode(bytes)).unwrap();
                return Ok(());
            }
            let values = val.value_as::<Vector>()?.unpack_unchecked()?;
            out.push('[');
            format_vector(
                context,
                std::iter::repeat(ty.as_ref()).take(values.len()),
                values,
                depth,
                !context.single_line && !primitive_type(ty.as_ref()),
                out,
            )?;
            out.push(']');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithTypes { type_, fields, .. }) => {
            let strct = val.value_as::<Struct>()?;
            if type_.name.as_str() == "String"
                && type_.module.as_str() == "string"
                && type_.address == AccountAddress::ONE
            {
                let v = strct.unpack()?.next().unwrap().value_as::<Vec<u8>>()?;
                if context.should_charge_gas {
                    context
                        .context
                        .charge(STRING_UTILS_PER_BYTE * NumBytes::new(v.len() as u64))?;
                }
                write!(
                    out,
                    "\"{}\"",
                    bytes_as_escaped_string(std::str::from_utf8(&v).unwrap())
                )
                .unwrap();
                return Ok(());
            } else if type_.name.as_str() == "Option"
                && type_.module.as_str() == "option"
                && type_.address == AccountAddress::ONE
            {
                let mut v = strct
                    .unpack()?
                    .next()
                    .unwrap()
                    .value_as::<Vector>()?
                    .unpack_unchecked()?;
                if v.is_empty() {
                    out.push_str("None");
                } else {
                    out.push_str("Some(");
                    let inner_ty = if let MoveTypeLayout::Vector(inner_ty) = &fields[0].layout {
                        inner_ty.deref()
                    } else {
                        unreachable!()
                    };
                    native_format_impl(context, inner_ty, v.pop().unwrap(), depth, out)?;
                    out.push(')');
                }
                return Ok(());
            }
            if context.type_tag {
                write!(out, "{} {{", TypeTag::from(type_.clone())).unwrap();
            } else {
                write!(out, "{} {{", type_.name.as_str()).unwrap();
            };
            format_vector(
                context,
                fields.iter(),
                strct.unpack()?.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithFields(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            format_vector(
                context,
                fields.iter(),
                strct.unpack()?.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::Runtime(fields)) => {
            let strct = val.value_as::<Struct>()?;
            out.push('{');
            format_vector(
                context,
                fields.iter(),
                strct.unpack()?.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(variants)) => {
            let struct_value = val.value_as::<Struct>()?;
            let (tag, elems) = struct_value.unpack_with_tag()?;
            if (tag as usize) >= variants.len() {
                return Err(SafeNativeError::Abort {
                    abort_code: EINVALID_FORMAT,
                });
            }
            out.push_str(&format!("#{}{{", tag));
            format_vector(
                context,
                variants[tag as usize].iter(),
                elems.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Struct(MoveStructLayout::WithVariants(variants)) => {
            let struct_value = val.value_as::<Struct>()?;
            let (tag, elems) = struct_value.unpack_with_tag()?;
            if (tag as usize) >= variants.len() {
                return Err(SafeNativeError::Abort {
                    abort_code: EINVALID_FORMAT,
                });
            }
            let variant = &variants[tag as usize];
            out.push_str(&format!("{}{{", variant.name));
            format_vector(
                context,
                variant.fields.iter(),
                elems.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push('}');
        },
        MoveTypeLayout::Function => {
            // Notice that we print the undecorated value representation,
            // avoiding potential loading of the function to get full
            // decorated type information.
            let (fun, args) = val.value_as::<Closure>()?.unpack();
            let data = context
                .context
                .function_value_extension()
                .get_serialization_data(fun.as_ref())?;
            out.push_str(&fun.to_stable_string());
            format_vector(
                context,
                data.captured_layouts.iter(),
                args.collect(),
                depth,
                !context.single_line,
                out,
            )?;
            out.push(')');
        },

        // Return error for native types
        MoveTypeLayout::Native(..) => {
            return Err(SafeNativeError::Abort {
                abort_code: EUNABLE_TO_FORMAT_DELAYED_FIELD,
            })
        },
    };
    if context.include_int_type {
        write!(out, "{}", suffix).unwrap();
    };
    Ok(())
}

/// For old debug implementation
/// TODO: remove when old framework is completely removed
pub(crate) fn native_format_debug(
    context: &mut SafeNativeContext,
    ty: &Type,
    v: Value,
) -> SafeNativeResult<String> {
    // TODO[agg_v2](cleanup): Shift this to annotated layout computation.
    let (_, has_identifier_mappings) = context
        .deref()
        .type_to_type_layout_with_identifier_mappings(ty)?;
    if has_identifier_mappings {
        return Err(SafeNativeError::Abort {
            abort_code: EUNABLE_TO_FORMAT_DELAYED_FIELD,
        });
    }

    let layout = context.deref().type_to_fully_annotated_layout(ty)?;
    let mut format_context = FormatContext {
        context,
        should_charge_gas: false,
        max_depth: usize::MAX,
        max_len: usize::MAX,
        type_tag: true,
        canonicalize: false,
        single_line: false,
        include_int_type: false,
    };
    let mut out = String::new();
    native_format_impl(&mut format_context, &layout, v, 0, &mut out)?;
    Ok(out)
}

fn native_format(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    debug_assert!(ty_args.len() == 1);

    // TODO[agg_v2](cleanup): Shift this to annotated layout computation.
    let (_, has_identifier_mappings) = context
        .deref()
        .type_to_type_layout_with_identifier_mappings(&ty_args[0])?;
    if has_identifier_mappings {
        return Err(SafeNativeError::Abort {
            abort_code: EUNABLE_TO_FORMAT_DELAYED_FIELD,
        });
    }

    let ty = context
        .deref()
        .type_to_fully_annotated_layout(&ty_args[0])?;
    let include_int_type = safely_pop_arg!(arguments, bool);
    let single_line = safely_pop_arg!(arguments, bool);
    let canonicalize = safely_pop_arg!(arguments, bool);
    let type_tag = safely_pop_arg!(arguments, bool);
    let x = safely_pop_arg!(arguments, Reference);
    let v = x.read_ref().map_err(SafeNativeError::InvariantViolation)?;
    let mut out = String::new();
    let mut format_context = FormatContext {
        context,
        should_charge_gas: true,
        max_depth: usize::MAX,
        max_len: usize::MAX,
        type_tag,
        canonicalize,
        single_line,
        include_int_type,
    };
    native_format_impl(&mut format_context, &ty, v, 0, &mut out)?;
    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

fn native_format_list(
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

    let fmt_ref = safely_pop_arg!(arguments, VectorRef);
    let fmt_ref2 = fmt_ref.as_bytes_ref();
    // Could use unsafe here, but it's forbidden in this crate.
    let fmt = std::str::from_utf8(fmt_ref2.as_slice()).map_err(|_| SafeNativeError::Abort {
        abort_code: EINVALID_FORMAT,
    })?;

    context.charge(STRING_UTILS_PER_BYTE * NumBytes::new(fmt.len() as u64))?;

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
                    abort_code: EARGS_MISMATCH,
                });
            }
            Ok(())
        } else {
            Err(SafeNativeError::Abort {
                abort_code: EARGS_MISMATCH,
            })
        }
    };

    let mut out = String::new();
    let mut in_braces = 0;
    for c in fmt.chars() {
        if in_braces == 1 {
            in_braces = 0;
            if c == '}' {
                // verify`that the type is a list
                match_list_ty(context, list_ty, "Cons")?;

                // We know that the type is a list, so we can safely unwrap
                let ty_args = if let Type::StructInstantiation { ty_args, .. } = list_ty {
                    ty_args
                } else {
                    unreachable!()
                };
                let mut it = val.value_as::<Struct>()?.unpack()?;
                let car = it.next().unwrap();
                val = it.next().unwrap();
                list_ty = &ty_args[1];

                // TODO[agg_v2](cleanup): Shift this to annotated layout computation.
                let (_, has_identifier_mappings) = context
                    .deref()
                    .type_to_type_layout_with_identifier_mappings(&ty_args[0])?;
                if has_identifier_mappings {
                    return Err(SafeNativeError::Abort {
                        abort_code: EUNABLE_TO_FORMAT_DELAYED_FIELD,
                    });
                }
                let ty = context.type_to_fully_annotated_layout(&ty_args[0])?;

                let mut format_context = FormatContext {
                    context,
                    should_charge_gas: true,
                    max_depth: usize::MAX,
                    max_len: usize::MAX,
                    type_tag: true,
                    canonicalize: false,
                    single_line: true,
                    include_int_type: false,
                };
                native_format_impl(&mut format_context, &ty, car, 0, &mut out)?;
                continue;
            } else if c != '{' {
                return Err(SafeNativeError::Abort {
                    abort_code: EINVALID_FORMAT,
                });
            }
        } else if in_braces == -1 {
            in_braces = 0;
            if c != '}' {
                return Err(SafeNativeError::Abort {
                    abort_code: EINVALID_FORMAT,
                });
            }
        } else if c == '{' {
            in_braces = 1;
            continue;
        } else if c == '}' {
            in_braces = -1;
            continue;
        }
        out.push(c);
    }
    if in_braces != 0 {
        return Err(SafeNativeError::Abort {
            abort_code: EINVALID_FORMAT,
        });
    }
    match_list_ty(context, list_ty, "NIL")?;

    let move_str = Value::struct_(Struct::pack(vec![Value::vector_u8(out.into_bytes())]));
    Ok(smallvec![move_str])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        ("native_format", native_format as RawSafeNative),
        ("native_format_list", native_format_list),
    ];

    builder.make_named_natives(natives)
}
