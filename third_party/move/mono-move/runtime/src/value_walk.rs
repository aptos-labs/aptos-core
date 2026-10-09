// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Iterative traversal of VM values, driven by their layout.
//!
//! [`walk`] visits one value, or `N` values of the same layout in lockstep,
//! and reports what it finds to a [`ValueVisitor`] as a stream of [`Event`]s:
//! a scalar, or the entry into and exit from a struct, a vector, or an enum,
//! with one [`Event::Field`] / [`Event::Element`] ahead of each child. The
//! walk owns all layout lookups, pointer arithmetic, vector length and enum
//! tag reads, and the tag range check; a visitor only decides what to do with
//! each node.
//!
//! The walk keeps an explicit frame stack rather than recursing, so a deeply
//! nested value costs heap rather than Rust stack, and the nesting is capped
//! at [`MAX_VALUE_DEPTH`]: past that the walk fails with
//! [`RuntimeError::ValueTooDeep`] instead of overflowing the stack.
//!
//! Clients: structural equality and ordering (`value_cmp.rs`, `N = 2`), BCS
//! serialization (`value_conv/bcs.rs`, `N = 1`), and text rendering
//! (`value_display.rs`, `N = 1`). Each keeps its fast paths by answering an
//! `Enter*` event with [`Step::Skip`] after handling the subtree itself, e.g.
//! one `memcmp` or `memcpy` for a pointer-free, padding-free struct.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    memory::{read_enum_tag, read_ptr, read_vec_len},
    types::VEC_DATA_OFFSET,
};
use mono_move_core::{
    FieldValueLayout, LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult, ValueLayout,
    VariantValueLayout, ENUM_DATA_OFFSET,
};

/// Deepest nesting of aggregates a walk descends into before failing.
///
/// Move forbids recursive struct and enum definitions, so a value nests no
/// deeper than its type, and V1 caps type depth at `max_ty_depth` (20 on
/// mainnet). This leaves generous headroom while keeping the frame stack to a
/// few KiB.
// TODO(metering): tie this to the VM config / gas schedule rather than a
// constant, and charge for the walk itself.
pub const MAX_VALUE_DEPTH: usize = 128;

/// What a visitor wants the walk to do after an event.
///
/// After an `Enter*` event, [`Step::Descend`] visits the children and
/// [`Step::Skip`] jumps straight to the matching `Exit*`. After a
/// [`Event::Field`] or [`Event::Element`], [`Step::Skip`] skips that child
/// and every later sibling. After a scalar or an `Exit*`, `Descend` and `Skip`
/// both just continue. [`Step::Break`] ends the whole walk with its payload.
#[derive(Debug)]
pub enum Step<B> {
    Descend,
    Skip,
    Break(B),
}

/// One node of the walk, over `N` values that share a layout.
///
/// Pointer arrays are indexed by value: `ptrs[i]` belongs to the `i`-th value
/// passed to [`walk`]. Every pointer is valid for its layout's size, as
/// guaranteed by the caller of [`walk`].
pub enum Event<'a, const N: usize> {
    /// A bool, integer, address or signer; `ptrs` address `layout.size` bytes.
    Scalar {
        layout: &'a ValueLayout,
        ptrs: [*const u8; N],
    },
    /// A struct, or an enum variant body (whose `layout.ty` is `None`); `ptrs`
    /// address the whole struct.
    EnterStruct {
        layout: &'a ValueLayout,
        fields: &'a [FieldValueLayout],
        ptrs: [*const u8; N],
    },
    /// Precedes the value of field `index` of the innermost open struct;
    /// `ptrs` address the field.
    Field {
        index: usize,
        field: &'a FieldValueLayout,
        ptrs: [*const u8; N],
    },
    ExitStruct {
        layout: &'a ValueLayout,
    },
    /// A vector. `lens` are the per-value lengths; `data[i]` addresses the
    /// first element of value `i`, or is null when that vector owns no
    /// storage. With `N > 1` the walk visits `min(lens)` elements.
    EnterVector {
        layout: &'a ValueLayout,
        elem_id: LayoutId,
        elem: &'a ValueLayout,
        lens: [u64; N],
        data: [*const u8; N],
    },
    /// Precedes element `index` of the innermost open vector.
    Element {
        index: u64,
        ptrs: [*const u8; N],
    },
    ExitVector {
        layout: &'a ValueLayout,
        lens: [u64; N],
    },
    /// An enum. Every tag is in range for `variants`. `body` is the layout of
    /// the selected variant when all tags agree, and `None` otherwise; the walk
    /// descends into the body only when they agree, so with `N > 1` a visitor
    /// must settle differing tags here. `bodies` address the variant payloads.
    EnterEnum {
        layout: &'a ValueLayout,
        variants: &'a [VariantValueLayout],
        tags: [u64; N],
        body: Option<&'a ValueLayout>,
        bodies: [*const u8; N],
    },
    ExitEnum {
        layout: &'a ValueLayout,
    },
}

