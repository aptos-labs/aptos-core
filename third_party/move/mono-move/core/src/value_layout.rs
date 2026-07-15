// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type layouts: a flat, ID-indexed description of a value's shape used to
//! drive layout-sensitive value walks (e.g., BCS size queries, BCS
//! serialization, BCS deserialization into the flat in-memory representation,
//! comparison, and equality).
//!
//! A [`ValueLayout`] is resolved once per concrete type at lowering time and
//! then walked by chasing [`LayoutId`]s, so there is no need for the walk to
//! re-interpret the [`Type`] DAG. Layouts are built in the same pass that
//! publishes GC [`ObjectDescriptor`](crate::ObjectDescriptor)s. The two tables
//! stay separate: descriptors are shallow, must-be-present-now, and live for
//! the value's lifetime (GC reads them out of object headers), while layouts
//! are deep and built on demand. They meet at one point: a [`LayoutKind::Vector`]
//! carries the [`DescriptorId`] so that deserialization can add it to the value
//! header.

use crate::{
    types::{intrinsic_slot_size_and_align, view_type, Alignment, InternedType, Size, Type},
    DescriptorId, MAX_ALIGN,
};
use bitflags::bitflags;
use std::collections::HashMap;

/// Typed index into the program's [`ValueLayout`] table.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LayoutId(u32);

impl LayoutId {
    /// Builds a layout ID from a table index.
    ///
    /// # Invariant
    ///
    /// - Index must always fit into `u32`.
    #[inline(always)]
    pub const fn from_usize(idx: usize) -> Self {
        debug_assert!(idx <= u32::MAX as usize);
        Self(idx as u32)
    }

    /// Returns the underlying index as `usize`.
    #[inline(always)]
    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

bitflags! {
    /// Layout flags encoding information about value layouts.
    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    pub struct LayoutFlags: u8 {
        /// This value has no heap pointers and no padding. That is, it can be
        /// serialized in one `memcpy` and compared for equality with `memcmp`.
        const NO_POINTERS_NO_PADDING = 0b0000_0001;
        /// Every byte pattern of this value's in-memory image is a valid value,
        /// so deserialization needs no per-byte validation and can copy the BCS
        /// bytes in one `memcpy`. Holds for integers and addresses but not for
        /// `bool` (only `0`/`1` are canonical). An aggregate has this flag only
        /// when it has no padding, no pointers, and no `bool` reachable.
        const ALL_BYTE_PATTERNS_VALID = 0b0000_0010;
    }
}

/// Layout description of a value.
pub struct ValueLayout {
    /// In-memory size in bytes.
    pub size: u32,
    /// In-memory alignment.
    pub align: u32,
    /// Fixed BCS size in bytes, or [`None`] when data-dependent (e.g., for
    /// vectors, enums, function values and anything that transitively owns
    /// them).
    pub fixed_bcs_size: Option<u32>,
    /// Flags with extra information about this layout / type.
    pub flags: LayoutFlags,
    /// Describes layout's shape.
    pub kind: LayoutKind,
}

/// Layout information for a struct field.
pub struct FieldValueLayout {
    pub offset: u32,
    pub id: LayoutId,
}

/// Shape-specific layout data.
pub enum LayoutKind {
    /// A boolean: a 1-byte value holding `0` or `1`.
    Bool,
    /// An unsigned integer (`u8`, ..., `u256`).
    UnsignedInt,
    /// A signed integer (`i8`, ..., `i256`).
    SignedInt,
    /// An `address` or `signer` (32 bytes).
    Address,
    /// An inline struct: fields laid out flat in the parent's payload.
    /// TODO(completeness): for non-inline structs (resources), we need a descriptor ID.
    Struct {
        /// Byte offsets and IDs of each field within the struct payload.
        fields: Box<[FieldValueLayout]>,
    },
    /// A vector: an 8-byte heap-pointer slot. `elem_id` is the element layout
    /// (for the per-element walk); `descriptor_id` is the GC descriptor of the
    /// heap box (for stamping the header on deserialization).
    Vector {
        elem_id: LayoutId,
        descriptor_id: DescriptorId,
    },
    /// An enum whose variant layouts are fixed: an 8-byte heap-pointer slot
    /// pointing at an enum object. Each variant body has its own published
    /// layout, which may itself contain enums, vectors, or structs.
    ///
    /// TODO(completeness): revisit with upgrade story, might not need to be frozen.
    FrozenEnum {
        descriptor_id: DescriptorId,
        /// One layout per variant body, indexed by variant tag.
        variants: Box<[LayoutId]>,
        /// Size of the enum object's data region: the 8-byte tag plus the
        /// widest variant body, rounded up to 8-byte alignment. Sized to the
        /// largest variant so any variant fits.
        max_size_across_variants: u32,
    },
    /// A reference (16-byte fat pointer). All references share this layout.
    Ref,
    /// A function/closure value (8-byte heap-pointer slot). All function
    /// values share this layout.
    Function,
}

impl ValueLayout {
    /// Builds a layout from its parts. The flag computation lives in the
    /// builder (it needs child layouts), so this is a plain constructor.
    pub fn new(
        size: u32,
        align: u32,
        fixed_bcs_size: Option<u32>,
        flags: LayoutFlags,
        kind: LayoutKind,
    ) -> Self {
        debug_assert!(
            align <= MAX_ALIGN as u32,
            "value alignment must not exceed MAX_ALIGN"
        );
        Self {
            size,
            align,
            fixed_bcs_size,
            flags,
            kind,
        }
    }

