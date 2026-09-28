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
    interner::{view_module_id, InternedIdentifier, InternedModuleId},
    prepared_module::FunctionSignature,
    Interner,
};
use mono_move_alloc::GlobalArenaPtr;
use move_core_types::{ability::AbilitySet, account_address::AccountAddress};
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

/// Whether `ty` can name a resource in global storage, i.e. it is a struct or
/// an enum type.
pub fn is_resource_type(ty: InternedType) -> bool {
    matches!(view_type(ty), Type::Nominal { .. })
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

/// Whether `ty` is the struct or enum `address::module::name`, whatever its
/// type arguments.
///
/// Inherits safety contract of [`view_type`].
pub fn is_nominal(ty: InternedType, address: &AccountAddress, module: &str, name: &str) -> bool {
    let Type::Nominal {
        module_id,
        name: ty_name,
        ..
    } = view_type(ty)
    else {
        return false;
    };
    let module_id = view_module_id(*module_id);
    module_id.address() == address
        && view_name(module_id.name()) == module
        && view_name(*ty_name) == name
}

/// Whether a value of type `actual` may be used where `expected` is required.
///
/// - Identical types are assignable.
/// - Two [`Type::Function`]s are assignable when their argument and result
///   lists are identical and `expected`'s abilities are a subset of `actual`'s.
/// - Two [`Type::ImmutRef`]s are assignable when their pointees are.
/// - Nothing else is assignable; in particular, `&mut T` and `&T` are not.
///
/// # Preconditions
///
/// Both types must come from the same interner, and from the same
/// type-parameter scope: a [`Type::TypeParam`] is interned by index alone, so
/// parameters sharing an index across scopes are the same pointer.
///
/// Inherits safety contract of [`view_type`].
///
/// TODO(metering): unbounded recursion on reference nesting; same family as the
/// `TODO(metering)` on [`is_closed_type`]. Depth is 1 in practice because
/// nested references are not expressible.
pub fn is_assignable(expected: InternedType, actual: InternedType) -> bool {
    // Interning makes pointer equality structural equality, which settles every
    // invariant constructor: below, only the two variant positions do work.
    if expected == actual {
        return true;
    }
    match view_type(expected) {
        Type::Function {
            args,
            results,
            abilities,
        } => matches!(
            view_type(actual),
            Type::Function {
                args: actual_args,
                results: actual_results,
                abilities: actual_abilities,
            } if args == actual_args
                && results == actual_results
                && abilities.is_subset(*actual_abilities)
        ),
        Type::ImmutRef { inner } => matches!(
            view_type(actual),
            Type::ImmutRef { inner: actual_inner } if is_assignable(*inner, *actual_inner)
        ),
        // Invariant: pointer inequality above already decided these. Listed
        // explicitly so a new `Type` variant forces a variance decision.
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
        | Type::MutRef { .. }
        | Type::Vector { .. }
        | Type::Nominal { .. }
        | Type::TypeParam { .. } => false,
    }
}

/// Why a declared function signature cannot be used at an expected function
/// type. Mirrors the distinctions the caller turns into reflection error codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionTypeMismatch {
    /// The expected type is not a function type at all.
    NotAFunction,
    /// The two signatures cannot be matched.
    Incompatible,
    /// Matching succeeded but left a declared type parameter unbound.
    NotInstantiated,
}

/// Matches a function's declared signature against the concrete function type
/// it is expected to have, inferring one type argument per declared type
/// parameter.
///
/// Every inferred argument is a pointer taken from `expected`, so nothing new
/// is interned.
///
/// # Preconditions
///
/// `expected` is closed: it comes from a monomorphized call site, where every
/// type is already concrete. Otherwise the pointer-equality fast paths below
/// would silently accept a type parameter without binding it.
///
/// TODO(metering): unbounded recursion on nesting depth, same family as the
/// `TODO(metering)` on [`is_closed_type`].
pub fn infer_function_type_args(
    declared: FunctionSignature,
    expected: InternedType,
    num_ty_params: usize,
) -> Result<Vec<InternedType>, FunctionTypeMismatch> {
    debug_assert!(is_closed_type(expected), "expected type must be closed");

    let Type::Function { args, results, .. } = view_type(expected) else {
        return Err(FunctionTypeMismatch::NotAFunction);
    };

    let mut bindings = vec![None; num_ty_params];
    if !match_ty_list(declared.params, *args, &mut bindings)
        || !match_ty_list(declared.returns, *results, &mut bindings)
    {
        return Err(FunctionTypeMismatch::Incompatible);
    }

    bindings
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or(FunctionTypeMismatch::NotInstantiated)
}

