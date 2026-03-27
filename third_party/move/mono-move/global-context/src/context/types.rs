// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type interning and layout metadata.
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

use crate::ExecutionGuard;
use dashmap::{Entry, Equivalent};
use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::ExecutableId;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use std::hash::{Hash, Hasher};

/// Total size of the type in flat memory including padding and any alignment.
pub type Size = u32;

/// When [`Type`] is stored in flat memory, the start address needs to be
/// this many bytes aligned.
pub type Alignment = u32;

/// Offset in bytes of struct fields in flat memory.
pub type FieldOffset = u32;

/// Layout for struct fields:
///   - Offset of the field in flat memory representation.
///   - Pointer to the field's type for traversals (e.g., serialization).
#[derive(Copy, Clone)]
pub struct FieldLayout {
    pub offset: FieldOffset,
    #[allow(dead_code)]
    ty: GlobalArenaPtr<Type>,
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
        inner: GlobalArenaPtr<Type>,
    },
    /// Mutable reference to a type; stores a pointer to canonicalized pointee
    /// type.
    MutRef {
        inner: GlobalArenaPtr<Type>,
    },
    /// Variable-length vector; stores a pointer to canonicalized element type.
    Vector {
        elem: GlobalArenaPtr<Type>,
    },
    /// Named struct with its layout. Layout is only set for fully-instantiated
    /// structs.
    Struct {
        // TODO: Make this a pointer to struct type struct which holds these pointers.
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
        ty_args: GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
        layout: Option<StructLayout>,
    },
    /// Named enum. Does not store any layout information as it may change (new
    /// variant can be added during module upgrade). Enum layouts are always
    /// resolved through the executable where they are defined.
    Enum {
        // TODO: Make this a pointer to enum type struct which holds these pointers.
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
        ty_args: GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
        // TODO: Optional layout for enums with fixed size (frozen).
    },
    // TODO: Support function types.
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
            | Type::TypeParam { .. } => None,
        }
    }
}

impl StructLayout {
    // TODO: This API is test-only for now, will change, so ignore safety.
    pub fn field_layouts(&self) -> &[FieldLayout] {
        unsafe { self.fields.as_ref_unchecked() }
    }
}

//
// Only private APIs below.
// ------------------------

impl FieldLayout {
    /// Returns field layout information.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the type pointer is valid.
    pub(super) fn new(offset: FieldOffset, ty: GlobalArenaPtr<Type>) -> Self {
        Self { offset, ty }
    }
}

impl StructLayout {
    /// Returns struct layout information.
    ///
    /// # Safety
    ///
    /// The caller must ensure that the field layouts pointer is valid.
    pub(super) fn new(size: Size, align: Alignment, fields: GlobalArenaPtr<[FieldLayout]>) -> Self {
        Self {
            size,
            align,
            fields,
        }
    }
}

impl<'ctx> ExecutionGuard<'ctx> {
    /// Returns the type pointer corresponding to the token if it has been
    /// interned before, or [`None`] otherwise.
    ///
    /// # Safety
    ///
    /// For any pointer that exists in the map, it must be still alive.
    pub(super) fn get_interned_type_pointer_internal(
        &self,
        token: &SignatureToken,
        module: &CompiledModule,
    ) -> Option<GlobalArenaPtr<Type>> {
        // SAFETY: All existing keys/values are valid pointers because the map
        // is guaranteed to be cleared on arena's reset.
        self.ctx
            .types
            .get(&SignatureTokenKey(token, module))
            .map(|entry| *entry.value())
    }

    /// Inserts the newly allocated type pointer into deduplication map. If the
    /// entry exists, the allocated pointer is discarded and the existing
    /// pointer is returned. If the entry does not exist, the allocated pointer
    /// is inserted and its copy returned.
    ///
    /// # Safety
    ///
    ///   1. The caller must ensure that the inserted pointer is alive.
    ///   2. For any pointer that exists in the map, it must be still alive.
    pub(super) fn insert_allocated_type_pointer_internal(
        &self,
        ptr: GlobalArenaPtr<Type>,
    ) -> GlobalArenaPtr<Type> {
        // SAFETY: We have just allocated the pointer, hence it is safe to wrap
        // it as a key and compute hash / equality. All existing keys are also
        // valid pointers because the map is cleared on arena's reset.
        let key = TypeInternerKey(ptr);
        match self.ctx.types.entry(key) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        }
    }
}