/// Receives the events of a walk.
pub trait ValueVisitor<const N: usize> {
    /// Payload of an early exit; use `Infallible` for a visitor that never
    /// breaks.
    type Break;

    fn visit(&mut self, event: Event<'_, N>) -> VMResult<Step<Self::Break>>;
}

/// Looks up a layout, reporting a missing one as an invariant violation.
pub(crate) fn lookup<L: LayoutProvider + ?Sized>(
    layouts: &L,
    id: LayoutId,
) -> VMResult<&ValueLayout> {
    layouts.layout(id).ok_or_else(|| {
        VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::ValueLayoutNotFound,
        ))
    })
}

/// Walks `N` values of layout `root` in lockstep, feeding `visitor`.
///
/// Returns `Ok(None)` when the walk ran to completion and `Ok(Some(b))` when
/// the visitor answered [`Step::Break`]`(b)`.
///
/// # Safety
///
/// Every pointer in `ptrs` must point to a fully initialized value of layout
/// `root` that stays live, with everything it reaches, for the duration of the
/// call.
///
/// # Precondition
///
/// `root` must not be the reference layout: a caller holding a reference reads
/// it first and walks the pointee.
pub unsafe fn walk<'a, L, V, const N: usize>(
    layouts: &'a L,
    ptrs: [*const u8; N],
    root: &'a ValueLayout,
    visitor: &mut V,
) -> VMResult<Option<V::Break>>
where
    L: LayoutProvider + ?Sized,
    V: ValueVisitor<N>,
{
    let () = NonEmpty::<N>::CHECK;
    let mut stack: Vec<Frame<'a, N>> = Vec::new();

    // SAFETY: forwarded from this function's contract.
    if let Some(b) = unsafe { enter(layouts, &mut stack, ptrs, root, visitor)? } {
        return Ok(Some(b));
    }
    loop {
        let Some(top) = stack.last_mut() else {
            return Ok(None);
        };
        let child = match top {
            Frame::Struct {
                fields,
                bases,
                next,
                ..
            } => {
                let fields: &'a [FieldValueLayout] = fields;
                if *next < fields.len() {
                    let index = *next;
                    *next += 1;
                    let field = &fields[index];
                    // SAFETY: every field lies at its offset within the struct.
                    let ptrs = bases.map(|base| unsafe { base.add(field.offset as usize) });
                    Some(Child::Field { index, field, ptrs })
                } else {
                    None
                }
            },
            Frame::Vector {
                elem_id,
                elem_size,
                data,
                len,
                next,
                ..
            } => {
                if *next < *len {
                    let index = *next;
                    *next += 1;
                    // SAFETY: `index < len`, so the element lies within every
                    // vector's data region and `data` is non-null.
                    let ptrs = data.map(|data| unsafe { data.add(index as usize * *elem_size) });
                    Some(Child::Element {
                        index,
                        ptrs,
                        id: *elem_id,
                    })
                } else {
                    None
                }
            },
            Frame::Enum {
                body,
                bodies,
                entered,
                ..
            } => {
                if *entered {
                    None
                } else {
                    *entered = true;
                    Some(Child::Body {
                        ptrs: *bodies,
                        layout: body,
                    })
                }
            },
        };

        match child {
            Some(Child::Field { index, field, ptrs }) => {
                match visitor.visit(Event::Field { index, field, ptrs })? {
                    Step::Descend => {
                        let layout = lookup(layouts, field.id)?;
                        // SAFETY: a field of a live struct is a live value of
                        // the field's layout.
                        if let Some(b) =
                            unsafe { enter(layouts, &mut stack, ptrs, layout, visitor)? }
                        {
                            return Ok(Some(b));
                        }
                    },
                    Step::Skip => finish_children(&mut stack),
                    Step::Break(b) => return Ok(Some(b)),
                }
            },
            Some(Child::Element { index, ptrs, id }) => {
                match visitor.visit(Event::Element { index, ptrs })? {
                    Step::Descend => {
                        let layout = lookup(layouts, id)?;
                        // SAFETY: an element of a live vector is a live value
                        // of the element layout.
                        if let Some(b) =
                            unsafe { enter(layouts, &mut stack, ptrs, layout, visitor)? }
                        {
                            return Ok(Some(b));
                        }
                    },
                    Step::Skip => finish_children(&mut stack),
                    Step::Break(b) => return Ok(Some(b)),
                }
            },
            Some(Child::Body { ptrs, layout }) => {
                // SAFETY: the variant body of a live enum object is a live
                // value of the body layout.
                if let Some(b) = unsafe { enter(layouts, &mut stack, ptrs, layout, visitor)? } {
                    return Ok(Some(b));
                }
            },
            None => {
                let Some(frame) = stack.pop() else {
                    return Err(unreachable("The frame stack cannot be empty here"));
                };
                let event = match frame {
                    Frame::Struct { layout, .. } => Event::ExitStruct { layout },
                    Frame::Vector { layout, lens, .. } => Event::ExitVector { layout, lens },
                    Frame::Enum { layout, .. } => Event::ExitEnum { layout },
                };
                if let Step::Break(b) = visitor.visit(event)? {
                    return Ok(Some(b));
                }
            },
        }
    }
}