/// Matches `declared` against `expected` positionally, recording any type
/// parameter binding it discovers in `bindings`.
fn match_ty_list(
    declared: InternedTypeList,
    expected: InternedTypeList,
    bindings: &mut [Option<InternedType>],
) -> bool {
    if declared == expected {
        return true;
    }
    let declared = view_type_list(declared);
    let expected = view_type_list(expected);
    declared.len() == expected.len()
        && (declared.iter())
            .zip(expected)
            .all(|(&d, &e)| match_ty(d, e, bindings))
}

/// See [`match_ty_list`].
fn match_ty(
    declared: InternedType,
    expected: InternedType,
    bindings: &mut [Option<InternedType>],
) -> bool {
    // Interning makes pointer equality structural equality, so an identical
    // subtree needs no walk. Since `expected` is closed, so is `declared` here,
    // and there is no binding to record.
    if declared == expected {
        return true;
    }
    match view_type(declared) {
        Type::TypeParam { idx } => {
            // A reference is not a valid type argument, and every occurrence of
            // the same parameter must agree.
            if matches!(
                view_type(expected),
                Type::ImmutRef { .. } | Type::MutRef { .. }
            ) {
                return false;
            }
            match bindings.get_mut(*idx as usize) {
                Some(slot) => *slot.get_or_insert(expected) == expected,
                None => false,
            }
        },
        Type::Vector { elem } => {
            let Type::Vector { elem: expected } = view_type(expected) else {
                return false;
            };
            match_ty(*elem, *expected, bindings)
        },
        Type::ImmutRef { inner } => {
            let Type::ImmutRef { inner: expected } = view_type(expected) else {
                return false;
            };
            match_ty(*inner, *expected, bindings)
        },
        Type::MutRef { inner } => {
            let Type::MutRef { inner: expected } = view_type(expected) else {
                return false;
            };
            match_ty(*inner, *expected, bindings)
        },
        Type::Nominal {
            module_id,
            name,
            ty_args,
        } => {
            let Type::Nominal {
                module_id: expected_module_id,
                name: expected_name,
                ty_args: expected_ty_args,
            } = view_type(expected)
            else {
                return false;
            };
            module_id == expected_module_id
                && name == expected_name
                && match_ty_list(*ty_args, *expected_ty_args, bindings)
        },
        Type::Function {
            args,
            results,
            abilities,
        } => {
            let Type::Function {
                args: expected_args,
                results: expected_results,
                abilities: expected_abilities,
            } = view_type(expected)
            else {
                return false;
            };
            abilities == expected_abilities
                && match_ty_list(*args, *expected_args, bindings)
                && match_ty_list(*results, *expected_results, bindings)
        },
        // Primitives: the pointer comparison above was the whole test. Listed
        // explicitly so a new `Type` variant forces a decision here.
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
        | Type::Signer => false,
    }
}

/// Whether `ty` contains no [`Type::TypeParam`] node.
///
/// Inherits safety contract of [`view_type`].
/// TODO(metering): memoize by interned type and convert to non-recursive.
/// The recursion has no cache and `.all()` short-circuits only on `false`,
/// so a type whose interned tree shares subtypes is traversed exponentially.
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

/// Whether `ty` is `signer` or `&signer`, the parameter shapes that fill from
/// a transaction signer.
pub fn is_signer_or_signer_immut_ref(ty: InternedType) -> bool {
    match view_type(ty) {
        Type::Signer => true,
        Type::ImmutRef { inner } => matches!(view_type(*inner), Type::Signer),
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
        | Type::MutRef { .. }
        | Type::Vector { .. }
        | Type::Nominal { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => false,
    }
}

impl Type {
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

/// Writes a textual representation of an interned type. Inherits the arena
/// safety contract on [`view_type`].
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
        Type::Function {
            args,
            results,
            abilities,
        } => {
            write!(f, "|")?;
            display_type_list(f, *args)?;
            write!(f, "|(")?;
            display_type_list(f, *results)?;
            write!(f, "){}", abilities.display_postfix())
        },
    }
}

