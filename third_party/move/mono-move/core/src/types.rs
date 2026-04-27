// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Runtime type model and layout metadata.
//!
//! A single **type graph** lives in the global arena: a DAG of [`Type`] nodes,
//! deduplicated by interning so that pointer equality implies structural
//! equality. Composite types (vectors, references, etc.) reference their
//! children via [`GlobalArenaPtr`].
//!
//! ## Primitives
//!
//! Primitives (boolean, integer types, etc.) are pre-allocated as statics. No
//! arena allocation needed. Layout, size and alignment can be deduced from the
//! type.
//!
//! ## Type parameters
//!
//! Type parameters are interned and allocated in arena as [`GlobalArenaPtr`].
//! During type substitution, pointers are replaced, and the whole type is re-
//! canonicalized.
//! TODO: This is currently not supported.
//!
//! ## Vectors
//!
//! Vectors are arena-allocated composite types with their inner
//! types interned recursively.
//!
//! In flat memory, vectors have 8-byte size and 8-byte alignment.
//!
//! ## References
//!
//! References are arena-allocated composite types with their inner
//! pointee types interned recursively.
//!
//! Size of references is 16 bytes (fat pointers). Alignment is also 16 bytes.
//!
//! ## Fully-instantiated structs
//!
//! Struct types are arena-allocated, and store executable ID, name and type
//! arguments that uniquely identify the type. Additionally, fully-instantiated
//! structs cache their layout information (size, alignment and field offsets).
//!
//! ## Enums
//!
//! Enums are simply pointers to arena-allocated executable IDs, identifiers
//! and type arguments that uniquely identify the type. Enum layouts are not
//! cached in the type graph because enum definitions can change on module
//! upgrade (new variants added). Instead, variant field layouts are stored
//! per-executable and resolved at runtime.
//!
//! ## Generic structs
//!
//! TODO: support substitution

use crate::{ExecutableId, Interner};
use mono_move_alloc::GlobalArenaPtr;
use move_core_types::ability::AbilitySet;

// ================================================================================================
// Layout types
// ================================================================================================

/// Total size of the type in flat memory including padding and any alignment.
pub type Size = u32;

/// When [`Type`] is stored in flat memory, the start address needs to be
/// this many bytes aligned.
pub type Alignment = u32;

/// Offset in bytes of struct fields in flat memory.
pub type FieldOffset = u32;

/// Pointer to an arena-interned [`Type`]. Pointer equality implies structural
/// equality because the global interner deduplicates types. The alias hides
/// the raw `GlobalArenaPtr<Type>` form throughout the codebase.
pub type InternedType = GlobalArenaPtr<Type>;

/// Pointer to an arena-interned list of [`InternedType`]s (e.g., function
/// parameter/return types, generic type arguments). The list itself is also
/// interned and deduplicated.
pub type InternedTypeList = GlobalArenaPtr<[InternedType]>;

// ================================================================================================
// View helpers for arena-interned pointers
// ================================================================================================
//
// These free functions wrap the raw `unsafe { ptr.as_ref_unchecked() }` deref
// pattern behind a safe-looking API.
//
// # Safety contract (applies to every `view_*` helper below)
//
// The returned reference aliases arena memory. Callers must ensure the arena
// is alive for as long as the reference is used. In practice this holds
// whenever:
//
//   - The caller is reachable only during the execution phase (i.e., some
//     `ExecutionGuard` is alive on the call stack).
//   - The caller holds a value that transitively stores arena pointers (like
//     `ModuleIR` or `FunctionIR`), whose very existence implies the arena is
//     live.
//
// The helpers return `&'static` references, which is an intentional lifetime
// widening: the *real* lifetime is "until the next maintenance phase," but
// Rust has no way to spell that. Callers must not store these references
// beyond the scope where the above invariants hold.
//
// TODO (design follow-up): the `&'static` widening makes these references
// effectively raw pointers at the type level — the "arena is alive" proof is
// carried only in docs, not in the types. Consider tying the returned
// reference to a witness value instead:
//
//   - Parameterize these helpers by a borrow of an `ExecutionGuard` (or a
//     lightweight `&ArenaLive<'a>` token) so the returned reference gets
//     lifetime `'a` instead of `'static`. That statically prevents callers
//     from stashing the reference across a maintenance phase.
//   - Alternatively, make `InternedType` / `GlobalArenaPtr<T>` carry a
//     phantom lifetime and remove the free `view_*` functions in favor of
//     `InternedType::view(&guard)`-style methods, so the compiler enforces
//     that every deref is witnessed by a live guard.

/// Returns a reference to the arena-interned [`Type`] behind `ptr`.
pub fn view_type(ptr: InternedType) -> &'static Type {
    // SAFETY: see module-level contract above.
    unsafe { ptr.as_ref_unchecked() }
}

/// Returns a reference to the arena-interned list of [`InternedType`]s
/// behind `ptr`.
pub fn view_type_list(ptr: InternedTypeList) -> &'static [InternedType] {
    // SAFETY: see module-level contract above.
    unsafe { ptr.as_ref_unchecked() }
}

