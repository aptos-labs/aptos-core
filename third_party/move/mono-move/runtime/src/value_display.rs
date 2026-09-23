// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Renders a VM value as text, driven by its layout.
//!
//! [`FormatOptions`] chooses the syntax; see its module docs for the presets
//! and for the rendering that deliberately differs from the V1 formatter.
//
// TODO(correctness): the integer reads assume a little-endian host, as the
// comparison and BCS walks do.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    memory::{read_enum_tag, read_ptr, read_vec_len},
    types::VEC_DATA_OFFSET,
};
use mono_move_core::{
    interner::InternedIdentifier,
    types::{is_nominal, type_to_string, view_name, view_type, InternedType, Type},
    value_layout::U8_LAYOUT_ID,
    FieldValueLayout, FormatOptions, LayoutId, LayoutKind, LayoutProvider, VMInternalError,
    VMResult, ValueLayout, ENUM_DATA_OFFSET,
};
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use std::fmt::Write;

/// Renders the value at `base` of type `ty` into `out`.
///
/// # Safety
///
/// `base` must point to a fully initialized value of type `ty`.
///
/// # Precondition
///
/// `ty` must not be a reference; for reference values the caller reads the
/// reference first and passes the pointee.
pub unsafe fn display<L: LayoutProvider + ?Sized>(
    layouts: &L,
    base: *const u8,
    ty: InternedType,
    options: &FormatOptions,
    out: &mut String,
) -> VMResult<()> {
    let id = layouts.layout_id(ty).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;
    // SAFETY: caller must enforce the safety precondition.
    unsafe { display_impl(layouts, base, id, options, 0, out) }
}