/// Renders an interned type to a string (see [`display_type`]).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interner::ModuleId;
    use move_core_types::ability::Ability;
    use std::collections::HashMap;

    /// Leak-based stand-in for the global interner. Pointer equality has to be
    /// structural equality, so each distinct type is allocated once and keyed
    /// by its rendering.
    #[derive(Default)]
    struct TestInterner {
        types: HashMap<String, InternedType>,
        lists: HashMap<String, InternedTypeList>,
        names: HashMap<String, InternedIdentifier>,
        modules: HashMap<String, InternedModuleId>,
    }

    impl TestInterner {
        fn ty(&mut self, ty: Type) -> InternedType {
            let ptr = GlobalArenaPtr::from_static(Box::leak(Box::new(ty)));
            *self.types.entry(type_to_string(ptr)).or_insert(ptr)
        }

        fn list(&mut self, tys: &[InternedType]) -> InternedTypeList {
            let key = tys.iter().copied().map(type_to_string).collect::<String>();
            *self.lists.entry(key).or_insert_with(|| {
                let leaked = Box::leak(tys.to_vec().into_boxed_slice());
                InternedTypeList::new(GlobalArenaPtr::from_static(leaked))
            })
        }

        fn name(&mut self, name: &str) -> InternedIdentifier {
            *self.names.entry(name.to_string()).or_insert_with(|| {
                GlobalArenaPtr::from_static(Box::leak(name.to_string().into_boxed_str()))
            })
        }

        fn nominal(&mut self, module: &str, name: &str, ty_args: &[InternedType]) -> InternedType {
            let module_name = self.name(module);
            let module_id = *self.modules.entry(module.to_string()).or_insert_with(|| {
                GlobalArenaPtr::from_static(Box::leak(Box::new(ModuleId::new(
                    AccountAddress::ONE,
                    module_name,
                ))))
            });
            let name = self.name(name);
            let ty_args = self.list(ty_args);
            self.ty(Type::Nominal {
                module_id,
                name,
                ty_args,
            })
        }

        fn vector(&mut self, elem: InternedType) -> InternedType {
            self.ty(Type::Vector { elem })
        }

        fn param(&mut self, idx: u16) -> InternedType {
            self.ty(Type::TypeParam { idx })
        }

        fn func(
            &mut self,
            args: &[InternedType],
            results: &[InternedType],
            abilities: AbilitySet,
        ) -> InternedType {
            let args = self.list(args);
            let results = self.list(results);
            self.ty(Type::Function {
                args,
                results,
                abilities,
            })
        }

        fn signature(
            &mut self,
            params: &[InternedType],
            returns: &[InternedType],
        ) -> FunctionSignature {
            FunctionSignature {
                params: self.list(params),
                returns: self.list(returns),
            }
        }
    }

    fn copy_drop() -> AbilitySet {
        AbilitySet::EMPTY | Ability::Copy | Ability::Drop
    }

    #[test]
    fn non_generic_signature_infers_nothing() {
        let mut i = TestInterner::default();
        let declared = i.signature(&[U64_TY, U64_TY], &[U64_TY]);
        let expected = i.func(&[U64_TY, U64_TY], &[U64_TY], copy_drop());

        assert_eq!(infer_function_type_args(declared, expected, 0), Ok(vec![]));
    }

    #[test]
    fn type_parameters_are_inferred_from_the_expected_type() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let t1 = i.param(1);
        let vec_t1 = i.vector(t1);
        let declared = i.signature(&[t0, vec_t1], &[t0]);

        let vec_bool = i.vector(BOOL_TY);
        let expected = i.func(&[U64_TY, vec_bool], &[U64_TY], copy_drop());

        assert_eq!(
            infer_function_type_args(declared, expected, 2),
            Ok(vec![U64_TY, BOOL_TY])
        );
    }

    #[test]
    fn inference_reaches_into_nominal_type_arguments() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let box_t0 = i.nominal("boxes", "Box", &[t0]);
        let declared = i.signature(&[box_t0], &[]);

        let box_address = i.nominal("boxes", "Box", &[ADDRESS_TY]);
        let expected = i.func(&[box_address], &[], copy_drop());

        assert_eq!(
            infer_function_type_args(declared, expected, 1),
            Ok(vec![ADDRESS_TY])
        );
    }

    #[test]
    fn a_parameter_used_twice_must_agree() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let declared = i.signature(&[t0, t0], &[]);

        let agreeing = i.func(&[U64_TY, U64_TY], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, agreeing, 1),
            Ok(vec![U64_TY])
        );

        let conflicting = i.func(&[U64_TY, BOOL_TY], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, conflicting, 1),
            Err(FunctionTypeMismatch::Incompatible)
        );
    }

    #[test]
    fn a_reference_is_not_a_valid_type_argument() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let declared = i.signature(&[t0], &[]);

        for inner in [Type::ImmutRef { inner: U64_TY }, Type::MutRef {
            inner: U64_TY,
        }] {
            let ref_ty = i.ty(inner);
            let expected = i.func(&[ref_ty], &[], copy_drop());
            assert_eq!(
                infer_function_type_args(declared, expected, 1),
                Err(FunctionTypeMismatch::Incompatible)
            );
        }
    }

    #[test]
    fn a_reference_parameter_still_matches_a_reference() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let ref_t0 = i.ty(Type::ImmutRef { inner: t0 });
        let declared = i.signature(&[ref_t0], &[]);

        let ref_u64 = i.ty(Type::ImmutRef { inner: U64_TY });
        let expected = i.func(&[ref_u64], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, expected, 1),
            Ok(vec![U64_TY])
        );

        // Mutability is invariant.
        let mut_u64 = i.ty(Type::MutRef { inner: U64_TY });
        let expected = i.func(&[mut_u64], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, expected, 1),
            Err(FunctionTypeMismatch::Incompatible)
        );
    }

    #[test]
    fn a_nested_function_type_must_have_the_very_same_abilities() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let callback = i.func(&[t0], &[], copy_drop());
        let declared = i.signature(&[callback], &[]);

        let matching = i.func(&[U64_TY], &[], copy_drop());
        let expected = i.func(&[matching], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, expected, 1),
            Ok(vec![U64_TY])
        );

        let weaker = i.func(&[U64_TY], &[], AbilitySet::EMPTY | Ability::Drop);
        let expected = i.func(&[weaker], &[], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, expected, 1),
            Err(FunctionTypeMismatch::Incompatible)
        );
    }

    #[test]
    fn arity_mismatches_are_incompatible() {
        let mut i = TestInterner::default();
        let declared = i.signature(&[U64_TY], &[U64_TY]);

        let too_few_args = i.func(&[], &[U64_TY], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, too_few_args, 0),
            Err(FunctionTypeMismatch::Incompatible)
        );

        let too_many_returns = i.func(&[U64_TY], &[U64_TY, U64_TY], copy_drop());
        assert_eq!(
            infer_function_type_args(declared, too_many_returns, 0),
            Err(FunctionTypeMismatch::Incompatible)
        );
    }

    #[test]
    fn a_parameter_the_signature_never_mentions_stays_unbound() {
        let mut i = TestInterner::default();
        let t0 = i.param(0);
        let declared = i.signature(&[t0], &[]);
        let expected = i.func(&[U64_TY], &[], copy_drop());

        assert_eq!(
            infer_function_type_args(declared, expected, 2),
            Err(FunctionTypeMismatch::NotInstantiated)
        );
    }

    #[test]
    fn a_non_function_expected_type_is_rejected() {
        let mut i = TestInterner::default();
        let declared = i.signature(&[], &[]);

        assert_eq!(
            infer_function_type_args(declared, U64_TY, 0),
            Err(FunctionTypeMismatch::NotAFunction)
        );
    }
}