    /// Returns true if a value of this layout has no padding and no pointers.
    pub fn has_no_pointers_no_padding(&self) -> bool {
        self.flags.contains(LayoutFlags::NO_POINTERS_NO_PADDING)
    }

    /// Returns true if every byte pattern of this value's in-memory size is a
    /// valid value, so deserialization can blit the BCS bytes without per-byte
    /// validation. False for anything that reaches a `bool`.
    pub fn all_byte_patterns_valid(&self) -> bool {
        self.flags.contains(LayoutFlags::ALL_BYTE_PATTERNS_VALID)
    }

    /// The fixed BCS size of a value of this type, or [`None`] when
    /// data-dependent.
    pub fn fixed_serialized_size(&self) -> Option<u32> {
        self.fixed_bcs_size
    }

    /// Layout of `bool`: a 1-byte `0`/`1` value.
    pub fn bool() -> Self {
        Self {
            size: 1,
            align: 1,
            fixed_bcs_size: Some(1),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING,
            kind: LayoutKind::Bool,
        }
    }

    /// Layout of `u8`.
    pub fn u8() -> Self {
        Self::unsigned_int(1, 1)
    }

    /// Layout of `u16`.
    pub fn u16() -> Self {
        Self::unsigned_int(2, 2)
    }

    /// Layout of `u32`.
    pub fn u32() -> Self {
        Self::unsigned_int(4, 4)
    }

    /// Layout of `u64`.
    pub fn u64() -> Self {
        Self::unsigned_int(8, 8)
    }

    /// Layout of `u128`.
    pub fn u128() -> Self {
        Self::unsigned_int(16, MAX_ALIGN as u32)
    }

    /// Layout of `u256`.
    pub fn u256() -> Self {
        Self::unsigned_int(32, MAX_ALIGN as u32)
    }

    /// Layout of `i8`.
    pub fn i8() -> Self {
        Self::signed_int(1, 1)
    }

    /// Layout of `i16`.
    pub fn i16() -> Self {
        Self::signed_int(2, 2)
    }

    /// Layout of `i32`.
    pub fn i32() -> Self {
        Self::signed_int(4, 4)
    }

    /// Layout of `i64`.
    pub fn i64() -> Self {
        Self::signed_int(8, 8)
    }

    /// Layout of `i128`.
    pub fn i128() -> Self {
        Self::signed_int(16, MAX_ALIGN as u32)
    }

    /// Layout of `i256`.
    pub fn i256() -> Self {
        Self::signed_int(32, MAX_ALIGN as u32)
    }

    /// Layout of `address` or a signer.
    pub fn address() -> Self {
        Self {
            size: 32,
            align: MAX_ALIGN as u32,
            fixed_bcs_size: Some(32),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING | LayoutFlags::ALL_BYTE_PATTERNS_VALID,
            kind: LayoutKind::Address,
        }
    }