/// Renders the value at `base` with the given layout, nested `depth` levels
/// deep.
///
/// # Safety
///
/// `base` must point to a fully initialized value with layout `id`.
unsafe fn display_impl<L: LayoutProvider + ?Sized>(
    layouts: &L,
    base: *const u8,
    id: LayoutId,
    options: &FormatOptions,
    depth: usize,
    out: &mut String,
) -> VMResult<()> {
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    let layout = layouts.layout(id).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;

    match &layout.kind {
        LayoutKind::Bool => {
            // SAFETY: a bool occupies one readable byte at `base`.
            out.push_str(
                if unsafe { *base } != 0 {
                    "true"
                } else {
                    "false"
                },
            );
            Ok(())
        },
        LayoutKind::UnsignedInt => {
            // SAFETY: `base` is readable for the layout's size.
            let suffix = unsafe {
                match layout.size {
                    1 => write_int(out, *base, "u8")?,
                    2 => write_int(out, u16::from_le_bytes(read_array(base)), "u16")?,
                    4 => write_int(out, u32::from_le_bytes(read_array(base)), "u32")?,
                    8 => write_int(out, u64::from_le_bytes(read_array(base)), "u64")?,
                    16 => write_int(out, u128::from_le_bytes(read_array(base)), "u128")?,
                    32 => write_int(out, U256::from_le_bytes(read_array(base)), "u256")?,
                    _ => return Err(bad_int_width("unsigned")),
                }
            };
            if options.int_suffixes {
                out.push_str(suffix);
            }
            Ok(())
        },
        LayoutKind::SignedInt => {
            // SAFETY: `base` is readable for the layout's size.
            let suffix = unsafe {
                match layout.size {
                    1 => write_int(out, *(base as *const i8), "i8")?,
                    2 => write_int(out, i16::from_le_bytes(read_array(base)), "i16")?,
                    4 => write_int(out, i32::from_le_bytes(read_array(base)), "i32")?,
                    8 => write_int(out, i64::from_le_bytes(read_array(base)), "i64")?,
                    16 => write_int(out, i128::from_le_bytes(read_array(base)), "i128")?,
                    32 => write_int(out, I256::from_le_bytes(read_array(base)), "i256")?,
                    _ => return Err(bad_int_width("signed")),
                }
            };
            if options.int_suffixes {
                out.push_str(suffix);
            }
            Ok(())
        },
        LayoutKind::Address => {
            // SAFETY: an address occupies `AccountAddress::LENGTH` readable
            // bytes at `base`.
            let addr = unsafe { read_address(base, layout)? };
            write_address(out, &addr, options);
            Ok(())
        },
        LayoutKind::Signer => {
            // SAFETY: a signer is an address in memory.
            let addr = unsafe { read_address(base, layout)? };
            out.push_str("signer(");
            write_address(out, &addr, options);
            out.push(')');
            Ok(())
        },
        LayoutKind::Vector { elem_id, .. } => {
            // SAFETY: vector values hold an 8-byte heap pointer to their data,
            // which stores the length.
            let vec = unsafe { read_ptr(base, 0usize) };
            let len = unsafe { read_vec_len(vec) } as usize;

            let elem = layouts.layout(*elem_id).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
            })?;
            if options.vec_u8_as_hex && *elem_id == U8_LAYOUT_ID {
                out.push_str("0x");
                for i in 0..len {
                    // SAFETY: the `i`th byte lies within the data region.
                    let byte = unsafe { *vec.add(VEC_DATA_OFFSET + i) };
                    write!(out, "{:02x}", byte).map_err(write_failed)?;
                }
                return Ok(());
            }

            let elem_size = elem.size as usize;
            let elems = (0..len).map(|i| Child {
                // SAFETY: the `i`th element lies within the data region.
                ptr: unsafe { vec.add(VEC_DATA_OFFSET + i * elem_size) },
                id: *elem_id,
                name: None,
            });
            out.push('[');
            // SAFETY: every element pointer is a valid value of `elem_id`.
            unsafe {
                display_body(
                    layouts,
                    elems,
                    options,
                    depth,
                    !options.single_line && is_aggregate(elem),
                    out,
                )?
            };
            out.push(']');
            Ok(())
        },
        LayoutKind::Struct { nominal, fields } => {
            if options.string_literals
                && is_nominal(*nominal, &AccountAddress::ONE, "string", "String")
            {
                // SAFETY: `String` wraps a single `vector<u8>` field.
                return unsafe { display_string(base, fields, out) };
            }
            write_nominal(out, *nominal, options);
            out.push_str(" {");
            // SAFETY: every field lies at its offset within the struct.
            unsafe { display_fields(layouts, base, fields, options, depth, out)? };
            out.push('}');
            Ok(())
        },
        LayoutKind::FrozenEnum {
            nominal, variants, ..
        } => {
            // SAFETY: enum values hold a heap pointer to an object storing the
            // tag followed by the variant body.
            let obj = unsafe { read_ptr(base, 0usize) };
            let tag = unsafe { read_enum_tag(obj) };
            let variant = variants.get(tag as usize).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::EnumTagOutOfRange {
                    tag,
                    variant_count: variants.len(),
                })
            })?;
            // SAFETY: the variant body lives at the data offset.
            let body = unsafe { obj.add(ENUM_DATA_OFFSET) };

            let body_layout = layouts.layout(variant.id).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
            })?;
            let LayoutKind::Struct { fields, .. } = &body_layout.kind else {
                return Err(VMInternalError::new(RuntimeError::InvariantViolation(
                    RuntimeInvariantViolation::Unreachable(
                        "An enum variant body must be a struct layout".to_string(),
                    ),
                )));
            };

            if is_nominal(*nominal, &AccountAddress::ONE, "option", "Option") {
                // SAFETY: `None` has no fields and `Some` has exactly one.
                return unsafe { display_option(layouts, body, fields, options, depth, out) };
            }

            write_nominal(out, *nominal, options);
            out.push_str("::");
            out.push_str(view_name(variant.name));
            out.push_str(" {");
            // SAFETY: every field lies at its offset within the variant body.
            unsafe { display_fields(layouts, body, fields, options, depth, out)? };
            out.push('}');
            Ok(())
        },
        // TODO(completeness): function values are not yet supported.
        LayoutKind::Function => Err(VMInternalError::new(RuntimeError::Unsupported(
            "function values are not yet supported",
        ))),
        LayoutKind::Ref => Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "Display runs on pointee types only".to_string(),
            ),
        ))),
    }
}

