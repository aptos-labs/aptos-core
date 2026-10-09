// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Renders a VM value as text, driven by its layout.
//!
//! [`FormatOptions`] chooses the syntax; see its module docs for the presets
//! and for the rendering that deliberately differs from the V1 formatter.
//!
//! The renderer is a visitor over the iterative walk in `value_walk.rs`: the
//! walk hands it scalars and the entry into and exit from each aggregate, and
//! it keeps one [`Frame`] per open aggregate for the brackets, separators and
//! indentation.
//
// TODO(correctness): the integer reads assume a little-endian host, as the
// comparison and BCS walks do.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    memory::{read_ptr, read_vec_len},
    types::VEC_DATA_OFFSET,
    value_walk::{walk, Event, Step, ValueVisitor},
};
use mono_move_core::{
    interner::InternedIdentifier,
    types::{is_nominal, type_to_string, view_name, view_type, InternedType, Type},
    value_layout::U8_LAYOUT_ID,
    FieldValueLayout, FormatOptions, LayoutKind, LayoutProvider, VMInternalError, VMResult,
    ValueLayout,
};
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use std::{convert::Infallible, fmt::Write};

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
    let layout = layouts.layout_by_ty(ty).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;
    let mut renderer = Renderer {
        options,
        out,
        frames: Vec::new(),
        pending: None,
    };
    // SAFETY: caller must enforce the safety precondition.
    unsafe { walk(layouts, [base], layout, &mut renderer)? };
    Ok(())
}

/// Writes the text for each node the walk reports.
struct Renderer<'o> {
    options: &'o FormatOptions,
    out: &'o mut String,
    /// One entry per aggregate the walk has entered and not yet exited.
    frames: Vec<Frame>,
    /// Set on entering an enum; consumed by the variant body that follows.
    pending: Option<Pending>,
}

/// An open aggregate: how its children and its closing bracket render.
struct Frame {
    /// Nesting level of the children. The aggregate itself sits one level up.
    child_depth: usize,
    /// Children break across lines rather than being space-separated.
    newline: bool,
    /// Children are preceded by `name: `.
    named: bool,
    /// Children render with no separator and no name at all: the payload of
    /// `Some(..)`.
    transparent: bool,
    /// A separator precedes the closing text, because a child was started.
    separate: bool,
    /// Written on exit.
    close: &'static str,
}

/// What the next struct body stands for.
enum Pending {
    /// The fields of a named enum variant; the header is already written.
    Variant,
    /// The single payload field of `Some(..)`.
    Some,
}