/// Returns a reference to the arena-interned identifier string behind `ptr`.
pub fn view_name(ptr: GlobalArenaPtr<str>) -> &'static str {
    // SAFETY: see module-level contract above.
    unsafe { ptr.as_ref_unchecked() }
}

/// Converts `&mut T` to `&T` by interning the immutable counterpart. Errors
/// if `mut_ref` is not a [`Type::MutRef`].
///
/// Reads through `view_type` and therefore inherits its safety contract.
pub fn convert_mut_to_immut_ref(
    interner: &impl Interner,
    mut_ref: InternedType,
) -> anyhow::Result<InternedType> {
    let Type::MutRef { inner } = view_type(mut_ref) else {
        anyhow::bail!("convert_mut_to_immut_ref: expected MutRef");
    };
    Ok(interner.immut_ref_of(*inner))
}

/// Strips the reference from `&T` or `&mut T`, returning `T`. Errors if
/// `ref_ty` is not a reference type.
///
/// Reads through `view_type` and therefore inherits its safety contract.
pub fn strip_ref(ref_ty: InternedType) -> anyhow::Result<InternedType> {
    let (Type::ImmutRef { inner } | Type::MutRef { inner }) = view_type(ref_ty) else {
        anyhow::bail!("strip_ref: expected reference type");
    };
    Ok(*inner)
}

/// Layout for struct fields:
///   - Offset of the field in flat memory representation.
///   - Pointer to the field's type for traversals (e.g., serialization).
#[derive(Copy, Clone)]
pub struct FieldLayout {
    pub offset: FieldOffset,
    #[allow(dead_code)]
    ty: InternedType,
}

impl FieldLayout {
    /// Creates a new field layout entry.
    pub fn new(offset: FieldOffset, ty: InternedType) -> Self {
        Self { offset, ty }
    }
}

/// Struct layout information: total size, alignment and information about the
/// field layouts.
pub struct StructLayout {
    /// Total size of the struct. Includes necessary padding based on the
    /// alignment requirements.
    pub size: Size,
    pub align: Alignment,
    fields: GlobalArenaPtr<[FieldLayout]>,
}

impl StructLayout {
    /// Creates a new struct layout entry.
    pub fn new(size: Size, align: Alignment, fields: GlobalArenaPtr<[FieldLayout]>) -> Self {
        Self {
            size,
            align,
            fields,
        }
    }

    // TODO: This API is test-only for now, will change, so ignore safety.
    pub fn field_layouts(&self) -> &[FieldLayout] {
        unsafe { self.fields.as_ref_unchecked() }
    }
}

// ================================================================================================
// Type enum
// ================================================================================================

/// A canonical type node in the arena-allocated canonical type DAG. Each node
/// is unique within the global arena: pointer equality implies structural
/// equality (interning guarantee).
pub enum Type {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
    /// Immutable reference to a type; stores a pointer to canonicalized
    /// pointee type.
    ImmutRef {
        inner: InternedType,
    },
    /// Mutable reference to a type; stores a pointer to canonicalized pointee
    /// type.
    MutRef {
        inner: InternedType,
    },
    /// Variable-length vector; stores a pointer to canonicalized element type.
    Vector {
        elem: InternedType,
    },
    /// Named struct with its layout. Layout is only set for fully-instantiated
    /// structs.
    Struct {
        // TODO: Make this a pointer to struct type struct which holds these pointers.
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
        ty_args: InternedTypeList,
        layout: Option<StructLayout>,
    },
    /// Named enum. Does not store any layout information as it may change (new
    /// variant can be added during module upgrade). Enum layouts are always
    /// resolved through the executable where they are defined.
    Enum {
        // TODO: Make this a pointer to enum type struct which holds these pointers.
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
        ty_args: InternedTypeList,
        // TODO: Optional layout for enums with fixed size (frozen).
    },
    /// Function type with argument types, result types and abilities.
    Function {
        args: InternedTypeList,
        results: InternedTypeList,
        abilities: AbilitySet,
    },
    /// Unresolved generic type parameter placeholder (index into the enclosing
    /// type-argument list). Note that pointer equality of type parameters does
    /// not guarantee anything. For example, for
    /// ```text
    /// struct A<T> { } // T is 0.
    ///
    /// struct B<T1, T2> {
    ///     x: A<T1>, // T1 is 0.
    ///     y: A<T2>, // T2 is 1.
    /// }
    /// ```
    /// `p: A<T>` and `q: B<T1, T2>` satisfy p == q.x, which is meaningless.
    TypeParam {
        idx: u16,
    },
}