/// Visits one node and, for an aggregate the visitor descends into, opens its
/// frame. An aggregate the visitor skips still gets a frame, already finished,
/// so that its `Exit*` event is delivered in order.
///
/// # Safety
///
/// Every pointer in `ptrs` must point to a fully initialized, live value of
/// layout `layout`.
unsafe fn enter<'a, L, V, const N: usize>(
    layouts: &'a L,
    stack: &mut Vec<Frame<'a, N>>,
    ptrs: [*const u8; N],
    layout: &'a ValueLayout,
    visitor: &mut V,
) -> VMResult<Option<V::Break>>
where
    L: LayoutProvider + ?Sized,
    V: ValueVisitor<N>,
{
    match &layout.kind {
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address
        | LayoutKind::Signer => match visitor.visit(Event::Scalar { layout, ptrs })? {
            Step::Descend | Step::Skip => Ok(None),
            Step::Break(b) => Ok(Some(b)),
        },
        LayoutKind::Struct { fields } => {
            let step = visitor.visit(Event::EnterStruct {
                layout,
                fields,
                ptrs,
            })?;
            let next = match step {
                Step::Descend => 0,
                Step::Skip => fields.len(),
                Step::Break(b) => return Ok(Some(b)),
            };
            push(stack, Frame::Struct {
                layout,
                fields,
                bases: ptrs,
                next,
            })?;
            Ok(None)
        },
        LayoutKind::Vector { elem_id, .. } => {
            // SAFETY: a vector value is an 8-byte heap pointer to its data,
            // which stores the length ahead of the elements; the pointer is
            // null for a vector without storage, which `read_vec_len` treats
            // as empty.
            let vecs = ptrs.map(|p| unsafe { read_ptr(p, 0usize) });
            let lens = vecs.map(|v| unsafe { read_vec_len(v) });
            let data = vecs.map(|v| {
                if v.is_null() {
                    std::ptr::null()
                } else {
                    // SAFETY: a non-null vector owns a data region at this offset.
                    unsafe { v.add(VEC_DATA_OFFSET) as *const u8 }
                }
            });
            let elem = lookup(layouts, *elem_id)?;
            let step = visitor.visit(Event::EnterVector {
                layout,
                elem_id: *elem_id,
                elem,
                lens,
                data,
            })?;
            let len = lens.iter().copied().min().unwrap_or(0);
            let next = match step {
                Step::Descend => 0,
                Step::Skip => len,
                Step::Break(b) => return Ok(Some(b)),
            };
            push(stack, Frame::Vector {
                layout,
                elem_id: *elem_id,
                elem_size: elem.size as usize,
                data,
                lens,
                len,
                next,
            })?;
            Ok(None)
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            // SAFETY: a well-typed enum value holds a non-null heap pointer to
            // an object storing the tag followed by the variant body.
            let objs = ptrs.map(|p| unsafe { read_ptr(p, 0usize) });
            let tags = objs.map(|o| unsafe { read_enum_tag(o) });
            // Validate every tag before anything else: an out-of-range tag is
            // heap corruption and must fail closed even when the tags differ.
            for tag in tags {
                if tag as usize >= variants.len() {
                    return Err(VMInternalError::new(RuntimeError::InvariantViolation(
                        RuntimeInvariantViolation::EnumTagOutOfRange {
                            tag,
                            variant_count: variants.len(),
                        },
                    )));
                }
            }
            let bodies = objs.map(|o| unsafe { o.add(ENUM_DATA_OFFSET) as *const u8 });
            let same = tags.iter().all(|tag| *tag == tags[0]);
            let body = if same {
                Some(lookup(layouts, variants[tags[0] as usize].id)?)
            } else {
                None
            };
            let step = visitor.visit(Event::EnterEnum {
                layout,
                variants,
                tags,
                body,
                bodies,
            })?;
            let entered = match (step, body) {
                (Step::Descend, Some(_)) => false,
                (Step::Descend, None) | (Step::Skip, _) => true,
                (Step::Break(b), _) => return Ok(Some(b)),
            };
            push(stack, Frame::Enum {
                layout,
                // Unused when `entered` is already true.
                body: body.unwrap_or(layout),
                bodies,
                entered,
            })?;
            Ok(None)
        },
        // TODO(completeness): function values are not yet supported.
        LayoutKind::Function => Err(VMInternalError::new(RuntimeError::Unsupported(
            "function values are not yet supported",
        ))),
        LayoutKind::Ref => Err(unreachable("Value walks run on pointee types only")),
    }
}

