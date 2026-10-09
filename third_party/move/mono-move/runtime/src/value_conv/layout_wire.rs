// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The V1 `MoveTypeLayout` wire format used inside serialized function values.
//!
//! A serialized closure carries, per captured value, the V1 `MoveTypeLayout`
//! of that value followed by the value itself. MonoMove has its own
//! `ValueLayout` and never builds a `MoveTypeLayout`, so this module bridges
//! the two directions directly on bytes:
//!
//! - [`emit_move_type_layout`] writes the wire bytes from a [`LayoutId`].
//! - [`walk_capture`] advances a layout cursor and a value cursor together
//!   over one captured value.
//! - [`compare_captures`] runs that walk over two closures at once, each under
//!   its own layouts.
//!
//! The tags are decoded in exactly one place, [`read_layout_node`], because a
//! desync between the two cursors is silent: the value cursor lands mid-value
//! and every later capture decodes garbage.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    value_conv::bcs::{read_slice, read_uleb128_len, write_uleb128_len},
};
use mono_move_core::{
    interner::{type_tag_of, view_module_id, FunctionRef},
    types::{view_name, view_type_list},
    LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult, ValueLayout,
};
use move_core_types::{
    account_address::AccountAddress, function::FUNCTION_DATA_SERIALIZATION_FORMAT_V1,
    identifier::IdentStr, language_storage::TypeTag,
};
use std::cmp::Ordering;

/// BCS tags of `MoveTypeLayout`, in declaration order. Serde numbers enum
/// variants by position, so these mirror the declaration in
/// `move-core/types/src/value.rs` and must never be reordered.
mod layout_tag {
    pub const BOOL: u64 = 0;
    pub const U8: u64 = 1;
    pub const U64: u64 = 2;
    pub const U128: u64 = 3;
    pub const ADDRESS: u64 = 4;
    pub const VECTOR: u64 = 5;
    pub const STRUCT: u64 = 6;
    pub const SIGNER: u64 = 7;
    pub const U16: u64 = 8;
    pub const U32: u64 = 9;
    pub const U256: u64 = 10;
    pub const NATIVE: u64 = 11;
    pub const FUNCTION: u64 = 12;
    pub const I8: u64 = 13;
    pub const I16: u64 = 14;
    pub const I32: u64 = 15;
    pub const I64: u64 = 16;
    pub const I128: u64 = 17;
    pub const I256: u64 = 18;
}

/// BCS tags of `MoveStructLayout`, in declaration order.
mod struct_tag {
    pub const RUNTIME: u64 = 0;
    pub const RUNTIME_VARIANTS: u64 = 1;
    pub const WITH_FIELDS: u64 = 2;
    pub const WITH_TYPES: u64 = 3;
    pub const WITH_VARIANTS: u64 = 4;
}

/// BCS tags of `TypeTag`, in declaration order.
mod type_tag {
    pub const BOOL: u64 = 0;
    pub const U8: u64 = 1;
    pub const U64: u64 = 2;
    pub const U128: u64 = 3;
    pub const ADDRESS: u64 = 4;
    pub const SIGNER: u64 = 5;
    pub const VECTOR: u64 = 6;
    pub const STRUCT: u64 = 7;
    pub const U16: u64 = 8;
    pub const U32: u64 = 9;
    pub const U256: u64 = 10;
    pub const FUNCTION: u64 = 11;
    pub const I8: u64 = 12;
    pub const I16: u64 = 13;
    pub const I32: u64 = 14;
    pub const I64: u64 = 15;
    pub const I128: u64 = 16;
    pub const I256: u64 = 17;
}

/// A layout carries a shape V1 can decode but never itself writes into a
/// capture. Falling back to V1 keeps MonoMove from diverging on an input V1
/// accepts.
fn unsupported_layout() -> RuntimeError {
    RuntimeError::Unsupported("captured layout is not supported by MonoMove")
}

fn layout_not_found() -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::ValueLayoutNotFound,
    ))
}

fn unreachable_layout(what: String) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(what),
    ))
}

// ---------------------------------------------------------------------------
// Emitting
// ---------------------------------------------------------------------------

/// Writes the V1 `MoveTypeLayout` BCS encoding of `id` to `out`. No
/// `MoveTypeLayout` is built: the bytes come straight off the layout table.
//
// TODO(metering): recursion depth here follows the layout, so a deeply nested
// layout consumes proportional native stack. Make it iterative.
//
// TODO(security, metering): V1 applies `check_layout_within_bounds` per
// capture, bounding the unfolded node count at 4096. This walks
// the layout table, which is a DAG keyed by `LayoutId`: a shared node is
// visited once but emitted in full on every path, so the emitted size is
// exponential in nesting depth while the walk looks linear. The e2e test
// `captured_layout_dag_over_cap_rejected` builds exactly that shape and expects
// V1 to reject it. Port the unfolded node count here.
pub(crate) fn emit_move_type_layout<T: LayoutProvider + ?Sized>(
    layouts: &T,
    id: LayoutId,
    out: &mut Vec<u8>,
) -> VMResult<()> {
    let layout = layouts.layout(id).ok_or_else(layout_not_found)?;

    match &layout.kind {
        LayoutKind::Bool => write_uleb128_len(out, layout_tag::BOOL),
        LayoutKind::UnsignedInt => write_uleb128_len(out, unsigned_int_tag(layout)?),
        LayoutKind::SignedInt => write_uleb128_len(out, signed_int_tag(layout)?),
        LayoutKind::Address => write_uleb128_len(out, layout_tag::ADDRESS),
        LayoutKind::Function => write_uleb128_len(out, layout_tag::FUNCTION),
        LayoutKind::Vector { elem_id, .. } => {
            write_uleb128_len(out, layout_tag::VECTOR);
            emit_move_type_layout(layouts, *elem_id, out)?;
        },
        LayoutKind::Struct { fields } => {
            write_uleb128_len(out, layout_tag::STRUCT);
            write_uleb128_len(out, struct_tag::RUNTIME);
            write_uleb128_len(out, fields.len() as u64);
            for field in fields.iter() {
                emit_move_type_layout(layouts, field.id, out)?;
            }
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            write_uleb128_len(out, layout_tag::STRUCT);
            write_uleb128_len(out, struct_tag::RUNTIME_VARIANTS);
            write_uleb128_len(out, variants.len() as u64);
            for variant in variants.iter() {
                let body = layouts.layout(variant.id).ok_or_else(layout_not_found)?;
                let LayoutKind::Struct { fields } = &body.kind else {
                    return Err(unreachable_layout(
                        "an enum variant body is a struct layout".to_string(),
                    ));
                };
                write_uleb128_len(out, fields.len() as u64);
                for field in fields.iter() {
                    emit_move_type_layout(layouts, field.id, out)?;
                }
            }
        },
        LayoutKind::Signer => {
            return Err(unreachable_layout(
                "a capture is never a signer".to_string(),
            ))
        },
        LayoutKind::Ref => {
            return Err(unreachable_layout(
                "a capture is never a reference".to_string(),
            ))
        },
    }
    Ok(())
}

fn unsigned_int_tag(layout: &ValueLayout) -> VMResult<u64> {
    Ok(match layout.size {
        1 => layout_tag::U8,
        2 => layout_tag::U16,
        4 => layout_tag::U32,
        8 => layout_tag::U64,
        16 => layout_tag::U128,
        32 => layout_tag::U256,
        size => return Err(bad_int_width(size)),
    })
}

fn signed_int_tag(layout: &ValueLayout) -> VMResult<u64> {
    Ok(match layout.size {
        1 => layout_tag::I8,
        2 => layout_tag::I16,
        4 => layout_tag::I32,
        8 => layout_tag::I64,
        16 => layout_tag::I128,
        32 => layout_tag::I256,
        size => return Err(bad_int_width(size)),
    })
}

