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
//! Size of references is 16 bytes (fat pointers). Alignment is 8 bytes —
//! each half (`base_ptr` and `byte_offset`) is an 8-byte word.
//!
//! ## Fully-instantiated structs and enums
//!
//! Struct and enum types are arena-allocated, and store module ID, name and
//! type arguments that uniquely identify the type.
//!
//! ## Generic structs

use crate::{
    interner::{InternedIdentifier, InternedModuleId},
    Interner,
};
use mono_move_alloc::GlobalArenaPtr;
use move_core_types::ability::AbilitySet;
use std::{cmp::PartialEq, fmt};

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

/// An enum variant's discriminant (the tag stored at `ENUM_TAG_OFFSET`).
pub type VariantTag = u64;

/// Pointer to an arena-interned [`Type`]. Pointer equality implies structural
/// equality because the global interner deduplicates types. The alias hides
/// the raw `GlobalArenaPtr<Type>` form throughout the codebase.
pub type InternedType = GlobalArenaPtr<Type>;

/// Pointer to an arena-interned list of [`InternedType`]s (e.g., function
/// parameter/return types, generic type arguments). The list itself is also
/// interned and deduplicated.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct InternedTypeList(GlobalArenaPtr<[InternedType]>);

impl InternedTypeList {
    /// Returns a new arena-interned type list.
    pub fn new(tys: GlobalArenaPtr<[InternedType]>) -> Self {
        Self(tys)
    }

    /// Returns true if this type list is empty.
    pub fn is_empty(&self) -> bool {
        self == &EMPTY_TYPE_LIST
    }
}

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
// TODO(cleanup): the `&'static` widening makes these references
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
    unsafe { ptr.0.as_ref_unchecked() }
}

/// Returns a reference to the arena-interned identifier string behind `ptr`.
pub fn view_name(ptr: InternedIdentifier) -> &'static str {
    // SAFETY: see module-level contract above.
    unsafe { ptr.as_ref_unchecked() }
}

/// Converts `&mut T` to `&T` by interning the immutable counterpart.
/// Returns [`None`] if `mut_ref` is not a [`Type::MutRef`].
///
/// Inherits safety contract of [`view_type`].
pub fn convert_mut_to_immut_ref(
    interner: &impl Interner,
    mut_ref: InternedType,
) -> Option<InternedType> {
    let Type::MutRef { inner } = view_type(mut_ref) else {
        return None;
    };
    Some(interner.immut_ref_of(*inner))
}

/// Strips the reference from `&T` or `&mut T`, returning `T`.
/// Returns [`None`] if `ref_ty` is not a reference type.
///
/// Inherits safety contract of [`view_type`].
pub fn strip_ref(ref_ty: InternedType) -> Option<InternedType> {
    let (Type::ImmutRef { inner } | Type::MutRef { inner }) = view_type(ref_ty) else {
        return None;
    };
    Some(*inner)
}