/// One rendered child of an aggregate: where it lives, how it is laid out, and
/// the field name printed ahead of it, if any.
struct Child {
    ptr: *const u8,
    id: LayoutId,
    name: Option<InternedIdentifier>,
}

/// Renders the fields of a struct or of an enum variant body, whose header the
/// caller has already written.
///
/// # Safety
///
/// `base` must point to a fully initialized value holding exactly `fields`.
unsafe fn display_fields<L: LayoutProvider + ?Sized>(
    layouts: &L,
    base: *const u8,
    fields: &[FieldValueLayout],
    options: &FormatOptions,
    depth: usize,
    out: &mut String,
) -> VMResult<()> {
    let children = fields.iter().map(|field| Child {
        // SAFETY: every field lies at its offset within the value.
        ptr: unsafe { base.add(field.offset as usize) },
        id: field.id,
        name: Some(field.name),
    });
    // SAFETY: every child pointer is a valid value of its field layout.
    unsafe { display_body(layouts, children, options, depth, !options.single_line, out) }
}

/// Renders the children of an aggregate between the brackets the caller writes.
/// Empty aggregates emit nothing at all, so they read as `[]` or `S {}`.
///
/// # Safety
///
/// Every child must point to a fully initialized value with its stated layout.
unsafe fn display_body<L, I>(
    layouts: &L,
    children: I,
    options: &FormatOptions,
    depth: usize,
    newline: bool,
    out: &mut String,
) -> VMResult<()>
where
    L: LayoutProvider + ?Sized,
    I: ExactSizeIterator<Item = Child>,
{
    if children.len() == 0 {
        return Ok(());
    }
    if depth >= options.max_depth {
        out.push_str(" .. ");
        return Ok(());
    }
    separator(out, newline, depth + 1);
    for (i, child) in children.enumerate() {
        if i > 0 {
            out.push(',');
            separator(out, newline, depth + 1);
        }
        if i >= options.max_len {
            out.push_str("..");
            break;
        }
        if let Some(name) = child.name {
            out.push_str(view_name(name));
            out.push_str(": ");
        }
        // SAFETY: caller must enforce the safety precondition.
        unsafe { display_impl(layouts, child.ptr, child.id, options, depth + 1, out)? };
    }
    separator(out, newline, depth);
    Ok(())
}

/// Renders `0x1::option::Option` as `None` or `Some(v)`. `body` points at the
/// variant body, whose `fields` are empty for `None` and hold the payload for
/// `Some`.
///
/// The payload nests one level deeper, like any other field. V1 keeps it at the
/// caller's level, so a multi-line `Some(S { .. })` indents differently there.
///
/// # Safety
///
/// `body` must point to a fully initialized variant body holding `fields`.
unsafe fn display_option<L: LayoutProvider + ?Sized>(
    layouts: &L,
    body: *const u8,
    fields: &[FieldValueLayout],
    options: &FormatOptions,
    depth: usize,
    out: &mut String,
) -> VMResult<()> {
    let Some(payload) = fields.first() else {
        out.push_str("None");
        return Ok(());
    };
    out.push_str("Some(");
    // SAFETY: the payload lies at its offset within the variant body.
    unsafe {
        display_impl(
            layouts,
            body.add(payload.offset as usize),
            payload.id,
            options,
            depth + 1,
            out,
        )?
    };
    out.push(')');
    Ok(())
}