impl Type {
    /// Returns the size and alignment of this type. Returns [`None`] if the
    /// size or alignment cannot be computed:
    ///   - If the type is a generic struct.
    ///   - If the type is an unresolved type parameter.
    /// In both cases, type substitution must run first.
    pub fn size_and_align(&self) -> Option<(Size, Alignment)> {
        Some(match self {
            // Primitives.
            Type::Bool | Type::U8 | Type::I8 => (1, 1),
            Type::U16 | Type::I16 => (2, 2),
            Type::U32 | Type::I32 => (4, 4),
            Type::U64 | Type::I64 => (8, 8),
            Type::U128 | Type::I128 => (16, 16),
            Type::U256 | Type::I256 | Type::Address | Type::Signer => (32, 32),

            // Vectors: pointer to the heap which stores vector metadata such
            // as length, capacity.
            Type::Vector { .. } => (8, 8),

            // References are 16-byte fat pointers.
            Type::ImmutRef { .. } | Type::MutRef { .. } => (16, 16),

            // Enums: always heap pointers because of upgradability.
            Type::Enum { .. } => (8, 8),

            // Function values - TODO: for now use heap pointer values.
            Type::Function { .. } => (8, 8),

            // Structs: the layout must be pre-computed for all fields inline.
            Type::Struct { layout, .. } => {
                match layout {
                    Some(layout) => (layout.size, layout.align),
                    None => {
                        // INVARIANT: If layout is unset, this struct contains
                        // generic type arguments.
                        return None;
                    },
                }
            },

            // Need type substitution to calculate the size and alignment.
            Type::TypeParam { .. } => {
                return None;
            },
        })
    }

    /// Returns layout for a struct type, or [`None`] for non-struct types or
    /// generic structs without a computed layout.
    pub fn struct_layout(&self) -> Option<&StructLayout> {
        match self {
            Type::Struct { layout, .. } => layout.as_ref(),
            Type::Bool
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256
            | Type::I8
            | Type::I16
            | Type::I32
            | Type::I64
            | Type::I128
            | Type::I256
            | Type::Address
            | Type::Signer
            | Type::ImmutRef { .. }
            | Type::MutRef { .. }
            | Type::Vector { .. }
            | Type::Enum { .. }
            | Type::Function { .. }
            | Type::TypeParam { .. } => None,
        }
    }
}

// ================================================================================================
// Static primitive type instances
// ================================================================================================

pub static BOOL: Type = Type::Bool;
pub static U8: Type = Type::U8;
pub static U16: Type = Type::U16;
pub static U32: Type = Type::U32;
pub static U64: Type = Type::U64;
pub static U128: Type = Type::U128;
pub static U256: Type = Type::U256;
pub static I8: Type = Type::I8;
pub static I16: Type = Type::I16;
pub static I32: Type = Type::I32;
pub static I64: Type = Type::I64;
pub static I128: Type = Type::I128;
pub static I256: Type = Type::I256;
pub static ADDRESS: Type = Type::Address;
pub static SIGNER: Type = Type::Signer;

// TODO: placeholder.
pub static EMPTY_LIST: [InternedType; 0] = [];

// ================================================================================================
// Interned-type constants for primitives
//
// These are the preferred way to spell "the interned type for this primitive"
// at call sites. They hide the underlying `GlobalArenaPtr::from_static` call.
// ================================================================================================

pub const BOOL_TY: InternedType = GlobalArenaPtr::from_static(&BOOL);
pub const U8_TY: InternedType = GlobalArenaPtr::from_static(&U8);
pub const U16_TY: InternedType = GlobalArenaPtr::from_static(&U16);
pub const U32_TY: InternedType = GlobalArenaPtr::from_static(&U32);
pub const U64_TY: InternedType = GlobalArenaPtr::from_static(&U64);
pub const U128_TY: InternedType = GlobalArenaPtr::from_static(&U128);
pub const U256_TY: InternedType = GlobalArenaPtr::from_static(&U256);
pub const I8_TY: InternedType = GlobalArenaPtr::from_static(&I8);
pub const I16_TY: InternedType = GlobalArenaPtr::from_static(&I16);
pub const I32_TY: InternedType = GlobalArenaPtr::from_static(&I32);
pub const I64_TY: InternedType = GlobalArenaPtr::from_static(&I64);
pub const I128_TY: InternedType = GlobalArenaPtr::from_static(&I128);
pub const I256_TY: InternedType = GlobalArenaPtr::from_static(&I256);
pub const ADDRESS_TY: InternedType = GlobalArenaPtr::from_static(&ADDRESS);
pub const SIGNER_TY: InternedType = GlobalArenaPtr::from_static(&SIGNER);

pub const EMPTY_TYPE_LIST: InternedTypeList = GlobalArenaPtr::from_static(&EMPTY_LIST);

// ================================================================================================
// Alignment utility
// ================================================================================================

/// Rounds a byte offset up to the next multiple of `align`.
///
/// **Pre-condition:** `align` is non-zero and is a power of two.
pub fn align_up(offset: u32, align: u32) -> u32 {
    debug_assert!(align > 0 && align.is_power_of_two());
    (offset + align - 1) & !(align - 1)
}