/// Whether `ty` contains no [`Type::TypeParam`] node.
///
/// Inherits safety contract of [`view_type`].
/// TODO(metering): convert to non-recursive.
pub fn is_closed_type(ty: InternedType) -> bool {
    match view_type(ty) {
        Type::TypeParam { .. } => false,
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
        | Type::Signer => true,
        Type::Vector { elem } => is_closed_type(*elem),
        Type::ImmutRef { inner } | Type::MutRef { inner } => is_closed_type(*inner),
        Type::Nominal { ty_args, .. } => {
            view_type_list(*ty_args).iter().copied().all(is_closed_type)
        },
        Type::Function { args, results, .. } => {
            view_type_list(*args).iter().copied().all(is_closed_type)
                && view_type_list(*results).iter().copied().all(is_closed_type)
        },
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
    /// Nominal type — a struct or enum identified by module, name, and type
    /// arguments.
    Nominal {
        // TODO(cleanup): Make this a pointer to a named-type struct holding these pointers.
        module_id: InternedModuleId,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
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

/// In-memory slot width and alignment for the shapes whose size is intrinsic:
/// primitives, references (16-byte fat pointers), and vectors and function
/// values (8-byte heap-pointer slots). Returns [`None`] for nominal types and
/// for type parameters.
pub fn intrinsic_slot_size_and_align(ty: &Type) -> Option<(Size, Alignment)> {
    Some(match ty {
        // Primitives.
        Type::Bool | Type::U8 | Type::I8 => (1, 1),
        Type::U16 | Type::I16 => (2, 2),
        Type::U32 | Type::I32 => (4, 4),
        Type::U64 | Type::I64 => (8, 8),
        Type::U128 | Type::I128 => (16, 8),
        Type::U256 | Type::I256 | Type::Address | Type::Signer => (32, 8),

        // Vectors: pointer to the heap which stores vector metadata such as
        // length, capacity.
        Type::Vector { .. } => (8, 8),

        // References are 16-byte fat pointers, 8-byte aligned.
        Type::ImmutRef { .. } | Type::MutRef { .. } => (16, 8),

        // Function values - TODO(completeness): for now use heap pointer values.
        Type::Function { .. } => (8, 8),

        // Nominal size is the sum of its fields (resolved through the layout
        // table); type parameters need substitution first.
        Type::Nominal { .. } | Type::TypeParam { .. } => return None,
    })
}

impl Type {
    /// The short kind word for this type (`"u64"`, `"vector"`, `"struct"`, ...).
    /// Mirrors the legacy VM's `TypeTag::to_short_string`.
    pub fn short_name(&self) -> &'static str {
        match self {
            Type::Bool => "bool",
            Type::U8 => "u8",
            Type::U16 => "u16",
            Type::U32 => "u32",
            Type::U64 => "u64",
            Type::U128 => "u128",
            Type::U256 => "u256",
            Type::I8 => "i8",
            Type::I16 => "i16",
            Type::I32 => "i32",
            Type::I64 => "i64",
            Type::I128 => "i128",
            Type::I256 => "i256",
            Type::Address => "address",
            Type::Signer => "signer",
            Type::Vector { .. } => "vector",
            Type::Nominal { .. } => "struct",
            Type::Function { .. } => "function",
            Type::ImmutRef { .. } | Type::MutRef { .. } => "reference",
            Type::TypeParam { .. } => "type parameter",
        }
    }

    /// True iff this is `Type::U64`. Used by the specializer to gate the
    /// u64-specialized micro-op fast paths.
    #[inline(always)]
    pub fn is_u64(&self) -> bool {
        matches!(self, Type::U64)
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

pub const EMPTY_TYPE_LIST: InternedTypeList =
    InternedTypeList(GlobalArenaPtr::from_static(&EMPTY_LIST));

/// Writes a textual representation of an interned type. Nominals print
/// just their name — IR variants that carry `ty_args` show them
/// separately. Inherits the arena safety contract on [`view_type`].
pub fn display_type(f: &mut fmt::Formatter<'_>, ty: InternedType) -> fmt::Result {
    match view_type(ty) {
        Type::Bool => write!(f, "bool"),
        Type::U8 => write!(f, "u8"),
        Type::U16 => write!(f, "u16"),
        Type::U32 => write!(f, "u32"),
        Type::U64 => write!(f, "u64"),
        Type::U128 => write!(f, "u128"),
        Type::U256 => write!(f, "u256"),
        Type::I8 => write!(f, "i8"),
        Type::I16 => write!(f, "i16"),
        Type::I32 => write!(f, "i32"),
        Type::I64 => write!(f, "i64"),
        Type::I128 => write!(f, "i128"),
        Type::I256 => write!(f, "i256"),
        Type::Address => write!(f, "address"),
        Type::Signer => write!(f, "signer"),
        Type::TypeParam { idx } => write!(f, "_{}", idx),
        Type::Vector { elem } => {
            write!(f, "vector<")?;
            display_type(f, *elem)?;
            write!(f, ">")
        },
        Type::ImmutRef { inner } => {
            write!(f, "&")?;
            display_type(f, *inner)
        },
        Type::MutRef { inner } => {
            write!(f, "&mut ")?;
            display_type(f, *inner)
        },
        Type::Nominal {
            module_id,
            name,
            ty_args,
            ..
        } => {
            let module_id = unsafe { module_id.as_ref_unchecked() };
            let addr = module_id.address().short_str_lossless();
            let module_name = view_name(module_id.name());
            write!(f, "0x{}::{}::{}", addr, module_name, view_name(*name))?;
            if !ty_args.is_empty() {
                write!(f, "<")?;
                display_type_list(f, *ty_args)?;
                write!(f, ">")?;
            }
            Ok(())
        },
        Type::Function { args, results, .. } => {
            write!(f, "|")?;
            display_type_list(f, *args)?;
            write!(f, "|")?;
            display_type_list(f, *results)?;
            Ok(())
        },
    }
}

/// Renders an interned type to its textual representation (see [`display_type`]).
//
// TODO(metering): this traversal is unbounded; replace with a metered, depth-bounded
// version.
pub fn type_to_string(ty: InternedType) -> String {
    struct Disp(InternedType);
    impl fmt::Display for Disp {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            display_type(f, self.0)
        }
    }
    Disp(ty).to_string()
}

/// Writes an interned type list as `T0, T1, ...`.
pub fn display_type_list(f: &mut fmt::Formatter<'_>, types: InternedTypeList) -> fmt::Result {
    for (i, ty) in view_type_list(types).iter().enumerate() {
        if i > 0 {
            write!(f, ", ")?;
        }
        display_type(f, *ty)?;
    }
    Ok(())
}