static BOOL: Type = Type::Bool;
static U8: Type = Type::U8;
static U16: Type = Type::U16;
static U32: Type = Type::U32;
static U64: Type = Type::U64;
static U128: Type = Type::U128;
static U256: Type = Type::U256;
static I8: Type = Type::I8;
static I16: Type = Type::I16;
static I32: Type = Type::I32;
static I64: Type = Type::I64;
static I128: Type = Type::I128;
static I256: Type = Type::I256;
static ADDRESS: Type = Type::Address;
static SIGNER: Type = Type::Signer;

// TODO: placeholder.
pub(super) static EMPTY_LIST: [GlobalArenaPtr<Type>; 0] = [];

/// Returns a static primitive type pointer for primitive signature tokens,
/// or [`None`] for composite types that require arena allocation.
pub(super) fn try_as_primitive_type(token: &SignatureToken) -> Option<GlobalArenaPtr<Type>> {
    use SignatureToken as S;

    match token {
        S::Bool => Some(GlobalArenaPtr::from_static(&BOOL)),
        S::U8 => Some(GlobalArenaPtr::from_static(&U8)),
        S::U16 => Some(GlobalArenaPtr::from_static(&U16)),
        S::U32 => Some(GlobalArenaPtr::from_static(&U32)),
        S::U64 => Some(GlobalArenaPtr::from_static(&U64)),
        S::U128 => Some(GlobalArenaPtr::from_static(&U128)),
        S::U256 => Some(GlobalArenaPtr::from_static(&U256)),
        S::I8 => Some(GlobalArenaPtr::from_static(&I8)),
        S::I16 => Some(GlobalArenaPtr::from_static(&I16)),
        S::I32 => Some(GlobalArenaPtr::from_static(&I32)),
        S::I64 => Some(GlobalArenaPtr::from_static(&I64)),
        S::I128 => Some(GlobalArenaPtr::from_static(&I128)),
        S::I256 => Some(GlobalArenaPtr::from_static(&I256)),
        S::Address => Some(GlobalArenaPtr::from_static(&ADDRESS)),
        S::Signer => Some(GlobalArenaPtr::from_static(&SIGNER)),
        S::Vector(_)
        | S::Function(_, _, _)
        | S::Struct(_)
        | S::StructInstantiation(_, _)
        | S::Reference(_)
        | S::MutableReference(_)
        | S::TypeParameter(_) => None,
    }
}

/// Canonical discriminants for cross-format hashing. This ensures that type
/// interner keys hash in the same way as signature tokens.
mod type_discriminant {
    pub(super) const BOOL: u8 = 0;
    pub(super) const U8: u8 = 1;
    pub(super) const U16: u8 = 2;
    pub(super) const U32: u8 = 3;
    pub(super) const U64: u8 = 4;
    pub(super) const U128: u8 = 5;
    pub(super) const U256: u8 = 6;
    pub(super) const I8: u8 = 7;
    pub(super) const I16: u8 = 8;
    pub(super) const I32: u8 = 9;
    pub(super) const I64: u8 = 10;
    pub(super) const I128: u8 = 11;
    pub(super) const I256: u8 = 12;
    pub(super) const ADDRESS: u8 = 13;
    pub(super) const SIGNER: u8 = 14;
    pub(super) const REFERENCE: u8 = 15;
    pub(super) const REFERENCE_MUT: u8 = 16;
    pub(super) const VECTOR: u8 = 17;
    pub(super) const STRUCT: u8 = 18;
    #[allow(dead_code)]
    pub(super) const FUNCTION: u8 = 19;
    pub(super) const TYPE_PARAM: u8 = 20;
}

/// Wraps allocated type pointer to implement structural hash and equality.
///
/// # Safety
///
/// Constructor must enforce the pointer points to the valid data and can be
/// safely dereferenced.
pub(super) struct TypeInternerKey(GlobalArenaPtr<Type>);

