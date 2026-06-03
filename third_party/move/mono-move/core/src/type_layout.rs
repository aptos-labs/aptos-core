// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type layouts: a flat, ID-indexed description of a value's shape used to
//! drive layout-sensitive value walks (BCS size queries, BCS serialization,
//! BCS deserialization into the flat in-memory representation, as well as
//! comparison and equality).
//!
//! A [`TypeLayout`] is resolved once per concrete type at lowering time and
//! then walked by chasing [`LayoutId`]s, so there is no need for the walk to
//! re-interpret the [`Type`] DAG. Layouts are built in the same pass that
//! publishes GC [`ObjectDescriptor`](crate::ObjectDescriptor)s. The two tables
//! stay separate: descriptors are shallow, must-be-present-now, and live for
//! the value's lifetime (GC reads them out of object headers), while layouts
//! are deep and built on demand. They meet at one point: a [`LayoutKind::Vector`]
//! carries the [`DescriptorId`] so that deserialization can add it to the value
//! header.
//! TODO: We will need descriptor IDs for non-inline structs and enums later.

use crate::{
    types::{view_type, InternedType, InternedTypeList, Type},
    DescriptorId,
};
use bitflags::bitflags;
use std::collections::HashMap;

/// Typed index into the program's [`TypeLayout`] table.
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
    }
}

/// Layout description of a value.
pub struct TypeLayout {
    /// In-memory size in bytes.
    pub size: u32,
    /// In-memory alignment.
    pub align: u32,
    /// Fixed BCS size in bytes, or [`None`] when data-dependent (e.g., for
    /// vectors, enums, function values and anything that transitively owns
    /// them).
    pub const_bcs_size: Option<u32>,
    /// Flags with extra information about this layout / type.
    pub flags: LayoutFlags,
    /// Describes layout's shape.
    pub kind: LayoutKind,
}

/// Layout information for a struct field.
pub struct FieldTypeLayout {
    pub offset: u32,
    pub id: LayoutId,
}

/// Shape-specific layout data.
pub enum LayoutKind {
    /// An unsigned integer (`u8`, ...,`u256`) or boolean.
    UnsignedInt,
    /// A signed integer (`i8`, ..., `i256`).
    SignedInt,
    /// An `address` or `signer` (32 bytes).
    Address,
    /// An inline struct: fields laid out flat in the parent's payload.
    /// TODO: for non-inline structs, we need a descriptor ID.
    Struct {
        /// Byte offsets and IDs of each field within the struct payload.
        fields: Box<[FieldTypeLayout]>,
    },
    /// A vector: an 8-byte heap-pointer slot. `elem_id` is the element layout
    /// (for the per-element walk); `descriptor_id` is the GC descriptor of the
    /// heap box (for stamping the header on deserialization).
    Vector {
        elem_id: LayoutId,
        descriptor_id: DescriptorId,
    },
    /// An enum whose variants are resolved lazily on every walk (upgradable).
    /// TODO: not yet implemented; the walks error on this kind.
    /// TODO: add closed enum (for framework, frozen ones)
    OpenEnum {
        ty: InternedType,
        ty_args: InternedTypeList,
    },
    /// A reference (16-byte fat pointer). All references share this layout.
    Ref,
    /// A function/closure value (8-byte heap-pointer slot). All function
    /// values share this layout.
    Function,
}

impl TypeLayout {
    /// Builds a layout from its parts. The flag computation lives in the
    /// builder (it needs child layouts), so this is a plain constructor.
    pub fn new(
        size: u32,
        align: u32,
        const_bcs_size: Option<u32>,
        flags: LayoutFlags,
        kind: LayoutKind,
    ) -> Self {
        Self {
            size,
            align,
            const_bcs_size,
            flags,
            kind,
        }
    }

    /// Returns true if a value of this layout has no padding and no pointers.
    pub fn has_no_pointers_no_padding(&self) -> bool {
        self.flags.contains(LayoutFlags::NO_POINTERS_NO_PADDING)
    }

    /// The fixed BCS size of this type, or [`None`] when data-dependent.
    pub fn const_serialized_size(&self) -> Option<u32> {
        self.const_bcs_size
    }

    /// Layout of `bool` (ordered as a 1-byte unsigned integer).
    pub fn bool() -> Self {
        Self::unsigned_int(1, 1)
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
        Self::unsigned_int(16, 8)
    }

    /// Layout of `u256`.
    pub fn u256() -> Self {
        Self::unsigned_int(32, 8)
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
        Self::signed_int(16, 8)
    }