fn bad_int_width(size: u32) -> VMInternalError {
    unreachable_layout(format!("{size} is not a Move integer width"))
}

/// Writes a closure's `5 + 2n` sequence prefix and its five header elements,
/// `(format_version, module_id, fun_id, ty_args, mask)`. The `n` capture pairs
/// follow and are written by the caller.
pub(crate) fn emit_closure_header(
    func_ref: &FunctionRef,
    mask: u64,
    out: &mut Vec<u8>,
) -> VMResult<()> {
    write_uleb128_len(out, 5 + 2 * mask.count_ones() as u64);
    out.extend_from_slice(&FUNCTION_DATA_SERIALIZATION_FORMAT_V1.to_le_bytes());

    let module_id = view_module_id(func_ref.module_id);
    out.extend_from_slice(&module_id.address().into_bytes());
    emit_identifier(out, view_name(module_id.name()));
    emit_identifier(out, view_name(func_ref.func_name));

    let ty_args = view_type_list(func_ref.ty_args);
    write_uleb128_len(out, ty_args.len() as u64);
    for &ty in ty_args {
        // Type parameters and references have no `TypeTag`. Neither can be a
        // closure's type argument once it is fully substituted, but bailing
        // beats emitting a header V1 could not have written.
        let tag = type_tag_of(ty).ok_or_else(|| {
            VMInternalError::new(RuntimeError::Unsupported(
                "function value over a type with no type tag",
            ))
        })?;
        bcs::serialize_into(out, &tag).map_err(|err| {
            unreachable_layout(format!(
                "a substituted type argument has no BCS form: {err}"
            ))
        })?;
    }

    // A `ClosureMask` is a newtype over `u64`.
    out.extend_from_slice(&mask.to_le_bytes());
    Ok(())
}

/// Writes an `Identifier`, a length-prefixed string.
fn emit_identifier(out: &mut Vec<u8>, name: &str) {
    write_uleb128_len(out, name.len() as u64);
    out.extend_from_slice(name.as_bytes());
}

// ---------------------------------------------------------------------------
// Walking
// ---------------------------------------------------------------------------

/// One decoded `MoveTypeLayout` node. The layout cursor is left just past the
/// node's own tag and inline data; child layouts, where the node has them,
/// follow immediately.
///
/// `Signer`, `Native` and the decorated `MoveStructLayout` forms never become a
/// node: [`read_layout_node`] rejects them.
enum LayoutNode {
    /// One canonical `0`/`1` byte.
    Bool,
    /// An integer of this many bytes, two's complement when signed.
    Int { width: usize, signed: bool },
    /// Thirty-two bytes.
    Address,
    /// A ULEB length then that many elements. One child layout follows and is
    /// shared by every element.
    Vector,
    /// This many child layouts follow, each a field in declaration order.
    Struct { fields: u64 },
    /// This many variants follow, each a ULEB field count then that many child
    /// layouts. The value is a ULEB variant tag then the fields of that one
    /// variant, so a walk descends into the selected variant and skips the
    /// rest.
    Enum { variants: u64 },
    /// A nested closure. Nothing follows on the layout side.
    Function,
}

/// Decodes one `MoveTypeLayout` node, advancing `cursor` past its tag and
/// inline data.
fn read_layout_node(bytes: &[u8], cursor: &mut usize) -> Result<LayoutNode, RuntimeError> {
    let tag = read_uleb128_len(bytes, cursor)?;
    Ok(match tag {
        layout_tag::BOOL => LayoutNode::Bool,
        layout_tag::U8 => int_node(1, false),
        layout_tag::U16 => int_node(2, false),
        layout_tag::U32 => int_node(4, false),
        layout_tag::U64 => int_node(8, false),
        layout_tag::U128 => int_node(16, false),
        layout_tag::U256 => int_node(32, false),
        layout_tag::I8 => int_node(1, true),
        layout_tag::I16 => int_node(2, true),
        layout_tag::I32 => int_node(4, true),
        layout_tag::I64 => int_node(8, true),
        layout_tag::I128 => int_node(16, true),
        layout_tag::I256 => int_node(32, true),
        layout_tag::ADDRESS => LayoutNode::Address,
        layout_tag::VECTOR => LayoutNode::Vector,
        layout_tag::STRUCT => read_struct_node(bytes, cursor)?,
        layout_tag::FUNCTION => LayoutNode::Function,
        // A `signer` decodes as `RuntimeVariants([[Address]])`, so it is not
        // zero-width on the value side. Both are rejected rather than walked:
        // V1's `construct_captured_layouts` produces neither a `signer` nor a
        // `Native` capture, so these bytes are adversarial.
        layout_tag::SIGNER | layout_tag::NATIVE => return Err(unsupported_layout()),
        tag => {
            return Err(RuntimeError::BCSInvalidWireTag {
                what: "MoveTypeLayout",
                tag,
            })
        },
    })
}

fn int_node(width: usize, signed: bool) -> LayoutNode {
    LayoutNode::Int { width, signed }
}

/// Decodes a `MoveStructLayout`, whose own tag has already been consumed.
fn read_struct_node(bytes: &[u8], cursor: &mut usize) -> Result<LayoutNode, RuntimeError> {
    let tag = read_uleb128_len(bytes, cursor)?;
    Ok(match tag {
        struct_tag::RUNTIME => LayoutNode::Struct {
            fields: read_uleb128_len(bytes, cursor)?,
        },
        struct_tag::RUNTIME_VARIANTS => LayoutNode::Enum {
            variants: read_uleb128_len(bytes, cursor)?,
        },
        // The decorated forms interleave field names and a `StructTag` with the
        // child layouts. V1 writes only the runtime forms into a capture, so
        // these are adversarial.
        struct_tag::WITH_FIELDS | struct_tag::WITH_TYPES | struct_tag::WITH_VARIANTS => {
            return Err(unsupported_layout())
        },
        tag => {
            return Err(RuntimeError::BCSInvalidWireTag {
                what: "MoveStructLayout",
                tag,
            })
        },
    })
}

/// Advances `layout` past one `MoveTypeLayout` without reading any value.
pub(crate) fn skip_layout(bytes: &[u8], layout: &mut usize) -> Result<(), RuntimeError> {
    match read_layout_node(bytes, layout)? {
        LayoutNode::Bool | LayoutNode::Int { .. } | LayoutNode::Address | LayoutNode::Function => {
            Ok(())
        },
        LayoutNode::Vector => skip_layout(bytes, layout),
        LayoutNode::Struct { fields } => {
            for _ in 0..fields {
                skip_layout(bytes, layout)?;
            }
            Ok(())
        },
        LayoutNode::Enum { variants } => skip_variants(bytes, layout, variants),
    }
}

/// Advances `layout` past `count` variants, each a field count then that many
/// field layouts.
fn skip_variants(bytes: &[u8], layout: &mut usize, count: u64) -> Result<(), RuntimeError> {
    for _ in 0..count {
        let fields = read_uleb128_len(bytes, layout)?;
        for _ in 0..fields {
            skip_layout(bytes, layout)?;
        }
    }
    Ok(())
}