impl Hash for TypeInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use Type::*;

        // TODO: non-recursive implementation.

        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        let ty = unsafe { self.0.as_ref_unchecked() };
        match ty {
            Bool => {
                type_discriminant::BOOL.hash(state);
            },
            U8 => {
                type_discriminant::U8.hash(state);
            },
            U16 => {
                type_discriminant::U16.hash(state);
            },
            U32 => {
                type_discriminant::U32.hash(state);
            },
            U64 => {
                type_discriminant::U64.hash(state);
            },
            U128 => {
                type_discriminant::U128.hash(state);
            },
            U256 => {
                type_discriminant::U256.hash(state);
            },
            I8 => {
                type_discriminant::I8.hash(state);
            },
            I16 => {
                type_discriminant::I16.hash(state);
            },
            I32 => {
                type_discriminant::I32.hash(state);
            },
            I64 => {
                type_discriminant::I64.hash(state);
            },
            I128 => {
                type_discriminant::I128.hash(state);
            },
            I256 => {
                type_discriminant::I256.hash(state);
            },
            Address => {
                type_discriminant::ADDRESS.hash(state);
            },
            Signer => {
                type_discriminant::SIGNER.hash(state);
            },
            ImmutRef { inner } => {
                type_discriminant::REFERENCE.hash(state);
                Self(*inner).hash(state);
            },
            MutRef { inner } => {
                type_discriminant::REFERENCE_MUT.hash(state);
                Self(*inner).hash(state);
            },
            Vector { elem } => {
                type_discriminant::VECTOR.hash(state);
                Self(*elem).hash(state);
            },
            Struct {
                executable_id,
                name,
                ..
            }
            | Enum {
                executable_id,
                name,
                ..
            } => {
                // SAFETY: It is safe to dereference pointers because the
                // caller ensures they remain valid during the lifetime of
                // the key.
                let executable_id = unsafe { executable_id.as_ref_unchecked() };
                let executable_name = unsafe { executable_id.name().as_ref_unchecked() };
                let name = unsafe { name.as_ref_unchecked() };

                // Must use structural hash because it is compared against the
                // hash of lookup key (e.g., signature token). Enums reuse the
                // same discriminant as structs because type identity is based
                // on address, executable name, name and type arguments.
                type_discriminant::STRUCT.hash(state);
                executable_id.address().hash(state);
                executable_name.hash(state);
                name.hash(state);

                // TODO:
                //   1. Hash length of type arguments.
                //   2. Hash type arguments.
                0usize.hash(state);
            },
            TypeParam { idx } => {
                type_discriminant::TYPE_PARAM.hash(state);
                idx.hash(state);
            },
        }
    }
}