/// Renders `0x1::string::String` as a quoted literal, escaping `\` and `"`.
///
/// # Safety
///
/// `base` must point to a fully initialized `String` holding `fields`.
unsafe fn display_string(
    base: *const u8,
    fields: &[FieldValueLayout],
    out: &mut String,
) -> VMResult<()> {
    let [bytes] = fields else {
        return Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "`String` must wrap a single byte vector".to_string(),
            ),
        )));
    };
    // SAFETY: the field holds an 8-byte heap pointer to the vector data, which
    // stores the length ahead of the bytes.
    let bytes = unsafe {
        let vec = read_ptr(base.add(bytes.offset as usize), 0usize);
        let len = read_vec_len(vec) as usize;
        std::slice::from_raw_parts(vec.add(VEC_DATA_OFFSET), len)
    };
    let text = std::str::from_utf8(bytes).map_err(|_| {
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::Unreachable(
            "`String` must hold UTF-8 bytes".to_string(),
        ))
    })?;
    out.push('"');
    for c in text.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    Ok(())
}

/// Writes the name of a struct or enum: qualified as `0x1::m::S<u64>`, or bare
/// as `S`. The bare form drops the type arguments, as V1 does.
fn write_nominal(out: &mut String, nominal: InternedType, options: &FormatOptions) {
    if !options.fully_qualified_nominals {
        if let Type::Nominal { name, .. } = view_type(nominal) {
            out.push_str(view_name(*name));
            return;
        }
    }
    out.push_str(&type_to_string(nominal));
}

/// Writes an address as `@0x1` or, canonically, as `@0x00..01`.
fn write_address(out: &mut String, addr: &AccountAddress, options: &FormatOptions) {
    out.push('@');
    if options.canonical_addresses {
        out.push_str(&addr.to_canonical_string());
    } else {
        out.push_str(&addr.to_hex_literal());
    }
}

/// Breaks the line and indents two spaces per nesting level, or writes a single
/// space when the value stays on one line.
fn separator(out: &mut String, newline: bool, depth: usize) {
    if newline {
        out.push('\n');
        for _ in 0..depth {
            out.push_str("  ");
        }
    } else {
        out.push(' ');
    }
}

/// Whether a layout renders as a bracketed aggregate. Vectors of these break
/// across lines; vectors of anything else stay compact.
fn is_aggregate(layout: &ValueLayout) -> bool {
    match layout.kind {
        LayoutKind::Vector { .. } | LayoutKind::Struct { .. } | LayoutKind::FrozenEnum { .. } => {
            true
        },
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address
        | LayoutKind::Signer
        | LayoutKind::Ref
        | LayoutKind::Function => false,
    }
}

/// Writes `v` in decimal and returns the width suffix for it.
fn write_int(
    out: &mut String,
    v: impl std::fmt::Display,
    suffix: &'static str,
) -> VMResult<&'static str> {
    write!(out, "{}", v).map_err(write_failed)?;
    Ok(suffix)
}

fn write_failed(err: std::fmt::Error) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(format!("Writing to a string failed: {err}")),
    ))
}

fn bad_int_width(signedness: &str) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(format!("Unexpected {signedness} integer width")),
    ))
}

/// Reads an address out of an address- or signer-shaped value.
///
/// # Safety
///
/// `base` must be readable for `layout.size` bytes.
unsafe fn read_address(base: *const u8, layout: &ValueLayout) -> VMResult<AccountAddress> {
    if layout.size as usize != AccountAddress::LENGTH {
        return Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable("Unexpected address width".to_string()),
        )));
    }
    // SAFETY: caller guarantees `AccountAddress::LENGTH` readable bytes.
    Ok(AccountAddress::new(unsafe { read_array(base) }))
}