/// Advances `layout` over one `MoveTypeLayout` and `value` over one value of
/// that layout. Both cursors index into `bytes`, which holds the whole closure.
//
// TODO(metering): recursion depth here follows the value, so a deeply nested
// value consumes proportional native stack. Make it iterative.
pub(crate) fn walk_capture(
    bytes: &[u8],
    layout: &mut usize,
    value: &mut usize,
) -> Result<(), RuntimeError> {
    match read_layout_node(bytes, layout)? {
        LayoutNode::Bool => {
            let byte = read_slice(bytes, value, 1)?[0];
            if byte > 1 {
                return Err(RuntimeError::BCSInvalidBool { byte });
            }
            Ok(())
        },
        LayoutNode::Int { width, .. } => {
            read_slice(bytes, value, width)?;
            Ok(())
        },
        LayoutNode::Address => {
            read_slice(bytes, value, AccountAddress::LENGTH)?;
            Ok(())
        },
        LayoutNode::Vector => {
            let len = read_uleb128_len(bytes, value)?;
            if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
                return Err(RuntimeError::BCSSequenceTooLong { len });
            }
            // One element layout serves every element, so each element rewinds
            // the layout cursor to where it started.
            let elem_layout = *layout;
            for i in 0..len {
                if i > 0 {
                    *layout = elem_layout;
                }
                walk_capture(bytes, layout, value)?;
            }
            if len == 0 {
                skip_layout(bytes, layout)?;
            }
            Ok(())
        },
        LayoutNode::Struct { fields } => {
            for _ in 0..fields {
                walk_capture(bytes, layout, value)?;
            }
            Ok(())
        },
        LayoutNode::Enum { variants } => {
            let tag = read_uleb128_len(bytes, value)?;
            if tag >= variants {
                return Err(RuntimeError::BCSInvalidEnumTag {
                    tag,
                    variant_count: variants as usize,
                });
            }
            for variant in 0..variants {
                let fields = read_uleb128_len(bytes, layout)?;
                for _ in 0..fields {
                    if variant == tag {
                        walk_capture(bytes, layout, value)?;
                    } else {
                        skip_layout(bytes, layout)?;
                    }
                }
            }
            Ok(())
        },
        LayoutNode::Function => walk_closure(bytes, value),
    }
}

/// Advances `cursor` past one serialized closure: the `5 + 2n` element sequence
/// of `(format_version, module_id, fun_id, ty_args, mask)` followed by `n`
/// capture pairs.
pub(crate) fn walk_closure(bytes: &[u8], cursor: &mut usize) -> Result<(), RuntimeError> {
    let header = read_closure_header(bytes, cursor)?;
    for _ in 0..header.captured {
        walk_capture_pair(bytes, cursor)?;
    }
    Ok(())
}

/// Advances `cursor` past one `(layout, value)` capture pair.
pub(crate) fn walk_capture_pair(bytes: &[u8], cursor: &mut usize) -> Result<(), RuntimeError> {
    // A value follows its layout, so the layout's end has to be found before
    // the value can be walked. Both passes decode the same tags.
    let mut value = *cursor;
    skip_layout(bytes, &mut value)?;

    let mut layout = *cursor;
    walk_capture(bytes, &mut layout, &mut value)?;
    *cursor = value;
    Ok(())
}

/// A serialized closure's five header elements, borrowed from the wire bytes.
pub(crate) struct ClosureHeader<'b> {
    pub(crate) address: AccountAddress,
    pub(crate) module_name: &'b IdentStr,
    pub(crate) func_name: &'b IdentStr,
    /// The `Vec<TypeTag>` BCS bytes, length prefix included.
    pub(crate) ty_args: &'b [u8],
    pub(crate) mask: u64,
    /// Number of captured values, `mask.count_ones()`.
    pub(crate) captured: u64,
}

/// Advances `cursor` past a closure's `5 + 2n` sequence prefix and its five
/// header elements, returning them.
///
/// The `n` the sequence declares and the `n` the mask implies must agree. That
/// is V1's arity check and, because BCS sequences are length-prefixed, also its
/// check for trailing elements.
pub(crate) fn read_closure_header<'b>(
    bytes: &'b [u8],
    cursor: &mut usize,
) -> Result<ClosureHeader<'b>, RuntimeError> {
    let len = read_uleb128_len(bytes, cursor)?;
    if len < 5 || (len - 5) % 2 != 0 {
        return Err(RuntimeError::BCSInvalidClosure("sequence length"));
    }

    let format_version = u16::from_le_bytes(
        read_slice(bytes, cursor, 2)?
            .try_into()
            .expect("read_slice returned two bytes"),
    );
    if format_version != FUNCTION_DATA_SERIALIZATION_FORMAT_V1 {
        return Err(RuntimeError::BCSInvalidClosure("format version"));
    }

    // A `ModuleId` is an address then an identifier, followed here by the
    // function identifier.
    let address = AccountAddress::from_bytes(read_slice(bytes, cursor, AccountAddress::LENGTH)?)
        .map_err(|_| RuntimeError::BCSInvalidClosure("module address"))?;
    let module_name = read_identifier(bytes, cursor)?;
    let func_name = read_identifier(bytes, cursor)?;

    let ty_args_start = *cursor;
    let ty_args = read_uleb128_len(bytes, cursor)?;
    for _ in 0..ty_args {
        skip_type_tag(bytes, cursor)?;
    }
    let ty_args = &bytes[ty_args_start..*cursor];

    // A `ClosureMask` is a newtype over `u64`.
    let mask = u64::from_le_bytes(
        read_slice(bytes, cursor, 8)?
            .try_into()
            .expect("read_slice returned eight bytes"),
    );
    let captured = mask.count_ones() as u64;
    if len - 5 != captured * 2 {
        return Err(RuntimeError::BCSInvalidClosure("capture count"));
    }
    Ok(ClosureHeader {
        address,
        module_name,
        func_name,
        ty_args,
        mask,
        captured,
    })
}

/// Reads an `Identifier`, a length-prefixed string.
fn read_identifier<'b>(bytes: &'b [u8], cursor: &mut usize) -> Result<&'b IdentStr, RuntimeError> {
    let len = read_uleb128_len(bytes, cursor)?;
    let len = usize::try_from(len).map_err(|_| RuntimeError::BCSEof)?;
    let text = read_slice(bytes, cursor, len)?;
    std::str::from_utf8(text)
        .ok()
        .and_then(|text| IdentStr::new(text).ok())
        .ok_or(RuntimeError::BCSInvalidClosure("identifier"))
}

/// Advances `cursor` past an `Identifier` without validating it.
fn skip_identifier(bytes: &[u8], cursor: &mut usize) -> Result<(), RuntimeError> {
    let len = read_uleb128_len(bytes, cursor)?;
    let len = usize::try_from(len).map_err(|_| RuntimeError::BCSEof)?;
    read_slice(bytes, cursor, len)?;
    Ok(())
}

/// Advances `cursor` past a `TypeTag`.
fn skip_type_tag(bytes: &[u8], cursor: &mut usize) -> Result<(), RuntimeError> {
    let tag = read_uleb128_len(bytes, cursor)?;
    match tag {
        type_tag::BOOL
        | type_tag::U8
        | type_tag::U64
        | type_tag::U128
        | type_tag::ADDRESS
        | type_tag::SIGNER
        | type_tag::U16
        | type_tag::U32
        | type_tag::U256
        | type_tag::I8
        | type_tag::I16
        | type_tag::I32
        | type_tag::I64
        | type_tag::I128
        | type_tag::I256 => Ok(()),
        type_tag::VECTOR => skip_type_tag(bytes, cursor),
        type_tag::STRUCT => {
            // A `StructTag` is an address, a module identifier, a name
            // identifier, then its type arguments.
            read_slice(bytes, cursor, AccountAddress::LENGTH)?;
            skip_identifier(bytes, cursor)?;
            skip_identifier(bytes, cursor)?;
            let ty_args = read_uleb128_len(bytes, cursor)?;
            for _ in 0..ty_args {
                skip_type_tag(bytes, cursor)?;
            }
            Ok(())
        },
        type_tag::FUNCTION => {
            // A `FunctionTag` is its argument tags, its result tags, then an
            // `AbilitySet`, a newtype over `u8`.
            for _ in 0..2 {
                let n = read_uleb128_len(bytes, cursor)?;
                for _ in 0..n {
                    // A `FunctionParamOrReturnTag` wraps one `TypeTag`.
                    read_uleb128_len(bytes, cursor)?;
                    skip_type_tag(bytes, cursor)?;
                }
            }
            read_slice(bytes, cursor, 1)?;
            Ok(())
        },
        tag => Err(RuntimeError::BCSInvalidWireTag {
            what: "TypeTag",
            tag,
        }),
    }
}