impl PartialEq for TypeInternerKey {
    fn eq(&self, other: &Self) -> bool {
        use Type::*;

        // TODO: non-recursive implementation.

        // SAFETY: It is safe to dereference pointers because the caller
        // ensures they remain valid during the lifetime of the key.
        let this = unsafe { self.0.as_ref_unchecked() };
        let other = unsafe { other.0.as_ref_unchecked() };

        match this {
            Bool => matches!(other, Bool),
            U8 => matches!(other, U8),
            U16 => matches!(other, U16),
            U32 => matches!(other, U32),
            U64 => matches!(other, U64),
            U128 => matches!(other, U128),
            U256 => matches!(other, U256),
            I8 => matches!(other, I8),
            I16 => matches!(other, I16),
            I32 => matches!(other, I32),
            I64 => matches!(other, I64),
            I128 => matches!(other, I128),
            I256 => matches!(other, I256),
            Address => matches!(other, Address),
            Signer => matches!(other, Signer),
            ImmutRef { inner } => {
                if let ImmutRef { inner: other_inner } = other {
                    // SAFETY: Inner pointers are already canonical pointers,
                    // so it is safe to compare by pointer equality.
                    *inner == *other_inner
                } else {
                    false
                }
            },
            MutRef { inner } => {
                if let MutRef { inner: other_inner } = other {
                    // SAFETY: Inner pointers are already canonical pointers,
                    // so it is safe to compare by pointer equality.
                    *inner == *other_inner
                } else {
                    false
                }
            },
            Vector { elem } => {
                if let Vector { elem: other_elem } = other {
                    // SAFETY: Inner pointers are already canonical pointers,
                    // so it is safe to compare by pointer equality.
                    *elem == *other_elem
                } else {
                    false
                }
            },
            Struct {
                executable_id,
                name,
                ty_args,
                ..
            } => {
                if let Struct {
                    executable_id: other_executable_id,
                    name: other_name,
                    ty_args: other_ty_args,
                    ..
                } = other
                {
                    // SAFETY: Inner pointers are already canonical pointers,
                    // so it is safe to compare by pointer equality.
                    executable_id == other_executable_id
                        && name == other_name
                        && ty_args == other_ty_args
                } else {
                    false
                }
            },
            Enum {
                executable_id,
                name,
                ty_args,
            } => {
                if let Enum {
                    executable_id: other_executable_id,
                    name: other_name,
                    ty_args: other_ty_args,
                } = other
                {
                    // SAFETY: Inner pointers are already canonical pointers,
                    // so it is safe to compare by pointer equality.
                    executable_id == other_executable_id
                        && name == other_name
                        && ty_args == other_ty_args
                } else {
                    false
                }
            },
            TypeParam { idx } => {
                if let TypeParam { idx: other_idx } = other {
                    idx == other_idx
                } else {
                    false
                }
            },
        }
    }
}

// PartialEq implementation above is a full equivalence relation.
impl Eq for TypeInternerKey {}

/// Wrapper around [`SignatureToken`] and owning [`CompiledModule`] that is
/// equivalent to [`TypeInternerKey`] and implements same hashing.
struct SignatureTokenKey<'a>(&'a SignatureToken, &'a CompiledModule);

impl Hash for SignatureTokenKey<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        use SignatureToken::*;

        // TODO: non-recursive implementation.

        match self.0 {
            Bool => {
                type_discriminant::BOOL.hash(state);
            },
            U8 => {
                type_discriminant::U8.hash(state);
            },
            U16 => {
                type_discriminant::U16.hash(state);
            },
            U32 => {
                type_discriminant::U32.hash(state);
            },
            U64 => {
                type_discriminant::U64.hash(state);
            },
            U128 => {
                type_discriminant::U128.hash(state);
            },
            U256 => {
                type_discriminant::U256.hash(state);
            },
            I8 => {
                type_discriminant::I8.hash(state);
            },
            I16 => {
                type_discriminant::I16.hash(state);
            },
            I32 => {
                type_discriminant::I32.hash(state);
            },
            I64 => {
                type_discriminant::I64.hash(state);
            },
            I128 => {
                type_discriminant::I128.hash(state);
            },
            I256 => {
                type_discriminant::I256.hash(state);
            },
            Address => {
                type_discriminant::ADDRESS.hash(state);
            },
            Signer => {
                type_discriminant::SIGNER.hash(state);
            },
            Reference(token) => {
                type_discriminant::REFERENCE.hash(state);
                Self(token.as_ref(), self.1).hash(state);
            },
            MutableReference(token) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                Self(token.as_ref(), self.1).hash(state);
            },
            Vector(elem_token) => {
                type_discriminant::VECTOR.hash(state);
                Self(elem_token.as_ref(), self.1).hash(state);
            },
            Struct(idx) => {
                hash_struct_signature_token(state, *idx, &[], self.1);
            },
            StructInstantiation(..) => todo!("Support generic types / type lists"),
            Function(..) => todo!("Support function types / type lists"),
            TypeParameter(idx) => {
                type_discriminant::TYPE_PARAM.hash(state);
                idx.hash(state);
            },
        }
    }
}