/// Reads `N` bytes from the pointer into an array.
///
/// # Safety
///
/// Pointer must point to at least `N` readable, initialized bytes.
#[inline(always)]
unsafe fn read_array<const N: usize>(p: *const u8) -> [u8; N] {
    // SAFETY: `[u8; N]` has alignment 1, so this unaligned read is valid given
    // the caller's guarantee of `N` readable bytes at `p`.
    unsafe { (p as *const [u8; N]).read_unaligned() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        heap::Heap,
        value_conv::{bcs::AlignedBuf, rust::write_value},
    };
    use mono_move_core::{
        intern_type_tag, reserved_layout_id, DescriptorId, FieldValueLayout, LayoutFlags,
        VariantValueLayout,
    };
    use mono_move_global_context::{ExecutionGuard, GlobalContext};
    use move_core_types::{
        identifier::Identifier,
        language_storage::{StructTag, TypeTag},
    };
    use move_value_view::MoveValueView;
    use move_value_view_derive::MoveValueView;
    use serde::Serialize;

    fn tag(module: &str, name: &str, type_args: Vec<TypeTag>) -> TypeTag {
        TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new(module).unwrap(),
            name: Identifier::new(name).unwrap(),
            type_args,
        }))
    }

    fn reserved(ty: &Type) -> LayoutId {
        reserved_layout_id(ty).expect("reserved layout")
    }

    /// A pointer-slot layout, as vectors and enums use.
    fn ptr_layout(kind: LayoutKind) -> ValueLayout {
        ValueLayout::new(8, 8, None, LayoutFlags::empty(), kind)
    }

    fn struct_layout(
        nominal: InternedType,
        size: u32,
        align: u32,
        fields: Vec<FieldValueLayout>,
    ) -> ValueLayout {
        ValueLayout::new(
            size,
            align,
            None,
            LayoutFlags::empty(),
            LayoutKind::Struct {
                nominal,
                fields: fields.into_boxed_slice(),
            },
        )
    }

    fn field(offset: u32, id: LayoutId, name: &'static str) -> FieldValueLayout {
        FieldValueLayout {
            offset,
            id,
            name: InternedIdentifier::from_static(name),
        }
    }

    fn variants(names: &[&'static str], ids: &[LayoutId]) -> Box<[VariantValueLayout]> {
        assert_eq!(names.len(), ids.len());
        names
            .iter()
            .zip(ids.iter())
            .map(|(&name, &id)| VariantValueLayout {
                name: InternedIdentifier::from_static(name),
                id,
            })
            .collect()
    }

    fn publish_vector(
        guard: &ExecutionGuard<'_>,
        elem_tag: &TypeTag,
        elem_id: LayoutId,
    ) -> InternedType {
        let ty = intern_type_tag(&TypeTag::Vector(Box::new(elem_tag.clone())), guard).unwrap();
        guard.publish_layout(
            ty,
            ptr_layout(LayoutKind::Vector {
                elem_id,
                descriptor_id: DescriptorId(0),
            }),
        );
        ty
    }

    /// Publishes an `Option<elem>` mirroring the stdlib's `None`/`Some` enum.
    fn publish_option(
        guard: &ExecutionGuard<'_>,
        elem_tag: &TypeTag,
        elem_id: LayoutId,
        elem_size: u32,
    ) -> InternedType {
        let ty = intern_type_tag(&tag("option", "Option", vec![elem_tag.clone()]), guard).unwrap();
        let variant_ids = guard.publish_variant_layouts(ty, vec![
            struct_layout(ty, 0, 1, vec![]),
            struct_layout(ty, elem_size, elem_size.max(1), vec![field(
                0, elem_id, "e",
            )]),
        ]);
        guard.publish_layout(
            ty,
            ptr_layout(LayoutKind::FrozenEnum {
                nominal: ty,
                descriptor_id: DescriptorId(0),
                variants: variants(&["None", "Some"], &variant_ids),
                max_size_across_variants: 8 + elem_size,
            }),
        );
        ty
    }

    fn publish_string(guard: &ExecutionGuard<'_>, vec_u8_id: LayoutId) -> InternedType {
        let ty = intern_type_tag(&tag("string", "String", vec![]), guard).unwrap();
        guard.publish_layout(
            ty,
            struct_layout(ty, 8, 8, vec![field(0, vec_u8_id, "bytes")]),
        );
        ty
    }

    #[derive(Serialize, MoveValueView)]
    struct Point {
        x: u64,
        y: bool,
    }

    fn publish_point(guard: &ExecutionGuard<'_>) -> InternedType {
        let ty = intern_type_tag(&tag("m", "Point", vec![]), guard).unwrap();
        guard.publish_layout(
            ty,
            struct_layout(ty, 16, 8, vec![
                field(0, reserved(&Type::U64), "x"),
                field(8, reserved(&Type::Bool), "y"),
            ]),
        );
        ty
    }

    #[derive(Serialize, MoveValueView)]
    struct Nested {
        p: Point,
        v: Vec<u64>,
        s: String,
    }

    #[derive(Serialize, MoveValueView)]
    enum Shape {
        Circle { r: u64 },
        Dot,
        Wrap { p: Point, s: String, o: Option<u64> },
    }

    /// The fixtures every test shares: the types are published once per guard,
    /// keyed by the Rust value each test writes.
    struct Fixtures {
        u8_ty: InternedType,
        bool_ty: InternedType,
        address_ty: InternedType,
        u256_ty: InternedType,
        vec_u8: InternedType,
        vec_u64: InternedType,
        string: InternedType,
        point: InternedType,
        nested: InternedType,
        option_u64: InternedType,
        shape: InternedType,
    }

    fn publish(guard: &ExecutionGuard<'_>) -> Fixtures {
        let vec_u8 = publish_vector(guard, &TypeTag::U8, reserved(&Type::U8));
        let vec_u64 = publish_vector(guard, &TypeTag::U64, reserved(&Type::U64));
        let vec_u8_id = guard.layout_id(vec_u8).unwrap();
        let string = publish_string(guard, vec_u8_id);
        let point = publish_point(guard);
        let option_u64 = publish_option(guard, &TypeTag::U64, reserved(&Type::U64), 8);

        let nested = intern_type_tag(&tag("m", "Nested", vec![]), guard).unwrap();
        guard.publish_layout(
            nested,
            struct_layout(nested, 32, 8, vec![
                field(0, guard.layout_id(point).unwrap(), "p"),
                field(16, guard.layout_id(vec_u64).unwrap(), "v"),
                field(24, guard.layout_id(string).unwrap(), "s"),
            ]),
        );

        let shape = intern_type_tag(&tag("m", "Shape", vec![]), guard).unwrap();
        let variant_ids = guard.publish_variant_layouts(shape, vec![
            struct_layout(shape, 8, 8, vec![field(0, reserved(&Type::U64), "r")]),
            struct_layout(shape, 0, 1, vec![]),
            struct_layout(shape, 32, 8, vec![
                field(0, guard.layout_id(point).unwrap(), "p"),
                field(16, guard.layout_id(string).unwrap(), "s"),
                field(24, guard.layout_id(option_u64).unwrap(), "o"),
            ]),
        ]);
        guard.publish_layout(
            shape,
            ptr_layout(LayoutKind::FrozenEnum {
                nominal: shape,
                descriptor_id: DescriptorId(0),
                variants: variants(&["Circle", "Dot", "Wrap"], &variant_ids),
                max_size_across_variants: 40,
            }),
        );

        Fixtures {
            u8_ty: intern_type_tag(&TypeTag::U8, guard).unwrap(),
            bool_ty: intern_type_tag(&TypeTag::Bool, guard).unwrap(),
            address_ty: intern_type_tag(&TypeTag::Address, guard).unwrap(),
            u256_ty: intern_type_tag(&TypeTag::U256, guard).unwrap(),
            vec_u8,
            vec_u64,
            string,
            point,
            nested,
            option_u64,
            shape,
        }
    }

    /// Materializes `value` on a fresh heap and renders it.
    fn render<T: MoveValueView + ?Sized>(
        guard: &ExecutionGuard<'_>,
        ty: InternedType,
        value: &T,
        options: &FormatOptions,
    ) -> String {
        let mut heap = Heap::new(1 << 20);
        let mut slot = AlignedBuf::zeroed(128);
        let mut out = String::new();
        // SAFETY: the slot is aligned and wider than any test type's in-memory
        // size, and holds an initialized value of `ty` on a live heap.
        unsafe {
            write_value(guard, &mut heap, ty, value, slot.as_mut_ptr())
                .expect("write_value succeeds");
            display(guard, slot.as_ptr(), ty, options, &mut out).expect("display succeeds");
        }
        out
    }

    /// One line, no names qualified.
    const COMPACT: FormatOptions = FormatOptions {
        single_line: true,
        ..FormatOptions::MONO_MOVE
    };

    fn nested_value() -> Nested {
        Nested {
            p: Point { x: 1, y: true },
            v: vec![1, 2],
            s: "hi".to_string(),
        }
    }

    #[test]
    fn renders_primitives() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        assert_eq!(render(&guard, f.u8_ty, &7u8, &COMPACT), "7");
        assert_eq!(render(&guard, f.u256_ty, &U256::from(9u64), &COMPACT), "9");
        assert_eq!(render(&guard, f.bool_ty, &false, &COMPACT), "false");
        assert_eq!(
            render(&guard, f.address_ty, &AccountAddress::ONE, &COMPACT),
            "@0x1"
        );
    }

    #[test]
    fn int_suffixes_knob() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let options = FormatOptions {
            int_suffixes: true,
            ..COMPACT
        };
        assert_eq!(render(&guard, f.u8_ty, &7u8, &options), "7u8");
        assert_eq!(render(&guard, f.bool_ty, &true, &options), "true");
    }

    #[test]
    fn canonical_addresses_knob() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let options = FormatOptions {
            canonical_addresses: true,
            ..COMPACT
        };
        assert_eq!(
            render(&guard, f.address_ty, &AccountAddress::ONE, &options),
            "@0000000000000000000000000000000000000000000000000000000000000001"
        );
    }

    #[test]
    fn vec_u8_as_hex_knob() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let bytes = vec![1u8, 2, 255];
        assert_eq!(render(&guard, f.vec_u8, &bytes, &COMPACT), "0x0102ff");
        let options = FormatOptions {
            vec_u8_as_hex: false,
            ..COMPACT
        };
        assert_eq!(render(&guard, f.vec_u8, &bytes, &options), "[ 1, 2, 255 ]");
        // Only `vector<u8>` takes the hex path.
        assert_eq!(
            render(&guard, f.vec_u64, &vec![1u64, 2], &COMPACT),
            "[ 1, 2 ]"
        );
    }

    #[test]
    fn string_literals_knob() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let s = r#"say "hi\""#.to_string();
        assert_eq!(render(&guard, f.string, &s, &COMPACT), r#""say \"hi\\\"""#);
        let options = FormatOptions {
            string_literals: false,
            ..COMPACT
        };
        assert_eq!(
            render(&guard, f.string, &"hi".to_string(), &options),
            "String { bytes: 0x6869 }"
        );
    }

    #[test]
    fn fully_qualified_nominals_knob() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let value = Point { x: 1, y: true };
        assert_eq!(
            render(&guard, f.point, &value, &COMPACT),
            "Point { x: 1, y: true }"
        );
        let options = FormatOptions {
            fully_qualified_nominals: true,
            ..COMPACT
        };
        assert_eq!(
            render(&guard, f.point, &value, &options),
            "0x1::m::Point { x: 1, y: true }"
        );
    }

    #[test]
    fn single_line_knob() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let value = nested_value();
        assert_eq!(
            render(&guard, f.nested, &value, &COMPACT),
            "Nested { p: Point { x: 1, y: true }, v: [ 1, 2 ], s: \"hi\" }"
        );
        // Struct fields always break; a vector of primitives never does.
        assert_eq!(
            render(&guard, f.nested, &value, &FormatOptions::MONO_MOVE),
            "Nested {\n  p: Point {\n    x: 1,\n    y: true\n  },\n  v: [ 1, 2 ],\n  s: \"hi\"\n}"
        );
    }

    #[test]
    fn vectors_of_aggregates_break_across_lines() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let vec_point = publish_vector(
            &guard,
            &tag("m", "Point", vec![]),
            guard.layout_id(f.point).unwrap(),
        );
        let value = vec![Point { x: 1, y: true }];
        assert_eq!(
            render(&guard, vec_point, &value, &FormatOptions::MONO_MOVE),
            "[\n  Point {\n    x: 1,\n    y: true\n  }\n]"
        );
    }

    #[test]
    fn empty_aggregates_have_no_separators() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        assert_eq!(
            render(
                &guard,
                f.vec_u64,
                &Vec::<u64>::new(),
                &FormatOptions::MONO_MOVE
            ),
            "[]"
        );
        assert_eq!(
            render(&guard, f.shape, &Shape::Dot, &FormatOptions::MONO_MOVE),
            "Shape::Dot {}"
        );
    }

    #[test]
    fn max_depth_elides_nested_aggregates() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let value = nested_value();
        let options = FormatOptions {
            max_depth: 1,
            ..COMPACT
        };
        assert_eq!(
            render(&guard, f.nested, &value, &options),
            "Nested { p: Point { .. }, v: [ .. ], s: \"hi\" }"
        );
        let options = FormatOptions {
            max_depth: 0,
            ..COMPACT
        };
        assert_eq!(render(&guard, f.nested, &value, &options), "Nested { .. }");
    }

    #[test]
    fn max_len_elides_trailing_elements() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        let options = FormatOptions {
            max_len: 2,
            ..COMPACT
        };
        assert_eq!(
            render(&guard, f.vec_u64, &vec![1u64, 2, 3, 4], &options),
            "[ 1, 2, .. ]"
        );
    }

    #[test]
    fn options_keep_their_sugar() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        for options in [COMPACT, FormatOptions::TO_STRING] {
            assert_eq!(render(&guard, f.option_u64, &None::<u64>, &options), "None");
            assert_eq!(
                render(&guard, f.option_u64, &Some(5u64), &options),
                "Some(5)"
            );
        }
    }

    /// Enums are named and their subtree stays decorated, unlike V1, which
    /// prints `#0{ 7 }` and `#2{ { 1, true }, { 0x6869 }, #1{ 5 } }`.
    #[test]
    fn enums_name_their_variant_and_decorate_the_subtree() {
        let ctx = GlobalContext::with_num_execution_workers(1);
        let guard = ctx.try_execution_context(0).unwrap();
        let f = publish(&guard);
        assert_eq!(
            render(&guard, f.shape, &Shape::Circle { r: 7 }, &COMPACT),
            "Shape::Circle { r: 7 }"
        );
        let value = Shape::Wrap {
            p: Point { x: 1, y: true },
            s: "hi".to_string(),
            o: Some(5),
        };
        assert_eq!(
            render(&guard, f.shape, &value, &COMPACT),
            "Shape::Wrap { p: Point { x: 1, y: true }, s: \"hi\", o: Some(5) }"
        );
        assert_eq!(
            render(&guard, f.shape, &value, &FormatOptions::LIST_ELEMENT),
            "0x1::m::Shape::Wrap { p: 0x1::m::Point { x: 1, y: true }, s: \"hi\", o: Some(5) }"
        );
    }
}