// ---------------------------------------------------------------------------
// Comparing
// ---------------------------------------------------------------------------

/// Two captures sit at the same position under layouts of different kinds.
///
/// V1 reports `INTERNAL_TYPE_ERROR` for a value-kind mismatch and defines no
/// cross-kind order. MonoMove has no equivalent error, so it falls back rather
/// than inventing one.
//
// TODO(correctness): `drop_unchanged_writes` compares at session close, where
// `Unsupported` aborts the block instead of falling back. See its own
// `TODO(correctness)` on auditing the `equals` error paths.
fn kind_mismatch() -> RuntimeError {
    RuntimeError::Unsupported("captures compared under layouts of different kinds")
}

/// One side of a two-sided walk over a serialized closure's captures.
struct Cursors<'b> {
    bytes: &'b [u8],
    layout: usize,
    value: usize,
}

impl<'b> Cursors<'b> {
    fn new(bytes: &'b [u8]) -> Self {
        Self {
            bytes,
            layout: 0,
            value: 0,
        }
    }

    /// Positions both cursors on the `(layout, value)` pair that starts where
    /// `value` sits: `layout` at the layout, `value` just past it.
    fn begin_pair(&mut self) -> Result<(), RuntimeError> {
        self.layout = self.value;
        skip_layout(self.bytes, &mut self.value)
    }
}