/// One open aggregate: what is left to visit under it.
enum Frame<'a, const N: usize> {
    Struct {
        layout: &'a ValueLayout,
        fields: &'a [FieldValueLayout],
        bases: [*const u8; N],
        next: usize,
    },
    Vector {
        layout: &'a ValueLayout,
        elem_id: LayoutId,
        elem_size: usize,
        data: [*const u8; N],
        lens: [u64; N],
        /// Elements to visit: the shortest of `lens`.
        len: u64,
        next: u64,
    },
    Enum {
        layout: &'a ValueLayout,
        body: &'a ValueLayout,
        bodies: [*const u8; N],
        entered: bool,
    },
}

/// The next child of the innermost frame.
enum Child<'a, const N: usize> {
    Field {
        index: usize,
        field: &'a FieldValueLayout,
        ptrs: [*const u8; N],
    },
    Element {
        index: u64,
        ptrs: [*const u8; N],
        id: LayoutId,
    },
    Body {
        ptrs: [*const u8; N],
        layout: &'a ValueLayout,
    },
}

fn push<'a, const N: usize>(stack: &mut Vec<Frame<'a, N>>, frame: Frame<'a, N>) -> VMResult<()> {
    if stack.len() >= MAX_VALUE_DEPTH {
        return Err(VMInternalError::new(RuntimeError::ValueTooDeep {
            max_depth: MAX_VALUE_DEPTH,
        }));
    }
    stack.push(frame);
    Ok(())
}