    /// Layout of `i256`.
    pub fn i256() -> Self {
        Self::signed_int(32, 8)
    }

    /// Layout of `address` or a signer.
    pub fn address() -> Self {
        Self {
            size: 32,
            align: 8,
            const_bcs_size: Some(32),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING,
            kind: LayoutKind::Address,
        }
    }

    /// Layout shared by all reference types (16-byte fat pointer).
    pub fn reference() -> Self {
        Self {
            size: 16,
            align: 8,
            const_bcs_size: None,
            flags: LayoutFlags::empty(),
            kind: LayoutKind::Ref,
        }
    }

    /// Layout shared by all function values (heap-pointer slot).
    pub fn function() -> Self {
        Self {
            size: 8,
            align: 8,
            const_bcs_size: None,
            flags: LayoutFlags::empty(),
            kind: LayoutKind::Function,
        }
    }

    /// Layout for vectors (heap pointer slot).
    pub fn vector(elem_id: LayoutId, descriptor_id: DescriptorId) -> Self {
        Self {
            size: 8,
            align: 8,
            const_bcs_size: None,
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
        const_bcs_size: Option<u32>,
        flags: LayoutFlags,
        fields: Box<[FieldTypeLayout]>,
    ) -> TypeLayout {
        Self {
            size,
            align,
            const_bcs_size,
            flags,
            kind: LayoutKind::Struct { fields },
        }
    }

    /// Layout for an open enum.
    pub fn open_enum(ty: InternedType, ty_args: InternedTypeList) -> TypeLayout {
        Self {
            size: 8,
            align: 8,
            const_bcs_size: None,
            flags: LayoutFlags::empty(),
            kind: LayoutKind::OpenEnum { ty, ty_args },
        }
    }

    fn unsigned_int(size: u32, align: u32) -> Self {
        Self {
            size,
            align,
            const_bcs_size: Some(size),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING,
            kind: LayoutKind::UnsignedInt,
        }
    }

    fn signed_int(size: u32, align: u32) -> Self {
        Self {
            size,
            align,
            const_bcs_size: Some(size),
            flags: LayoutFlags::NO_POINTERS_NO_PADDING,
            kind: LayoutKind::SignedInt,
        }
    }
}

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
pub fn reserved_layouts() -> Vec<TypeLayout> {
    // Order MUST match the `*_LAYOUT_ID` constants above.
    vec![
        TypeLayout::bool(),
        TypeLayout::u8(),
        TypeLayout::u16(),
        TypeLayout::u32(),
        TypeLayout::u64(),
        TypeLayout::u128(),
        TypeLayout::u256(),
        TypeLayout::i8(),
        TypeLayout::i16(),
        TypeLayout::i32(),
        TypeLayout::i64(),
        TypeLayout::i128(),
        TypeLayout::i256(),
        TypeLayout::address(),
        TypeLayout::reference(),
        TypeLayout::function(),
    ]
}

/// Per-ID and per-type lookup of [`TypeLayout`]s.
pub trait LayoutProvider {
    /// Returns the layout for `id`, or [`None`] if `id` is unknown.
    fn layout(&self, id: LayoutId) -> Option<&TypeLayout>;

    /// Returns the layout id for `ty`, or [`None`] if no layout has been
    /// published for it yet (e.g. its module is not loaded).
    fn layout_id(&self, ty: InternedType) -> Option<LayoutId>;

    fn layout_by_ty(&self, ty: InternedType) -> Option<&TypeLayout> {
        let id = self.layout_id(ty)?;
        self.layout(id)
    }
}

// TODO: Test-only, remove when local execution context is refactored and removed.
pub struct TypeLayoutTable {
    table: Vec<TypeLayout>,
    by_ty: HashMap<InternedType, LayoutId>,
}

impl TypeLayoutTable {
    pub fn new() -> Self {
        Self {
            table: reserved_layouts(),
            by_ty: HashMap::new(),
        }
    }

    pub fn push(&mut self, ty: InternedType, layout: TypeLayout) -> LayoutId {
        let id = LayoutId::from_usize(self.table.len());
        self.table.push(layout);
        self.by_ty.insert(ty, id);
        id
    }
}

impl Default for TypeLayoutTable {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutProvider for TypeLayoutTable {
    fn layout(&self, id: LayoutId) -> Option<&TypeLayout> {
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
            assert_eq!(l.const_bcs_size, Some(width));
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
        assert_eq!(r.const_bcs_size, None);
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