/// Orders the `(layout, value)*` capture tails of two serialized closures over
/// targets that already compared equal, so both captured `count` values.
///
/// Each side is read under its own embedded layouts. A closure loaded from
/// storage keeps the layouts it was written with, which a later upgrade may
/// have changed, so the two sides can disagree on field counts.
pub(crate) fn compare_captures(a: &[u8], b: &[u8], count: u64) -> Result<Ordering, RuntimeError> {
    let mut a = Cursors::new(a);
    let mut b = Cursors::new(b);
    for _ in 0..count {
        a.begin_pair()?;
        b.begin_pair()?;
        let ord = compare_value(&mut a, &mut b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    Ok(Ordering::Equal)
}

/// Compares one value on each side, each under its own layout, advancing both
/// pairs of cursors past it.
//
// TODO(metering): recursion depth here follows the value, so a deeply nested
// value consumes proportional native stack. Make it iterative.
fn compare_value(a: &mut Cursors, b: &mut Cursors) -> Result<Ordering, RuntimeError> {
    let node_a = read_layout_node(a.bytes, &mut a.layout)?;
    let node_b = read_layout_node(b.bytes, &mut b.layout)?;
    match (node_a, node_b) {
        (LayoutNode::Bool, LayoutNode::Bool) => Ok(read_bool(a)?.cmp(&read_bool(b)?)),
        (
            LayoutNode::Int {
                width: width_a,
                signed: signed_a,
            },
            LayoutNode::Int {
                width: width_b,
                signed: signed_b,
            },
        ) => {
            // V1 holds each width in its own `ValueImpl` variant, so `u64`
            // against `u8` is a type error there, not a numeric comparison.
            if width_a != width_b || signed_a != signed_b {
                return Err(kind_mismatch());
            }
            compare_int(a, b, width_a, signed_a)
        },
        (LayoutNode::Address, LayoutNode::Address) => {
            let bytes_a = read_slice(a.bytes, &mut a.value, AccountAddress::LENGTH)?;
            let bytes_b = read_slice(b.bytes, &mut b.value, AccountAddress::LENGTH)?;
            Ok(bytes_a.cmp(bytes_b))
        },
        (LayoutNode::Vector, LayoutNode::Vector) => compare_vector(a, b),
        (LayoutNode::Struct { fields: fields_a }, LayoutNode::Struct { fields: fields_b }) => {
            compare_fields(a, b, fields_a, fields_b)
        },
        (
            LayoutNode::Enum {
                variants: variants_a,
            },
            LayoutNode::Enum {
                variants: variants_b,
            },
        ) => compare_enum(a, b, variants_a, variants_b),
        (LayoutNode::Function, LayoutNode::Function) => {
            // A nested closure lives entirely on the value side, and the layout
            // cursors are already past the unit `Function` tag. The nested walk
            // reuses the layout cursors, so they are restored after it.
            let (layout_a, layout_b) = (a.layout, b.layout);
            let ord = compare_closure(a, b)?;
            a.layout = layout_a;
            b.layout = layout_b;
            Ok(ord)
        },
        (LayoutNode::Bool, _)
        | (LayoutNode::Int { .. }, _)
        | (LayoutNode::Address, _)
        | (LayoutNode::Vector, _)
        | (LayoutNode::Struct { .. }, _)
        | (LayoutNode::Enum { .. }, _)
        | (LayoutNode::Function, _) => Err(kind_mismatch()),
    }
}

fn read_bool(side: &mut Cursors) -> Result<u8, RuntimeError> {
    let byte = read_slice(side.bytes, &mut side.value, 1)?[0];
    if byte > 1 {
        return Err(RuntimeError::BCSInvalidBool { byte });
    }
    Ok(byte)
}

/// Compares two integers of the same width and signedness.
fn compare_int(
    a: &mut Cursors,
    b: &mut Cursors,
    width: usize,
    signed: bool,
) -> Result<Ordering, RuntimeError> {
    let bytes_a = read_slice(a.bytes, &mut a.value, width)?;
    let bytes_b = read_slice(b.bytes, &mut b.value, width)?;

    if signed {
        // Two's complement puts the sign in the top bit, where it sorts the
        // wrong way round, so it is compared first and inverted.
        let sign_a = bytes_a[width - 1] & 0x80;
        let sign_b = bytes_b[width - 1] & 0x80;
        if sign_a != sign_b {
            return Ok(sign_b.cmp(&sign_a));
        }
    }
    // BCS is little-endian, so the most significant byte is last.
    Ok(bytes_a.iter().rev().cmp(bytes_b.iter().rev()))
}

fn compare_vector(a: &mut Cursors, b: &mut Cursors) -> Result<Ordering, RuntimeError> {
    // V1 stores a vector of primitives in a container variant picked by the
    // element type and compares the variants before the elements, so two
    // vectors whose element kinds disagree are a type error there even when
    // both are empty or lengths already differ.
    if vector_class(a.bytes, a.layout)? != vector_class(b.bytes, b.layout)? {
        return Err(kind_mismatch());
    }

    let len_a = read_vector_len(a)?;
    let len_b = read_vector_len(b)?;

    // One element layout serves every element, so each element rewinds the
    // layout cursors to where they started.
    let (elem_a, elem_b) = (a.layout, b.layout);
    for i in 0..len_a.min(len_b) {
        if i > 0 {
            a.layout = elem_a;
            b.layout = elem_b;
        }
        let ord = compare_value(a, b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    if len_a != len_b {
        return Ok(len_a.cmp(&len_b));
    }
    if len_a == 0 {
        // No element advanced the layout cursors past the element layout.
        skip_layout(a.bytes, &mut a.layout)?;
        skip_layout(b.bytes, &mut b.layout)?;
    }
    Ok(Ordering::Equal)
}

fn read_vector_len(side: &mut Cursors) -> Result<u64, RuntimeError> {
    let len = read_uleb128_len(side.bytes, &mut side.value)?;
    if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
        return Err(RuntimeError::BCSSequenceTooLong { len });
    }
    Ok(len)
}

/// The V1 container variant a vector of this element layout takes: a dedicated
/// one per primitive element type, and one shared by everything else.
fn vector_class(bytes: &[u8], layout: usize) -> Result<Option<u64>, RuntimeError> {
    let mut cursor = layout;
    let tag = read_uleb128_len(bytes, &mut cursor)?;
    Ok(match tag {
        layout_tag::BOOL
        | layout_tag::U8
        | layout_tag::U16
        | layout_tag::U32
        | layout_tag::U64
        | layout_tag::U128
        | layout_tag::U256
        | layout_tag::I8
        | layout_tag::I16
        | layout_tag::I32
        | layout_tag::I64
        | layout_tag::I128
        | layout_tag::I256
        | layout_tag::ADDRESS => Some(tag),
        _ => None,
    })
}

/// Compares a run of field layouts and values on each side, then the counts.
fn compare_fields(
    a: &mut Cursors,
    b: &mut Cursors,
    fields_a: u64,
    fields_b: u64,
) -> Result<Ordering, RuntimeError> {
    for _ in 0..fields_a.min(fields_b) {
        let ord = compare_value(a, b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    Ok(fields_a.cmp(&fields_b))
}

fn compare_enum(
    a: &mut Cursors,
    b: &mut Cursors,
    variants_a: u64,
    variants_b: u64,
) -> Result<Ordering, RuntimeError> {
    let tag_a = read_variant_tag(a, variants_a)?;
    let tag_b = read_variant_tag(b, variants_b)?;
    // V1 decodes an enum into a struct whose first field is the variant tag, so
    // the tag orders ahead of the fields and the field counts it compares are
    // the selected variants' plus one.
    if tag_a != tag_b {
        return Ok(tag_a.cmp(&tag_b));
    }

    let fields_a = enter_variant(a, tag_a)?;
    let fields_b = enter_variant(b, tag_b)?;
    let ord = compare_fields(a, b, fields_a, fields_b)?;
    if ord.is_ne() {
        return Ok(ord);
    }

    skip_variants(a.bytes, &mut a.layout, variants_a - tag_a - 1)?;
    skip_variants(b.bytes, &mut b.layout, variants_b - tag_b - 1)?;
    Ok(Ordering::Equal)
}

fn read_variant_tag(side: &mut Cursors, variants: u64) -> Result<u64, RuntimeError> {
    let tag = read_uleb128_len(side.bytes, &mut side.value)?;
    if tag >= variants {
        return Err(RuntimeError::BCSInvalidEnumTag {
            tag,
            variant_count: variants as usize,
        });
    }
    Ok(tag)
}

/// Advances `layout` to the first field layout of variant `tag`, returning that
/// variant's field count.
fn enter_variant(side: &mut Cursors, tag: u64) -> Result<u64, RuntimeError> {
    skip_variants(side.bytes, &mut side.layout, tag)?;
    read_uleb128_len(side.bytes, &mut side.layout)
}

/// Compares two nested closures, advancing both value cursors past them.
fn compare_closure(a: &mut Cursors, b: &mut Cursors) -> Result<Ordering, RuntimeError> {
    let header_a = read_closure_header(a.bytes, &mut a.value)?;
    let header_b = read_closure_header(b.bytes, &mut b.value)?;
    let ord = compare_closure_headers(&header_a, &header_b)?;
    if ord.is_ne() {
        return Ok(ord);
    }

    // Equal masks mean equal capture counts.
    for _ in 0..header_a.captured {
        a.begin_pair()?;
        b.begin_pair()?;
        let ord = compare_value(a, b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    Ok(Ordering::Equal)
}

/// Orders two serialized closures by target then mask, the order V1's `cmp_dyn`
/// defines.
fn compare_closure_headers(a: &ClosureHeader, b: &ClosureHeader) -> Result<Ordering, RuntimeError> {
    let ord = a
        .address
        .cmp(&b.address)
        .then_with(|| a.module_name.cmp(b.module_name))
        .then_with(|| a.func_name.cmp(b.func_name));
    if ord.is_ne() {
        return Ok(ord);
    }
    let ord = read_ty_args(a.ty_args)?.cmp(&read_ty_args(b.ty_args)?);
    Ok(ord.then_with(|| a.mask.cmp(&b.mask)))
}

/// Decodes a closure header's type arguments. `TypeTag` orders by declaration,
/// which its BCS bytes do not reproduce, so they have to be decoded.
fn read_ty_args(bytes: &[u8]) -> Result<Vec<TypeTag>, RuntimeError> {
    bcs::from_bytes(bytes).map_err(|_| RuntimeError::BCSInvalidClosure("type arguments"))
}

/// Orders two function references the way V1's `cmp_dyn` orders closure
/// targets: module id, then function name, then type arguments.
pub(crate) fn compare_func_refs(a: &FunctionRef, b: &FunctionRef) -> VMResult<Ordering> {
    let (module_a, module_b) = (view_module_id(a.module_id), view_module_id(b.module_id));
    let ord = module_a
        .address()
        .cmp(module_b.address())
        .then_with(|| view_name(module_a.name()).cmp(view_name(module_b.name())))
        .then_with(|| view_name(a.func_name).cmp(view_name(b.func_name)));
    if ord.is_ne() {
        return Ok(ord);
    }
    Ok(func_ref_ty_args(a)?.cmp(&func_ref_ty_args(b)?))
}

fn func_ref_ty_args(func_ref: &FunctionRef) -> VMResult<Vec<TypeTag>> {
    view_type_list(func_ref.ty_args)
        .iter()
        .map(|&ty| {
            type_tag_of(ty).ok_or_else(|| {
                VMInternalError::new(RuntimeError::Unsupported(
                    "function value over a type with no type tag",
                ))
            })
        })
        .collect::<VMResult<Vec<TypeTag>>>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::{
        interner::InternedIdentifier,
        types::{U64_TY, U8_TY},
        value_layout::{
            ADDRESS_LAYOUT_ID, BOOL_LAYOUT_ID, FUNCTION_LAYOUT_ID, I32_LAYOUT_ID, U128_LAYOUT_ID,
            U16_LAYOUT_ID, U64_LAYOUT_ID, U8_LAYOUT_ID,
        },
        DescriptorId, FieldValueLayout, LayoutFlags, ValueLayoutTable, VariantValueLayout,
    };
    use move_core_types::{
        account_address::AccountAddress,
        function::ClosureMask,
        identifier::Identifier,
        language_storage::{ModuleId, StructTag, TypeTag},
        value::{IdentifierMappingKind, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    };
    use serde::Serialize;
    use std::sync::Arc;

    /// Emits `id` and checks the bytes against `expected` both structurally and
    /// byte for byte. `MoveTypeLayout` has no `PartialEq`, so the structural
    /// half compares debug renderings.
    fn assert_emits(layouts: &ValueLayoutTable, id: LayoutId, expected: &MoveTypeLayout) {
        let mut out = vec![];
        emit_move_type_layout(layouts, id, &mut out).unwrap();
        let decoded = bcs::from_bytes::<MoveTypeLayout>(&out)
            .expect("emitted bytes decode as a MoveTypeLayout");
        assert_eq!(format!("{decoded:?}"), format!("{expected:?}"));
        assert_eq!(out, bcs::to_bytes(expected).unwrap());
    }

    fn field(name: &'static str, offset: u32, id: LayoutId) -> FieldValueLayout {
        FieldValueLayout {
            offset,
            id,
            name: InternedIdentifier::from_static(name),
        }
    }

    #[test]
    fn emit_primitives() {
        let layouts = ValueLayoutTable::new();
        let cases = [
            (BOOL_LAYOUT_ID, MoveTypeLayout::Bool),
            (U8_LAYOUT_ID, MoveTypeLayout::U8),
            (U16_LAYOUT_ID, MoveTypeLayout::U16),
            (U64_LAYOUT_ID, MoveTypeLayout::U64),
            (U128_LAYOUT_ID, MoveTypeLayout::U128),
            (I32_LAYOUT_ID, MoveTypeLayout::I32),
            (ADDRESS_LAYOUT_ID, MoveTypeLayout::Address),
            (FUNCTION_LAYOUT_ID, MoveTypeLayout::Function),
        ];
        for (id, expected) in &cases {
            assert_emits(&layouts, *id, expected);
        }
    }

    #[test]
    fn emit_vector() {
        let mut layouts = ValueLayoutTable::new();
        let id = layouts.push(ValueLayout::vector(U8_TY, U8_LAYOUT_ID, DescriptorId(2)));
        assert_emits(
            &layouts,
            id,
            &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
        );
    }

    #[test]
    fn emit_struct() {
        let mut layouts = ValueLayoutTable::new();
        let id = layouts.push_anonymous(ValueLayout::struct_layout(
            None,
            16,
            8,
            None,
            LayoutFlags::empty(),
            vec![field("a", 0, U64_LAYOUT_ID), field("b", 8, BOOL_LAYOUT_ID)].into_boxed_slice(),
        ));
        let expected = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::Bool,
        ])));
        assert_emits(&layouts, id, &expected);
    }

    #[test]
    fn emit_enum() {
        let mut layouts = ValueLayoutTable::new();
        let none = layouts.push_anonymous(ValueLayout::struct_layout(
            None,
            0,
            1,
            Some(0),
            LayoutFlags::NO_POINTERS_NO_PADDING,
            Box::new([]),
        ));
        let some = layouts.push_anonymous(ValueLayout::struct_layout(
            None,
            8,
            8,
            Some(8),
            LayoutFlags::NO_POINTERS_NO_PADDING,
            vec![field("x", 0, U64_LAYOUT_ID)].into_boxed_slice(),
        ));
        let id = layouts.push(ValueLayout::frozen_enum(
            U64_TY,
            DescriptorId(3),
            vec![
                VariantValueLayout {
                    name: InternedIdentifier::from_static("None"),
                    id: none,
                },
                VariantValueLayout {
                    name: InternedIdentifier::from_static("Some"),
                    id: some,
                },
            ]
            .into_boxed_slice(),
            16,
        ));
        let expected = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::RuntimeVariants(vec![
            vec![],
            vec![MoveTypeLayout::U64],
        ])));
        assert_emits(&layouts, id, &expected);
    }

    /// Emits `layout` then a BCS-encoded `value`, and checks that one
    /// `walk_capture` lands both cursors exactly at the end.
    fn check_walk<V: Serialize>(layout: &MoveTypeLayout, value: &V) {
        let mut bytes = bcs::to_bytes(layout).unwrap();
        let layout_end = bytes.len();
        bytes.extend_from_slice(&bcs::to_bytes(value).unwrap());
        check_walk_bytes(&bytes, layout_end);
    }

    fn check_walk_bytes(bytes: &[u8], layout_end: usize) {
        let mut layout_cursor = 0;
        let mut value_cursor = layout_end;
        walk_capture(bytes, &mut layout_cursor, &mut value_cursor).unwrap();
        assert_eq!(layout_cursor, layout_end, "layout cursor");
        assert_eq!(value_cursor, bytes.len(), "value cursor");
    }

    fn walk_err(bytes: &[u8], layout_end: usize) -> RuntimeError {
        let mut layout_cursor = 0;
        let mut value_cursor = layout_end;
        walk_capture(bytes, &mut layout_cursor, &mut value_cursor)
            .expect_err("expected the walk to fail")
    }

    #[test]
    fn walk_primitives() {
        check_walk(&MoveTypeLayout::Bool, &true);
        check_walk(&MoveTypeLayout::U8, &7u8);
        check_walk(&MoveTypeLayout::U16, &7u16);
        check_walk(&MoveTypeLayout::U64, &u64::MAX);
        check_walk(&MoveTypeLayout::U128, &u128::MAX);
        check_walk(&MoveTypeLayout::Address, &AccountAddress::ONE);
    }

    #[test]
    fn walk_vectors() {
        check_walk(
            &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
            &vec![1u8, 2, 3],
        );
        check_walk(
            &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
            &Vec::<u8>::new(),
        );
        check_walk(
            &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::Vector(Box::new(
                MoveTypeLayout::U64,
            )))),
            &vec![vec![1u64], vec![], vec![2u64, 3]],
        );
    }

    #[test]
    fn walk_struct() {
        #[derive(Serialize)]
        struct S {
            a: u64,
            b: bool,
            c: Vec<u8>,
        }
        let layout = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::Bool,
            MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
        ])));
        check_walk(&layout, &S {
            a: 9,
            b: true,
            c: vec![4, 5],
        });
    }

    /// Three variants of two, zero and one `u64` field.
    fn enum_layout() -> MoveTypeLayout {
        MoveTypeLayout::Struct(Arc::new(MoveStructLayout::RuntimeVariants(vec![
            vec![MoveTypeLayout::U64, MoveTypeLayout::U64],
            vec![],
            vec![MoveTypeLayout::U64],
        ])))
    }

    /// Builds [`enum_layout`] followed by an enum value: a ULEB variant tag
    /// then its fields. Returns the bytes and where the layout ends.
    fn enum_bytes(tag: u64, fields: &[u64]) -> (Vec<u8>, usize) {
        let mut bytes = bcs::to_bytes(&enum_layout()).unwrap();
        let layout_end = bytes.len();
        write_uleb128_len(&mut bytes, tag);
        for value in fields {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        (bytes, layout_end)
    }

    #[test]
    fn walk_enum_skips_unselected_variants() {
        for (tag, fields) in [(0u64, vec![1u64, 2]), (1, vec![]), (2, vec![3])] {
            let (bytes, layout_end) = enum_bytes(tag, &fields);
            check_walk_bytes(&bytes, layout_end);
        }
    }

    #[test]
    fn walk_rejects_out_of_range_enum_tag() {
        let (bytes, layout_end) = enum_bytes(3, &[]);
        assert!(matches!(
            walk_err(&bytes, layout_end),
            RuntimeError::BCSInvalidEnumTag {
                tag: 3,
                variant_count: 3
            }
        ));
    }

    #[test]
    fn walk_rejects_signer_and_native() {
        for layout in [
            MoveTypeLayout::Signer,
            MoveTypeLayout::Native(
                IdentifierMappingKind::Aggregator,
                Box::new(MoveTypeLayout::U64),
            ),
        ] {
            let bytes = bcs::to_bytes(&layout).unwrap();
            let layout_end = bytes.len();
            assert!(
                matches!(walk_err(&bytes, layout_end), RuntimeError::Unsupported(_)),
                "{layout:?} must fall back to V1"
            );
        }
    }

    #[test]
    fn walk_rejects_decorated_struct_layouts() {
        let layouts = [
            MoveStructLayout::WithFields(vec![MoveFieldLayout::new(
                Identifier::new("a").unwrap(),
                MoveTypeLayout::U64,
            )]),
            MoveStructLayout::WithTypes {
                type_: StructTag {
                    address: AccountAddress::ONE,
                    module: Identifier::new("m").unwrap(),
                    name: Identifier::new("S").unwrap(),
                    type_args: vec![],
                },
                fields: vec![],
            },
        ];
        for layout in layouts {
            let bytes = bcs::to_bytes(&MoveTypeLayout::Struct(Arc::new(layout))).unwrap();
            let layout_end = bytes.len();
            assert!(matches!(
                walk_err(&bytes, layout_end),
                RuntimeError::Unsupported(_)
            ));
        }
    }

    #[test]
    fn walk_rejects_unknown_tag() {
        let bytes = [99u8, 0, 0, 0];
        assert!(matches!(
            walk_err(&bytes, 1),
            RuntimeError::BCSInvalidWireTag {
                what: "MoveTypeLayout",
                tag: 99
            }
        ));
    }

    #[test]
    fn walk_rejects_truncated_value() {
        let mut bytes = bcs::to_bytes(&MoveTypeLayout::U64).unwrap();
        let layout_end = bytes.len();
        bytes.extend_from_slice(&[0u8; 4]);
        assert!(matches!(walk_err(&bytes, layout_end), RuntimeError::BCSEof));
    }

    #[test]
    fn walk_rejects_truncated_layout() {
        // A vector tag with no element layout after it.
        let bytes = bcs::to_bytes(&MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8))).unwrap();
        assert!(matches!(walk_err(&bytes[..1], 1), RuntimeError::BCSEof));
    }

    #[test]
    fn walk_vector_rewinds_element_layout() {
        // One element layout serves every element, so the layout cursor rewinds
        // between elements and ends where the value begins.
        let layout = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8));
        check_walk(&layout, &vec![0u8; 8192]);
    }

    /// Builds the V1 wire bytes of a closure over `0x1::m::f`, with `captures`
    /// already holding the encoded capture pairs.
    fn closure_bytes(ty_args: &[TypeTag], mask: u64, captures: &[u8], captured: usize) -> Vec<u8> {
        let mut out = vec![];
        write_uleb128_len(&mut out, (5 + captured * 2) as u64);
        out.extend_from_slice(&FUNCTION_DATA_SERIALIZATION_FORMAT_V1.to_le_bytes());
        let module = ModuleId::new(AccountAddress::ONE, Identifier::new("m").unwrap());
        out.extend_from_slice(&bcs::to_bytes(&module).unwrap());
        out.extend_from_slice(&bcs::to_bytes(&Identifier::new("f").unwrap()).unwrap());
        out.extend_from_slice(&bcs::to_bytes(&ty_args.to_vec()).unwrap());
        out.extend_from_slice(&bcs::to_bytes(&ClosureMask::new(mask)).unwrap());
        out.extend_from_slice(captures);
        out
    }

    fn capture<V: Serialize>(out: &mut Vec<u8>, layout: &MoveTypeLayout, value: &V) {
        out.extend_from_slice(&bcs::to_bytes(layout).unwrap());
        out.extend_from_slice(&bcs::to_bytes(value).unwrap());
    }

    fn check_walk_closure(bytes: &[u8]) {
        let mut cursor = 0;
        walk_closure(bytes, &mut cursor).unwrap();
        assert_eq!(cursor, bytes.len());
    }

    fn walk_closure_err(bytes: &[u8]) -> RuntimeError {
        let mut cursor = 0;
        walk_closure(bytes, &mut cursor).expect_err("expected the walk to fail")
    }

    #[test]
    fn walk_closure_without_captures() {
        check_walk_closure(&closure_bytes(&[], 0, &[], 0));
    }

    #[test]
    fn walk_closure_with_captures() {
        let mut captures = vec![];
        capture(&mut captures, &MoveTypeLayout::U64, &7u64);
        capture(
            &mut captures,
            &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
            &vec![1u8, 2],
        );
        check_walk_closure(&closure_bytes(&[TypeTag::U64], 0b11, &captures, 2));
    }

    #[test]
    fn walk_closure_with_struct_type_argument() {
        let ty_args = vec![TypeTag::Struct(Box::new(StructTag {
            address: AccountAddress::ONE,
            module: Identifier::new("m").unwrap(),
            name: Identifier::new("S").unwrap(),
            type_args: vec![TypeTag::Vector(Box::new(TypeTag::U8))],
        }))];
        check_walk_closure(&closure_bytes(&ty_args, 0, &[], 0));
    }

    #[test]
    fn walk_nested_closure_capture() {
        // The layout side of a function-typed capture is the unit `Function`
        // tag; the value side is a whole nested closure.
        let mut inner_captures = vec![];
        capture(&mut inner_captures, &MoveTypeLayout::U64, &5u64);
        let inner = closure_bytes(&[], 0b1, &inner_captures, 1);

        let mut captures = bcs::to_bytes(&MoveTypeLayout::Function).unwrap();
        captures.extend_from_slice(&inner);
        check_walk_closure(&closure_bytes(&[], 0b1, &captures, 1));
    }

    #[test]
    fn walk_closure_rejects_capture_count_mismatch() {
        let mut captures = vec![];
        capture(&mut captures, &MoveTypeLayout::U64, &7u64);
        // The sequence declares one capture but the mask bits say two.
        let bytes = closure_bytes(&[], 0b11, &captures, 1);
        assert!(matches!(
            walk_closure_err(&bytes),
            RuntimeError::BCSInvalidClosure("capture count")
        ));
    }

    #[test]
    fn walk_closure_rejects_bad_format_version() {
        let mut bytes = closure_bytes(&[], 0, &[], 0);
        bytes[1] = 9;
        assert!(matches!(
            walk_closure_err(&bytes),
            RuntimeError::BCSInvalidClosure("format version")
        ));
    }

    #[test]
    fn walk_closure_rejects_odd_sequence_length() {
        let mut bytes = closure_bytes(&[], 0, &[], 0);
        bytes[0] = 6;
        assert!(matches!(
            walk_closure_err(&bytes),
            RuntimeError::BCSInvalidClosure("sequence length")
        ));
    }

    #[test]
    fn walk_closure_rejects_short_sequence_length() {
        let mut bytes = closure_bytes(&[], 0, &[], 0);
        bytes[0] = 4;
        assert!(matches!(
            walk_closure_err(&bytes),
            RuntimeError::BCSInvalidClosure("sequence length")
        ));
    }

    #[test]
    fn walk_closure_rejects_truncation() {
        let mut captures = vec![];
        capture(&mut captures, &MoveTypeLayout::U64, &7u64);
        let bytes = closure_bytes(&[], 0b1, &captures, 1);
        for end in 1..bytes.len() {
            let mut cursor = 0;
            assert!(
                walk_closure(&bytes[..end], &mut cursor).is_err(),
                "truncating to {end} bytes must fail"
            );
        }
    }

    /// One capture tail holding a single `(layout, value)` pair.
    fn one_capture<V: Serialize>(layout: &MoveTypeLayout, value: &V) -> Vec<u8> {
        let mut out = vec![];
        capture(&mut out, layout, value);
        out
    }

    fn cmp_one<V: Serialize, W: Serialize>(
        layout_a: &MoveTypeLayout,
        a: &V,
        layout_b: &MoveTypeLayout,
        b: &W,
    ) -> Result<Ordering, RuntimeError> {
        compare_captures(&one_capture(layout_a, a), &one_capture(layout_b, b), 1)
    }

    /// Compares `a` against `b` under one shared layout.
    fn cmp_same<V: Serialize, W: Serialize>(
        layout: &MoveTypeLayout,
        a: &V,
        b: &W,
    ) -> Result<Ordering, RuntimeError> {
        cmp_one(layout, a, layout, b)
    }

    #[test]
    fn compare_unsigned_is_numeric_not_byte_order() {
        // Little-endian bytes order these the other way round.
        assert_eq!(
            cmp_same(&MoveTypeLayout::U64, &1u64, &256u64).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_same(&MoveTypeLayout::U128, &u128::MAX, &0u128).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            cmp_same(&MoveTypeLayout::U16, &7u16, &7u16).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_signed_orders_by_sign_first() {
        for (a, b, expected) in [
            (-1i64, 0i64, Ordering::Less),
            (0, -1, Ordering::Greater),
            (-2, -1, Ordering::Less),
            (i64::MIN, i64::MAX, Ordering::Less),
            (-5, -5, Ordering::Equal),
        ] {
            assert_eq!(
                cmp_same(&MoveTypeLayout::I64, &a, &b).unwrap(),
                expected,
                "{a} against {b}"
            );
        }
    }

    #[test]
    fn compare_bools_and_addresses() {
        assert_eq!(
            cmp_same(&MoveTypeLayout::Bool, &false, &true).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_same(
                &MoveTypeLayout::Address,
                &AccountAddress::ONE,
                &AccountAddress::TWO
            )
            .unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_vectors_by_element_then_length() {
        let layout = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8));
        // The longer vector loses on its first element, which the ULEB length
        // prefix would hide from a byte comparison.
        assert_eq!(
            cmp_same(&layout, &vec![9u8], &vec![1u8, 1]).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            cmp_same(&layout, &vec![1u8], &vec![1u8, 1]).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_same(&layout, &Vec::<u8>::new(), &Vec::<u8>::new()).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_vectors_of_different_element_kinds_is_a_type_error() {
        // V1 puts these in different container variants and compares the
        // variants before it looks inside, so even two empty ones are an error.
        let u8s = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8));
        let u64s = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U64));
        for (a, b) in [(vec![], vec![]), (vec![1u8], vec![1u64])] {
            assert!(matches!(
                cmp_one(&u8s, &a, &u64s, &b).unwrap_err(),
                RuntimeError::Unsupported(_)
            ));
        }
    }

    #[test]
    fn compare_struct_by_field_then_count() {
        let one = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
        ])));
        let two = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::U64,
        ])));
        // A stored closure can carry a pre-upgrade layout, so differing field
        // counts order by count rather than erroring.
        assert_eq!(
            cmp_one(&one, &(7u64,), &two, &(7u64, 0u64)).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_one(&one, &(8u64,), &two, &(7u64, 0u64)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            cmp_same(&two, &(7u64, 1u64), &(7u64, 1u64)).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_struct_against_primitive_is_a_type_error() {
        let s = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
        ])));
        assert!(matches!(
            cmp_one(&s, &(7u64,), &MoveTypeLayout::U64, &7u64).unwrap_err(),
            RuntimeError::Unsupported(_)
        ));
    }

    #[test]
    fn compare_ints_of_different_widths_is_a_type_error() {
        for (a, b) in [
            (MoveTypeLayout::U8, MoveTypeLayout::U64),
            (MoveTypeLayout::U64, MoveTypeLayout::I64),
        ] {
            assert!(matches!(
                cmp_one(&a, &0u8, &b, &0u8).unwrap_err(),
                RuntimeError::Unsupported(_)
            ));
        }
    }

    /// Builds a capture tail holding one value of [`enum_layout`].
    fn enum_capture(tag: u64, fields: &[u64]) -> Vec<u8> {
        let mut out = bcs::to_bytes(&enum_layout()).unwrap();
        write_uleb128_len(&mut out, tag);
        for value in fields {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    #[test]
    fn compare_enum_by_tag_then_fields() {
        let cases = [
            ((0u64, vec![1u64, 2]), (1u64, vec![]), Ordering::Less),
            ((2, vec![3]), (1, vec![]), Ordering::Greater),
            ((0, vec![1, 2]), (0, vec![1, 3]), Ordering::Less),
            ((2, vec![3]), (2, vec![3]), Ordering::Equal),
        ];
        for ((tag_a, fields_a), (tag_b, fields_b), expected) in cases {
            let ord = compare_captures(
                &enum_capture(tag_a, &fields_a),
                &enum_capture(tag_b, &fields_b),
                1,
            )
            .unwrap();
            assert_eq!(ord, expected, "variant {tag_a} against {tag_b}");
        }
    }

    #[test]
    fn compare_enum_leaves_cursors_at_the_end() {
        // Two captures in a row: the first has to land the cursors exactly at
        // the start of the second, whichever variant it selects.
        for tag in 0..3u64 {
            let fields = match tag {
                0 => vec![1u64, 2],
                1 => vec![],
                _ => vec![3],
            };
            let mut bytes = enum_capture(tag, &fields);
            capture(&mut bytes, &MoveTypeLayout::U64, &9u64);
            assert_eq!(
                compare_captures(&bytes, &bytes, 2).unwrap(),
                Ordering::Equal
            );

            let mut other = enum_capture(tag, &fields);
            capture(&mut other, &MoveTypeLayout::U64, &10u64);
            assert_eq!(compare_captures(&bytes, &other, 2).unwrap(), Ordering::Less);
        }
    }

    #[test]
    fn compare_vector_leaves_cursors_at_the_end() {
        for elements in [vec![], vec![1u64], vec![1u64, 2]] {
            let mut bytes = one_capture(
                &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U64)),
                &elements,
            );
            capture(&mut bytes, &MoveTypeLayout::U64, &9u64);
            let mut other = bytes.clone();
            *other.last_mut().unwrap() = 1;
            assert_eq!(
                compare_captures(&bytes, &bytes, 2).unwrap(),
                Ordering::Equal
            );
            assert_eq!(
                compare_captures(&bytes, &other, 2).unwrap(),
                Ordering::Less,
                "{elements:?}"
            );
        }
    }

    #[test]
    fn compare_nested_closures() {
        let inner = |value: u64| {
            let mut captures = vec![];
            capture(&mut captures, &MoveTypeLayout::U64, &value);
            let mut out = bcs::to_bytes(&MoveTypeLayout::Function).unwrap();
            out.extend_from_slice(&closure_bytes(&[], 0b1, &captures, 1));
            out
        };
        assert_eq!(
            compare_captures(&inner(1), &inner(1), 1).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            compare_captures(&inner(1), &inner(2), 1).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_nested_closures_by_target() {
        // Same shape, different type arguments: the header decides.
        let closure = |ty_args: &[TypeTag]| {
            let mut out = bcs::to_bytes(&MoveTypeLayout::Function).unwrap();
            out.extend_from_slice(&closure_bytes(ty_args, 0, &[], 0));
            out
        };
        assert_eq!(
            compare_captures(&closure(&[TypeTag::U8]), &closure(&[TypeTag::U64]), 1).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare_captures(&closure(&[]), &closure(&[TypeTag::U8]), 1).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_is_a_total_order() {
        // Sorting by the comparison must agree with it pairwise, which catches
        // a cursor left mid-value on an inequality.
        let layout = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
        ])));
        let values = [
            (0u64, vec![]),
            (0u64, vec![1u8]),
            (1u64, vec![]),
            (1u64, vec![0u8, 0]),
            (u64::MAX, vec![255u8]),
        ];
        for (i, a) in values.iter().enumerate() {
            for (j, b) in values.iter().enumerate() {
                assert_eq!(
                    cmp_same(&layout, a, b).unwrap(),
                    i.cmp(&j),
                    "{a:?} against {b:?}"
                );
            }
        }
    }

    #[test]
    fn compare_rejects_signer_and_native() {
        for layout in [
            MoveTypeLayout::Signer,
            MoveTypeLayout::Native(
                IdentifierMappingKind::Aggregator,
                Box::new(MoveTypeLayout::U64),
            ),
        ] {
            let bytes = one_capture(&layout, &AccountAddress::ONE);
            assert!(matches!(
                compare_captures(&bytes, &bytes, 1).unwrap_err(),
                RuntimeError::Unsupported(_)
            ));
        }
    }

    #[test]
    fn compare_rejects_truncation() {
        let bytes = one_capture(&MoveTypeLayout::U64, &7u64);
        for end in 1..bytes.len() {
            assert!(
                compare_captures(&bytes[..end], &bytes, 1).is_err(),
                "truncating to {end} bytes must fail"
            );
        }
    }
}