impl ValueVisitor<1> for Renderer<'_> {
    type Break = Infallible;

    fn visit(&mut self, event: Event<'_, 1>) -> VMResult<Step<Infallible>> {
        match event {
            Event::Scalar {
                layout,
                ptrs: [base],
            } => {
                self.no_pending()?;
                // SAFETY: the walk hands out pointers to live scalars of
                // `layout.size` bytes.
                unsafe { self.scalar(layout, base)? };
                Ok(Step::Skip)
            },
            Event::EnterStruct {
                layout,
                fields,
                ptrs: [base],
            } => match self.pending.take() {
                None => {
                    let nominal = nominal_of(layout)?;
                    if self.options.string_literals
                        && is_nominal(nominal, &AccountAddress::ONE, "string", "String")
                    {
                        // SAFETY: `String` wraps a single `vector<u8>` field.
                        unsafe { display_string(base, fields, self.out)? };
                        self.frames.push(self.leaf_frame());
                        return Ok(Step::Skip);
                    }
                    write_nominal(self.out, nominal, self.options);
                    self.out.push_str(" {");
                    let child_depth = self.depth() + 1;
                    Ok(self.open(fields.len(), child_depth, true, "}"))
                },
                // The enum frame already holds the body's depth.
                Some(Pending::Variant) => Ok(self.open(fields.len(), self.depth(), true, "}")),
                Some(Pending::Some) => {
                    self.frames.push(Frame {
                        child_depth: self.depth(),
                        newline: false,
                        named: false,
                        transparent: true,
                        separate: false,
                        close: "",
                    });
                    Ok(Step::Descend)
                },
            },
            Event::Field { index, field, .. } => Ok(self.child(index as u64, Some(field.name))),
            Event::EnterVector {
                elem_id,
                elem,
                lens: [len],
                data: [data],
                ..
            } => {
                self.no_pending()?;
                if self.options.vec_u8_as_hex && elem_id == U8_LAYOUT_ID {
                    self.out.push_str("0x");
                    for i in 0..len as usize {
                        // SAFETY: `i < len`, so the byte lies within the data
                        // region and `data` is non-null.
                        let byte = unsafe { *data.add(i) };
                        write!(self.out, "{:02x}", byte).map_err(write_failed)?;
                    }
                    self.frames.push(self.leaf_frame());
                    return Ok(Step::Skip);
                }
                self.out.push('[');
                let child_depth = self.depth() + 1;
                let newline = !self.options.single_line && is_aggregate(elem);
                Ok(self.open_with(len as usize, child_depth, newline, false, "]"))
            },
            Event::Element { index, .. } => Ok(self.child(index, None)),
            Event::EnterEnum {
                layout,
                variants,
                tags: [tag],
                body,
                ..
            } => {
                self.no_pending()?;
                let nominal = nominal_of(layout)?;
                let Some(body) = body else {
                    return Err(unreachable(
                        "A single-value walk always selects a variant body",
                    ));
                };
                let LayoutKind::Struct { fields } = &body.kind else {
                    return Err(unreachable("An enum variant body must be a struct layout"));
                };
                let child_depth = self.depth() + 1;
                if is_nominal(nominal, &AccountAddress::ONE, "option", "Option") {
                    // `None` has no fields and `Some` has exactly one.
                    if fields.is_empty() {
                        self.out.push_str("None");
                        self.frames.push(self.leaf_frame());
                        return Ok(Step::Skip);
                    }
                    self.out.push_str("Some(");
                    // The payload nests one level deeper, like any other
                    // field. V1 keeps it at the caller's level, so a
                    // multi-line `Some(S { .. })` indents differently there.
                    self.frames.push(Frame {
                        child_depth,
                        newline: false,
                        named: false,
                        transparent: false,
                        separate: false,
                        close: ")",
                    });
                    self.pending = Some(Pending::Some);
                    return Ok(Step::Descend);
                }
                write_nominal(self.out, nominal, self.options);
                self.out.push_str("::");
                self.out.push_str(view_name(variants[tag as usize].name));
                self.out.push_str(" {");
                self.frames.push(Frame {
                    child_depth,
                    newline: false,
                    named: false,
                    transparent: false,
                    separate: false,
                    close: "",
                });
                self.pending = Some(Pending::Variant);
                Ok(Step::Descend)
            },
            Event::ExitStruct { .. } | Event::ExitVector { .. } | Event::ExitEnum { .. } => {
                self.close()?;
                Ok(Step::Descend)
            },
        }
    }
}