/// Marks the innermost frame as having no children left to visit.
fn finish_children<const N: usize>(stack: &mut [Frame<'_, N>]) {
    match stack.last_mut() {
        Some(Frame::Struct { fields, next, .. }) => *next = fields.len(),
        Some(Frame::Vector { len, next, .. }) => *next = *len,
        Some(Frame::Enum { entered, .. }) => *entered = true,
        None => {},
    }
}

fn unreachable(msg: &str) -> VMInternalError {
    VMInternalError::new(RuntimeError::InvariantViolation(
        RuntimeInvariantViolation::Unreachable(msg.to_string()),
    ))
}

/// Rejects `N == 0` at compile time.
struct NonEmpty<const N: usize>;

impl<const N: usize> NonEmpty<N> {
    const CHECK: () = assert!(N > 0, "a value walk needs at least one value");
}

#[cfg(test)]
mod tests {
    use super::*;
    use mono_move_core::{
        interner::InternedIdentifier, value_layout::U64_LAYOUT_ID, DescriptorId, LayoutFlags,
        ValueLayoutTable,
    };

    /// What a test visitor saw, reduced to the facts the walk is responsible
    /// for: order, counts, lengths and tags.
    #[derive(Debug, PartialEq, Eq, Clone)]
    enum Seen {
        Scalar(u32),
        EnterStruct(usize),
        Field(usize),
        ExitStruct,
        EnterVector(Vec<u64>),
        Element(u64),
        ExitVector(Vec<u64>),
        EnterEnum(Vec<u64>, bool),
        ExitEnum,
    }

    /// Records every event and answers each with `policy`.
    struct Recorder {
        seen: Vec<Seen>,
        policy: fn(&Seen) -> Step<()>,
    }

    impl Recorder {
        fn new(policy: fn(&Seen) -> Step<()>) -> Self {
            Self {
                seen: Vec::new(),
                policy,
            }
        }
    }

    impl<const N: usize> ValueVisitor<N> for Recorder {
        type Break = ();

        fn visit(&mut self, event: Event<'_, N>) -> VMResult<Step<()>> {
            let seen = match event {
                Event::Scalar { layout, .. } => Seen::Scalar(layout.size),
                Event::EnterStruct { fields, .. } => Seen::EnterStruct(fields.len()),
                Event::Field { index, .. } => Seen::Field(index),
                Event::ExitStruct { .. } => Seen::ExitStruct,
                Event::EnterVector { lens, .. } => Seen::EnterVector(lens.to_vec()),
                Event::Element { index, .. } => Seen::Element(index),
                Event::ExitVector { lens, .. } => Seen::ExitVector(lens.to_vec()),
                Event::EnterEnum { tags, body, .. } => {
                    Seen::EnterEnum(tags.to_vec(), body.is_some())
                },
                Event::ExitEnum { .. } => Seen::ExitEnum,
            };
            let step = (self.policy)(&seen);
            self.seen.push(seen);
            Ok(step)
        }
    }

    fn descend(_: &Seen) -> Step<()> {
        Step::Descend
    }

    fn anonymous(size: u32, kind: LayoutKind) -> ValueLayout {
        ValueLayout::new(None, size, 8, None, LayoutFlags::empty(), kind)
    }

    fn field(offset: u32, id: LayoutId) -> FieldValueLayout {
        FieldValueLayout {
            offset,
            id,
            name: InternedIdentifier::from_static("f"),
        }
    }

    fn vector_of(table: &mut ValueLayoutTable, elem_id: LayoutId) -> LayoutId {
        table.push_anonymous(anonymous(8, LayoutKind::Vector {
            elem_id,
            descriptor_id: DescriptorId(2),
        }))
    }

    fn struct_of(
        table: &mut ValueLayoutTable,
        size: u32,
        fields: Vec<FieldValueLayout>,
    ) -> LayoutId {
        table.push_anonymous(anonymous(size, LayoutKind::Struct {
            fields: fields.into_boxed_slice(),
        }))
    }

    fn enum_of(table: &mut ValueLayoutTable, bodies: Vec<LayoutId>) -> LayoutId {
        let variants = bodies
            .into_iter()
            .map(|id| VariantValueLayout {
                name: InternedIdentifier::from_static("V"),
                id,
            })
            .collect();
        table.push_anonymous(anonymous(8, LayoutKind::FrozenEnum {
            descriptor_id: DescriptorId(3),
            variants,
            max_size_across_variants: 16,
        }))
    }

    fn ptr<T>(x: &T) -> *const u8 {
        x as *const T as *const u8
    }

    /// `S { a: u64, v: vector<u64> }` holding `S { a: 7, v: [1, 2] }`. The
    /// vector object is `[len, elements..]`, as on the heap, minus the header
    /// the walk never reads.
    struct Fixture {
        table: ValueLayoutTable,
        id: LayoutId,
        slot: [u64; 2],
        _vec: Box<[u64; 3]>,
    }

    fn fixture() -> Fixture {
        let mut table = ValueLayoutTable::new();
        let vec_id = vector_of(&mut table, U64_LAYOUT_ID);
        let id = struct_of(&mut table, 16, vec![
            field(0, U64_LAYOUT_ID),
            field(8, vec_id),
        ]);
        let vec = Box::new([2u64, 1, 2]);
        let slot = [7u64, vec.as_ptr() as u64];
        Fixture {
            table,
            id,
            slot,
            _vec: vec,
        }
    }

    fn run<const N: usize>(
        table: &ValueLayoutTable,
        id: LayoutId,
        ptrs: [*const u8; N],
        policy: fn(&Seen) -> Step<()>,
    ) -> (Option<()>, Vec<Seen>) {
        let layout = table.layout(id).unwrap();
        let mut recorder = Recorder::new(policy);
        // SAFETY: the test fixtures hold initialized values of the layouts
        // they are walked with, and outlive the walk.
        let broke = unsafe { walk(table, ptrs, layout, &mut recorder).unwrap() };
        (broke, recorder.seen)
    }

    #[test]
    fn reports_every_node_in_order() {
        let f = fixture();
        let (broke, seen) = run(&f.table, f.id, [ptr(&f.slot)], descend);
        assert_eq!(broke, None);
        assert_eq!(seen, vec![
            Seen::EnterStruct(2),
            Seen::Field(0),
            Seen::Scalar(8),
            Seen::Field(1),
            Seen::EnterVector(vec![2]),
            Seen::Element(0),
            Seen::Scalar(8),
            Seen::Element(1),
            Seen::Scalar(8),
            Seen::ExitVector(vec![2]),
            Seen::ExitStruct,
        ]);
    }

    #[test]
    fn skipping_an_aggregate_still_exits_it() {
        let f = fixture();
        let (_, seen) = run(&f.table, f.id, [ptr(&f.slot)], |seen| match seen {
            Seen::EnterVector(_) => Step::Skip,
            _ => Step::Descend,
        });
        assert_eq!(&seen[4..], &[
            Seen::EnterVector(vec![2]),
            Seen::ExitVector(vec![2]),
            Seen::ExitStruct,
        ]);
    }

    #[test]
    fn skipping_a_child_skips_its_later_siblings() {
        let f = fixture();
        let (_, seen) = run(&f.table, f.id, [ptr(&f.slot)], |seen| match seen {
            Seen::Element(0) => Step::Skip,
            _ => Step::Descend,
        });
        assert_eq!(&seen[4..], &[
            Seen::EnterVector(vec![2]),
            Seen::Element(0),
            Seen::ExitVector(vec![2]),
            Seen::ExitStruct,
        ]);
    }

    #[test]
    fn breaking_ends_the_walk() {
        let f = fixture();
        let (broke, seen) = run(&f.table, f.id, [ptr(&f.slot)], |seen| match seen {
            Seen::Scalar(_) => Step::Break(()),
            _ => Step::Descend,
        });
        assert_eq!(broke, Some(()));
        assert_eq!(seen, vec![
            Seen::EnterStruct(2),
            Seen::Field(0),
            Seen::Scalar(8)
        ]);
    }

    #[test]
    fn lockstep_visits_the_common_prefix() {
        let mut table = ValueLayoutTable::new();
        let id = vector_of(&mut table, U64_LAYOUT_ID);
        let a = [3u64, 10, 20, 30];
        let b = [2u64, 10, 20];
        let (slot_a, slot_b) = (a.as_ptr() as u64, b.as_ptr() as u64);
        let (_, seen) = run(&table, id, [ptr(&slot_a), ptr(&slot_b)], descend);
        assert_eq!(seen, vec![
            Seen::EnterVector(vec![3, 2]),
            Seen::Element(0),
            Seen::Scalar(8),
            Seen::Element(1),
            Seen::Scalar(8),
            Seen::ExitVector(vec![3, 2]),
        ]);
    }

    #[test]
    fn empty_vectors_have_no_storage() {
        let mut table = ValueLayoutTable::new();
        let id = vector_of(&mut table, U64_LAYOUT_ID);
        let slot = 0u64;
        let (_, seen) = run(&table, id, [ptr(&slot)], descend);
        assert_eq!(seen, vec![
            Seen::EnterVector(vec![0]),
            Seen::ExitVector(vec![0])
        ]);
    }

    /// `enum E { A, B(u64) }`: two enum objects holding `[tag, payload]`.
    fn enum_fixture() -> (ValueLayoutTable, LayoutId) {
        let mut table = ValueLayoutTable::new();
        let unit = struct_of(&mut table, 0, vec![]);
        let tuple = struct_of(&mut table, 8, vec![field(0, U64_LAYOUT_ID)]);
        let id = enum_of(&mut table, vec![unit, tuple]);
        (table, id)
    }

    #[test]
    fn differing_tags_do_not_descend() {
        let (table, id) = enum_fixture();
        let a = [0u64, 0];
        let b = [1u64, 9];
        let (slot_a, slot_b) = (a.as_ptr() as u64, b.as_ptr() as u64);
        let (_, seen) = run(&table, id, [ptr(&slot_a), ptr(&slot_b)], descend);
        assert_eq!(seen, vec![
            Seen::EnterEnum(vec![0, 1], false),
            Seen::ExitEnum
        ]);
    }

    #[test]
    fn matching_tags_descend_into_the_body() {
        let (table, id) = enum_fixture();
        let a = [1u64, 5];
        let b = [1u64, 9];
        let (slot_a, slot_b) = (a.as_ptr() as u64, b.as_ptr() as u64);
        let (_, seen) = run(&table, id, [ptr(&slot_a), ptr(&slot_b)], descend);
        assert_eq!(seen, vec![
            Seen::EnterEnum(vec![1, 1], true),
            Seen::EnterStruct(1),
            Seen::Field(0),
            Seen::Scalar(8),
            Seen::ExitStruct,
            Seen::ExitEnum,
        ]);
    }

    #[test]
    fn out_of_range_tag_fails_closed() {
        let (table, id) = enum_fixture();
        let obj = [5u64, 0];
        let slot = obj.as_ptr() as u64;
        let layout = table.layout(id).unwrap();
        let mut recorder = Recorder::new(descend);
        // SAFETY: the slot holds a pointer to an initialized enum object.
        let err = unsafe { walk(&table, [ptr(&slot)], layout, &mut recorder).unwrap_err() };
        assert!(matches!(
            err.downcast_ref::<RuntimeError>(),
            Some(RuntimeError::InvariantViolation(
                RuntimeInvariantViolation::EnumTagOutOfRange {
                    tag: 5,
                    variant_count: 2
                }
            ))
        ));
        assert!(recorder.seen.is_empty());
    }

    /// `depth` nested single-element vectors around one `u64`.
    struct Chain {
        table: ValueLayoutTable,
        id: LayoutId,
        slot: u64,
        /// Sized up front and never grown, so the levels keep their addresses.
        _levels: Vec<[u64; 2]>,
    }

    fn chain(depth: usize) -> Chain {
        let mut table = ValueLayoutTable::new();
        let mut id = U64_LAYOUT_ID;
        let mut levels: Vec<[u64; 2]> = Vec::with_capacity(depth);
        let mut inner = 42u64;
        for _ in 0..depth {
            id = vector_of(&mut table, id);
            levels.push([1u64, inner]);
            inner = levels.last().unwrap().as_ptr() as u64;
        }
        Chain {
            table,
            id,
            slot: inner,
            _levels: levels,
        }
    }

    #[test]
    fn nesting_is_bounded() {
        let ok = chain(MAX_VALUE_DEPTH);
        let (broke, seen) = run(&ok.table, ok.id, [ptr(&ok.slot)], descend);
        assert_eq!(broke, None);
        assert_eq!(seen.len(), 3 * MAX_VALUE_DEPTH + 1);

        let too_deep = chain(MAX_VALUE_DEPTH + 1);
        let layout = too_deep.table.layout(too_deep.id).unwrap();
        let mut recorder = Recorder::new(descend);
        // SAFETY: every level holds a pointer to the initialized level below.
        let err = unsafe {
            walk(
                &too_deep.table,
                [ptr(&too_deep.slot)],
                layout,
                &mut recorder,
            )
            .unwrap_err()
        };
        assert!(matches!(
            err.downcast_ref::<RuntimeError>(),
            Some(RuntimeError::ValueTooDeep {
                max_depth: MAX_VALUE_DEPTH
            })
        ));
    }
}