fn hash_struct_signature_token<H: Hasher>(
    state: &mut H,
    idx: StructHandleIndex,
    ty_args: &[SignatureToken],
    module: &CompiledModule,
) {
    type_discriminant::STRUCT.hash(state);
    let (address, module_name, struct_name) = struct_info_at(module, idx);
    address.hash(state);
    module_name.as_str().hash(state);
    struct_name.as_str().hash(state);
    ty_args.len().hash(state);
    for ty_arg in ty_args {
        SignatureTokenKey(ty_arg, module).hash(state);
    }
}

impl Equivalent<TypeInternerKey> for SignatureTokenKey<'_> {
    fn equivalent(&self, key: &TypeInternerKey) -> bool {
        use SignatureToken::*;

        // TODO: non-recursive implementation.

        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        let ty = unsafe { key.0.as_ref_unchecked() };

        match self.0 {
            Bool => matches!(ty, Type::Bool),
            U8 => matches!(ty, Type::U8),
            U16 => matches!(ty, Type::U16),
            U32 => matches!(ty, Type::U32),
            U64 => matches!(ty, Type::U64),
            U128 => matches!(ty, Type::U128),
            U256 => matches!(ty, Type::U256),
            I8 => matches!(ty, Type::I8),
            I16 => matches!(ty, Type::I16),
            I32 => matches!(ty, Type::I32),
            I64 => matches!(ty, Type::I64),
            I128 => matches!(ty, Type::I128),
            I256 => matches!(ty, Type::I256),
            Address => matches!(ty, Type::Address),
            Signer => matches!(ty, Type::Signer),
            Reference(inner) => {
                if let Type::ImmutRef { inner: other_inner } = ty {
                    Self(inner.as_ref(), self.1).equivalent(&TypeInternerKey(*other_inner))
                } else {
                    false
                }
            },
            MutableReference(inner) => {
                if let Type::MutRef { inner: other_inner } = ty {
                    Self(inner.as_ref(), self.1).equivalent(&TypeInternerKey(*other_inner))
                } else {
                    false
                }
            },
            Vector(elem) => {
                if let Type::Vector { elem: other_elem } = ty {
                    Self(elem.as_ref(), self.1).equivalent(&TypeInternerKey(*other_elem))
                } else {
                    false
                }
            },
            Struct(idx) => {
                // This signature token can refer to either a struct or an enum
                // in the binary format.
                let (other_executable_id, other_name, _other_ty_args) = match ty {
                    Type::Struct {
                        executable_id,
                        name,
                        ty_args,
                        ..
                    }
                    | Type::Enum {
                        executable_id,
                        name,
                        ty_args,
                    } => (executable_id, name, ty_args),
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
                    | Type::TypeParam { .. } => {
                        return false;
                    },
                };

                // SAFETY: It is safe to dereference pointers because the
                // caller ensures they remain valid during the lifetime of
                // the key.
                let other_executable_id = unsafe { other_executable_id.as_ref_unchecked() };
                let other_executable_name =
                    unsafe { other_executable_id.name().as_ref_unchecked() };
                let other_name = unsafe { other_name.as_ref_unchecked() };

                // TODO:
                //   Also compare type arguments structurally.

                let (address, module_name, struct_name) = struct_info_at(self.1, *idx);
                address == other_executable_id.address()
                    && module_name.as_str() == other_executable_name
                    && struct_name.as_str() == other_name
            },
            StructInstantiation(..) => todo!("Support generic types / type lists"),
            Function(..) => todo!("Support function types / type lists"),
            TypeParameter(idx) => {
                if let Type::TypeParam { idx: other_idx } = ty {
                    idx == other_idx
                } else {
                    false
                }
            },
        }
    }
}

/// Returns struct information (module address, name and struct name) per given
/// index. The index must come from the given compiled module.
pub(super) fn struct_info_at(
    module: &CompiledModule,
    idx: StructHandleIndex,
) -> (&AccountAddress, &IdentStr, &IdentStr) {
    let struct_handle = module.struct_handle_at(idx);
    let module_handle = module.module_handle_at(struct_handle.module);
    let address = module.address_identifier_at(module_handle.address);
    let module_name = module.identifier_at(module_handle.name);
    let struct_name = module.identifier_at(struct_handle.name);
    (address, module_name, struct_name)
}