    /// Layout shared by all reference types (16-byte fat pointer).
    pub fn reference() -> Self {
        Self {
            size: 16,
            align: MAX_ALIGN as u32,
            fixed_bcs_size: None,
            flags: LayoutFlags::empty(),
            kind: LayoutKind::Ref,
        }
    }

    /// Layout shared by all function values (heap-pointer slot).
    pub fn function() -> Self {
        Self {
            size: 8,
            align: MAX_ALIGN as u32,
            fixed_bcs_size: None,
            flags: LayoutFlags::empty(),
            kind: LayoutKind::Function,
        }
    }

    /// Layout for vectors (heap pointer slot).
    pub fn vector(elem_id: LayoutId, descriptor_id: DescriptorId) -> Self {
        Self {
            size: 8,
            align: MAX_ALIGN as u32,
            fixed_bcs_size: None,
            // Vectors are heap pointers, so their data can be bulk copied or
            // compared only if its element has no pointers or no padding, but
            // otherwise vectors do not qualify.
            flags: LayoutFlags::empty(),
            kind: LayoutKind::Vector {
                elem_id,
                descriptor_id,
            },
        }
    }

    /// Layout for a struct.
    pub fn struct_layout(
        size: u32,
        align: u32,
        fixed_bcs_size: Option<u32>,
        flags: LayoutFlags,
        fields: Box<[FieldValueLayout]>,
    ) -> ValueLayout {
        debug_assert!(
            align <= MAX_ALIGN as u32,
            "value alignment must not exceed MAX_ALIGN"
        );
        Self {
            size,
            align,
            fixed_bcs_size,
            flags,
            kind: LayoutKind::Struct { fields },
        }
    }

    /// Layout for a frozen enum.
    pub fn frozen_enum(
        descriptor_id: DescriptorId,
        variants: Box<[LayoutId]>,
        max_size_across_variants: u32,
    ) -> ValueLayout {
        Self {
            size: 8,
            align: MAX_ALIGN as u32,
            fixed_bcs_size: None,
            flags: LayoutFlags::empty(),
            kind: LayoutKind::FrozenEnum {
                descriptor_id,
                variants,
                max_size_across_variants,
            },
        }
    }

    fn unsigned_int(size: u32, align: u32) -> Self {
        Self {
            size,
            align,
            fixed_bcs_size: Some(size),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING | LayoutFlags::ALL_BYTE_PATTERNS_VALID,
            kind: LayoutKind::UnsignedInt,
        }
    }