impl Renderer<'_> {
    /// Nesting level at which a value opened now renders its children.
    fn depth(&self) -> usize {
        self.frames.last().map_or(0, |frame| frame.child_depth)
    }

    /// A frame for an aggregate rendered in one go, with nothing to close.
    fn leaf_frame(&self) -> Frame {
        Frame {
            child_depth: self.depth(),
            newline: false,
            named: false,
            transparent: false,
            separate: false,
            close: "",
        }
    }

    fn open(
        &mut self,
        count: usize,
        child_depth: usize,
        named: bool,
        close: &'static str,
    ) -> Step<Infallible> {
        let newline = !self.options.single_line;
        self.open_with(count, child_depth, newline, named, close)
    }

    /// Opens an aggregate with `count` children whose opening bracket is
    /// already written. Empty aggregates render as `[]` or `S {}`; aggregates
    /// past `max_depth` as `[ .. ]`.
    fn open_with(
        &mut self,
        count: usize,
        child_depth: usize,
        newline: bool,
        named: bool,
        close: &'static str,
    ) -> Step<Infallible> {
        let mut frame = Frame {
            child_depth,
            newline,
            named,
            transparent: false,
            separate: false,
            close,
        };
        if count == 0 {
            self.frames.push(frame);
            return Step::Skip;
        }
        // The aggregate itself sits one level above its children.
        if child_depth > self.options.max_depth {
            self.out.push_str(" .. ");
            self.frames.push(frame);
            return Step::Skip;
        }
        frame.separate = true;
        self.frames.push(frame);
        Step::Descend
    }

    /// Writes what precedes child `index` of the innermost aggregate, or
    /// elides it and the rest past `max_len`.
    fn child(&mut self, index: u64, name: Option<InternedIdentifier>) -> Step<Infallible> {
        let Some(frame) = self.frames.last() else {
            return Step::Descend;
        };
        if frame.transparent {
            return Step::Descend;
        }
        let (newline, child_depth, named) = (frame.newline, frame.child_depth, frame.named);
        if index > 0 {
            self.out.push(',');
        }
        separator(self.out, newline, child_depth);
        if index >= self.options.max_len as u64 {
            self.out.push_str("..");
            return Step::Skip;
        }
        if let (true, Some(name)) = (named, name) {
            self.out.push_str(view_name(name));
            self.out.push_str(": ");
        }
        Step::Descend
    }

    /// Closes the innermost aggregate.
    fn close(&mut self) -> VMResult<()> {
        let Some(frame) = self.frames.pop() else {
            return Err(unreachable("Every exit matches an open frame"));
        };
        if frame.separate {
            separator(self.out, frame.newline, frame.child_depth - 1);
        }
        self.out.push_str(frame.close);
        Ok(())
    }

    fn no_pending(&self) -> VMResult<()> {
        if self.pending.is_some() {
            return Err(unreachable("An enum variant body must be a struct layout"));
        }
        Ok(())
    }

    /// Writes a bool, integer, address or signer.
    ///
    /// # Safety
    ///
    /// `base` must be readable for `layout.size` bytes.
    unsafe fn scalar(&mut self, layout: &ValueLayout, base: *const u8) -> VMResult<()> {
        let out = &mut *self.out;
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
                if self.options.int_suffixes {
                    out.push_str(suffix);
                }
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
                if self.options.int_suffixes {
                    out.push_str(suffix);
                }
            },
            LayoutKind::Address => {
                // SAFETY: an address occupies `AccountAddress::LENGTH` readable
                // bytes at `base`.
                let addr = unsafe { read_address(base, layout)? };
                write_address(out, &addr, self.options);
            },
            LayoutKind::Signer => {
                // SAFETY: a signer is an address in memory.
                let addr = unsafe { read_address(base, layout)? };
                out.push_str("signer(");
                write_address(out, &addr, self.options);
                out.push(')');
            },
            LayoutKind::Struct { .. }
            | LayoutKind::Vector { .. }
            | LayoutKind::FrozenEnum { .. }
            | LayoutKind::Function
            | LayoutKind::Ref => {
                return Err(unreachable(
                    "The walk reports only scalars as scalar events",
                ))
            },
        }
        Ok(())
    }
}

/// Renders `0x1::string::String` as a quoted literal with `\` and `"` escaped.
///
/// # Safety
///
/// `base` must point to a fully initialized `String` whose single field is
/// described by `fields`.
unsafe fn display_string(
    base: *const u8,
    fields: &[FieldValueLayout],
    out: &mut String,
) -> VMResult<()> {
    let [bytes] = fields else {
        return Err(unreachable("`String` must wrap a single byte vector"));
    };
    // SAFETY: the field holds an 8-byte heap pointer to the vector data, which
    // stores the length ahead of the bytes. An empty vector is a null pointer,
    // so the data region is only addressed for a non-zero length.
    let bytes = unsafe {
        let vec = read_ptr(base.add(bytes.offset as usize), 0usize);
        let len = read_vec_len(vec) as usize;
        if len == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(vec.add(VEC_DATA_OFFSET), len)
        }
    };
    let text =
        std::str::from_utf8(bytes).map_err(|_| unreachable("`String` must hold UTF-8 bytes"))?;
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