    fn signed_int(size: u32, align: u32) -> Self {
        Self {
            size,
            align,
            fixed_bcs_size: Some(size),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING | LayoutFlags::ALL_BYTE_PATTERNS_VALID,
            kind: LayoutKind::SignedInt,
        }
    }
}

// Reserved layout IDs for types whose layout does not depend on type
// arguments (primitives, references, functions). They occupy the first slots
// of every layout table, in exactly this order.
//
// # Invariants
//
// - The numeric IDs below must match the order of [`reserved_layouts`], which
//   seeds the table. Keep the two in sync.
// - Never reorder or renumber an existing entry, and never insert in the
//   middle: a layout table built at an earlier version would then resolve old
//   IDs to the wrong layout. Only append new reserved IDs at the end.

/// Reserved layout ID for [`Type::Bool`].
pub const BOOL_LAYOUT_ID: LayoutId = LayoutId(0);

/// Reserved layout ID for [`Type::U8`].
pub const U8_LAYOUT_ID: LayoutId = LayoutId(1);

/// Reserved layout ID for [`Type::U16`].
pub const U16_LAYOUT_ID: LayoutId = LayoutId(2);

/// Reserved layout ID for [`Type::U32`].
pub const U32_LAYOUT_ID: LayoutId = LayoutId(3);

/// Reserved layout ID for [`Type::U64`].
pub const U64_LAYOUT_ID: LayoutId = LayoutId(4);

/// Reserved layout ID for [`Type::U128`].
pub const U128_LAYOUT_ID: LayoutId = LayoutId(5);

/// Reserved layout ID for [`Type::U256`].
pub const U256_LAYOUT_ID: LayoutId = LayoutId(6);

/// Reserved layout ID for [`Type::I8`].
pub const I8_LAYOUT_ID: LayoutId = LayoutId(7);

/// Reserved layout ID for [`Type::I16`].
pub const I16_LAYOUT_ID: LayoutId = LayoutId(8);

/// Reserved layout ID for [`Type::I32`].
pub const I32_LAYOUT_ID: LayoutId = LayoutId(9);

/// Reserved layout ID for [`Type::I64`].
pub const I64_LAYOUT_ID: LayoutId = LayoutId(10);

/// Reserved layout ID for [`Type::I128`].
pub const I128_LAYOUT_ID: LayoutId = LayoutId(11);

/// Reserved layout ID for [`Type::I256`].
pub const I256_LAYOUT_ID: LayoutId = LayoutId(12);

/// Reserved layout ID for [`Type::Address`]. Note that  [`Type::Signer`]
/// has the same layout.
pub const ADDRESS_LAYOUT_ID: LayoutId = LayoutId(13);

/// Reserved layout ID shared by all reference types.
pub const REF_LAYOUT_ID: LayoutId = LayoutId(14);

/// Reserved layout ID shared by all function values.
pub const FUNCTION_LAYOUT_ID: LayoutId = LayoutId(15);

/// Returns the reserved [`LayoutId`] for a type whose layout is arg-independent
/// (primitives, references, functions), or [`None`] for types that must be
/// built and looked up in the table (vectors, nominals) or are unsized (type
/// parameters).
pub fn reserved_layout_id(ty: &Type) -> Option<LayoutId> {
    Some(match ty {
        Type::Bool => BOOL_LAYOUT_ID,
        Type::U8 => U8_LAYOUT_ID,
        Type::U16 => U16_LAYOUT_ID,
        Type::U32 => U32_LAYOUT_ID,
        Type::U64 => U64_LAYOUT_ID,
        Type::U128 => U128_LAYOUT_ID,
        Type::U256 => U256_LAYOUT_ID,
        Type::I8 => I8_LAYOUT_ID,
        Type::I16 => I16_LAYOUT_ID,
        Type::I32 => I32_LAYOUT_ID,
        Type::I64 => I64_LAYOUT_ID,
        Type::I128 => I128_LAYOUT_ID,
        Type::I256 => I256_LAYOUT_ID,
        // Signer shares the address layout.
        Type::Address | Type::Signer => ADDRESS_LAYOUT_ID,
        Type::ImmutRef { .. } | Type::MutRef { .. } => REF_LAYOUT_ID,
        Type::Function { .. } => FUNCTION_LAYOUT_ID,
        Type::Vector { .. } | Type::Nominal { .. } | Type::TypeParam { .. } => return None,
    })
}

/// Returns the initial layout table: the reserved entries in ID order.
pub fn reserved_layouts() -> Vec<ValueLayout> {
    // Order MUST match the `*_LAYOUT_ID` constants above.
    vec![
        ValueLayout::bool(),
        ValueLayout::u8(),
        ValueLayout::u16(),
        ValueLayout::u32(),
        ValueLayout::u64(),
        ValueLayout::u128(),
        ValueLayout::u256(),
        ValueLayout::i8(),
        ValueLayout::i16(),
        ValueLayout::i32(),
        ValueLayout::i64(),
        ValueLayout::i128(),
        ValueLayout::i256(),
        ValueLayout::address(),
        ValueLayout::reference(),
        ValueLayout::function(),
    ]
}

/// Per-ID and per-type lookup of [`ValueLayout`]s.
pub trait LayoutProvider {
    /// Returns the layout for `id`, or [`None`] if `id` is unknown.
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout>;

    /// Returns the layout id for `ty`, or [`None`] if no layout has been
    /// published for it yet (e.g. its module is not loaded).
    fn layout_id(&self, ty: InternedType) -> Option<LayoutId>;

    fn layout_by_ty(&self, ty: InternedType) -> Option<&ValueLayout> {
        let id = self.layout_id(ty)?;
        self.layout(id)
    }

    /// In-memory size and alignment of a value of type `ty`. Intrinsic shapes
    /// (primitives, references, vectors, functions) resolve without the table;
    /// nominal types resolve through their published [`ValueLayout`]. Returns
    /// [`None`] for unresolved type parameters and for nominals whose layout
    /// has not been published yet.
    fn size_and_align(&self, ty: InternedType) -> Option<(Size, Alignment)> {
        intrinsic_slot_size_and_align(view_type(ty))
            .or_else(|| self.layout_by_ty(ty).map(|l| (l.size, l.align)))
    }
}

/// In-memory layout table used by unit tests as a lightweight
/// [`LayoutProvider`].
pub struct ValueLayoutTable {
    table: Vec<ValueLayout>,
    by_ty: HashMap<InternedType, LayoutId>,
}

impl ValueLayoutTable {
    pub fn new() -> Self {
        Self {
            table: reserved_layouts(),
            by_ty: HashMap::new(),
        }
    }

    pub fn push(&mut self, ty: InternedType, layout: ValueLayout) -> LayoutId {
        let id = LayoutId::from_usize(self.table.len());
        self.table.push(layout);
        self.by_ty.insert(ty, id);
        id
    }
}

impl Default for ValueLayoutTable {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutProvider for ValueLayoutTable {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.table.get(id.as_usize())
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        if let Some(id) = reserved_layout_id(view_type(ty)) {
            return Some(id);
        }
        self.by_ty.get(&ty).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_layouts_match_ids() {
        let layouts = reserved_layouts();
        assert_eq!(layouts.len(), 16);

        let cases = [
            (BOOL_LAYOUT_ID, 1),
            (U8_LAYOUT_ID, 1),
            (U16_LAYOUT_ID, 2),
            (U32_LAYOUT_ID, 4),
            (U64_LAYOUT_ID, 8),
            (U128_LAYOUT_ID, 16),
            (U256_LAYOUT_ID, 32),
            (I8_LAYOUT_ID, 1),
            (I256_LAYOUT_ID, 32),
            (ADDRESS_LAYOUT_ID, 32),
        ];
        for (id, width) in cases {
            let l = &layouts[id.as_usize()];
            assert_eq!(l.size, width);
            assert_eq!(l.fixed_bcs_size, Some(width));
            assert!(l.has_no_pointers_no_padding());
        }

        assert!(matches!(
            layouts[U64_LAYOUT_ID.as_usize()].kind,
            LayoutKind::UnsignedInt
        ));
        assert!(matches!(
            layouts[I8_LAYOUT_ID.as_usize()].kind,
            LayoutKind::SignedInt
        ));
        assert!(matches!(
            layouts[ADDRESS_LAYOUT_ID.as_usize()].kind,
            LayoutKind::Address
        ));

        let r = &layouts[REF_LAYOUT_ID.as_usize()];
        assert_eq!(r.size, 16);
        assert_eq!(r.fixed_bcs_size, None);
        assert!(matches!(r.kind, LayoutKind::Ref));

        let f = &layouts[FUNCTION_LAYOUT_ID.as_usize()];
        assert_eq!(f.size, 8);
        assert!(matches!(f.kind, LayoutKind::Function));
    }

    #[test]
    fn reserved_layout_id_maps_primitives_and_pointers() {
        assert_eq!(reserved_layout_id(&Type::U64), Some(U64_LAYOUT_ID));
        assert_eq!(reserved_layout_id(&Type::Address), Some(ADDRESS_LAYOUT_ID));
        assert_eq!(reserved_layout_id(&Type::Signer), Some(ADDRESS_LAYOUT_ID));
        assert_eq!(
            reserved_layout_id(&Type::ImmutRef {
                inner: crate::types::U64_TY
            }),
            Some(REF_LAYOUT_ID)
        );
        assert_eq!(
            reserved_layout_id(&Type::Vector {
                elem: crate::types::U64_TY
            }),
            None
        );
        assert_eq!(reserved_layout_id(&Type::TypeParam { idx: 0 }), None);
    }
}