/// The type a struct or enum layout describes. Variant bodies are the only
/// struct layouts without one, and they are rendered through their enum.
fn nominal_of(layout: &ValueLayout) -> VMResult<InternedType> {
    layout
        .ty
        .ok_or_else(|| unreachable("A struct or enum layout must name its type"))
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
    unreachable(&format!("Writing to a string failed: {err}"))
}

fn bad_int_width(signedness: &str) -> VMInternalError {
    unreachable(&format!("Unexpected {signedness} integer width"))
}

fn unreachable(msg: &str) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(msg.to_string()),
    ))
}

/// Reads an address out of an address- or signer-shaped value.
///
/// # Safety
///
/// `base` must be readable for `layout.size` bytes.
unsafe fn read_address(base: *const u8, layout: &ValueLayout) -> VMResult<AccountAddress> {
    if layout.size as usize != AccountAddress::LENGTH {
        return Err(unreachable("Unexpected address width"));
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
        intern_type_tag, reserved_layout_id, DescriptorId, FieldValueLayout, LayoutFlags, LayoutId,
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
    fn ptr_layout(ty: InternedType, kind: LayoutKind) -> ValueLayout {
        ValueLayout::new(Some(ty), 8, 8, None, LayoutFlags::empty(), kind)
    }

    fn struct_layout(
        ty: Option<InternedType>,
        size: u32,
        align: u32,
        fields: Vec<FieldValueLayout>,
    ) -> ValueLayout {
        ValueLayout::new(
            ty,
            size,
            align,
            None,
            LayoutFlags::empty(),
            LayoutKind::Struct {
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
        guard.publish_layout(ptr_layout(ty, LayoutKind::Vector {
            elem_id,
            descriptor_id: DescriptorId(0),
        }));
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
            struct_layout(None, 0, 1, vec![]),
            struct_layout(None, elem_size, elem_size.max(1), vec![field(
                0, elem_id, "e",
            )]),
        ]);
        guard.publish_layout(ptr_layout(ty, LayoutKind::FrozenEnum {
            descriptor_id: DescriptorId(0),
            variants: variants(&["None", "Some"], &variant_ids),
            max_size_across_variants: 8 + elem_size,
        }));
        ty
    }

    fn publish_string(guard: &ExecutionGuard<'_>, vec_u8_id: LayoutId) -> InternedType {
        let ty = intern_type_tag(&tag("string", "String", vec![]), guard).unwrap();
        guard.publish_layout(struct_layout(Some(ty), 8, 8, vec![field(
            0, vec_u8_id, "bytes",
        )]));
        ty
    }

    #[derive(Serialize, MoveValueView)]
    struct Point {
        x: u64,
        y: bool,
    }

    fn publish_point(guard: &ExecutionGuard<'_>) -> InternedType {
        let ty = intern_type_tag(&tag("m", "Point", vec![]), guard).unwrap();
        guard.publish_layout(struct_layout(Some(ty), 16, 8, vec![
            field(0, reserved(&Type::U64), "x"),
            field(8, reserved(&Type::Bool), "y"),
        ]));
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
        guard.publish_layout(struct_layout(Some(nested), 32, 8, vec![
            field(0, guard.layout_id(point).unwrap(), "p"),
            field(16, guard.layout_id(vec_u64).unwrap(), "v"),
            field(24, guard.layout_id(string).unwrap(), "s"),
        ]));

        let shape = intern_type_tag(&tag("m", "Shape", vec![]), guard).unwrap();
        let variant_ids = guard.publish_variant_layouts(shape, vec![
            struct_layout(None, 8, 8, vec![field(0, reserved(&Type::U64), "r")]),
            struct_layout(None, 0, 1, vec![]),
            struct_layout(None, 32, 8, vec![
                field(0, guard.layout_id(point).unwrap(), "p"),
                field(16, guard.layout_id(string).unwrap(), "s"),
                field(24, guard.layout_id(option_u64).unwrap(), "o"),
            ]),
        ]);
        guard.publish_layout(ptr_layout(shape, LayoutKind::FrozenEnum {
            descriptor_id: DescriptorId(0),
            variants: variants(&["Circle", "Dot", "Wrap"], &variant_ids),
            max_size_across_variants: 40,
        }));

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
