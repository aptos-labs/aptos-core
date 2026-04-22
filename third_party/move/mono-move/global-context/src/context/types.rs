// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type interning infrastructure.
//!
//! The pure type model ([`Type`], [`FieldLayout`], [`StructLayout`], etc.)
//! lives in `mono_move_core::types`. This module provides the interning
//! machinery that deduplicates types in the global arena, plus cross-format
//! hashing/equality between [`SignatureToken`]s and interned [`Type`]s.

// Re-export the pure type model from mono-move-core so existing consumers
// within this crate can continue to use `crate::context::types::Type` etc.
use crate::ExecutionGuard;
use dashmap::Equivalent;
pub use mono_move_core::types::*;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};
use std::hash::{Hash, Hasher};

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
    ) -> Option<InternedType> {
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
    pub(super) fn insert_allocated_type_pointer_internal(&self, ptr: InternedType) -> InternedType {
        // SAFETY: We have just allocated the pointer, hence it is safe to wrap
        // it as a key and compute hash / equality. All existing keys are also
        // valid pointers because the map is cleared on arena's reset.
        *self.ctx.types.entry(TypeInternerKey(ptr)).or_insert(ptr)
    }

    /// Inserts the newly allocated type list pointer into the deduplication
    /// map. If the entry exists, the allocated pointer is discarded and the
    /// existing pointer is returned. If the entry does not exist, the
    /// allocated pointer is inserted and its copy returned.
    ///
    /// # Safety
    ///
    ///   1. The caller must ensure that the inserted pointer is alive.
    ///   2. All inner type pointers must be canonical (previously interned).
    ///   3. For any pointer that exists in the map, it must be still alive.
    pub(super) fn insert_allocated_type_list_internal(
        &self,
        ptr: InternedTypeList,
    ) -> InternedTypeList {
        // SAFETY: We have just allocated the pointer, hence it is safe to wrap
        // it as a key and compute hash / equality. All existing keys are also
        // valid pointers because the map is cleared on arena's reset.
        *self
            .ctx
            .type_lists
            .entry(TypeListInternerKey(ptr))
            .or_insert(ptr)
    }
}

/// Returns a static primitive type pointer for primitive signature tokens,
/// or [`None`] for composite types that require arena allocation.
pub fn try_as_primitive_type(token: &SignatureToken) -> Option<InternedType> {
    use SignatureToken as S;

    match token {
        S::Bool => Some(BOOL_TY),
        S::U8 => Some(U8_TY),
        S::U16 => Some(U16_TY),
        S::U32 => Some(U32_TY),
        S::U64 => Some(U64_TY),
        S::U128 => Some(U128_TY),
        S::U256 => Some(U256_TY),
        S::I8 => Some(I8_TY),
        S::I16 => Some(I16_TY),
        S::I32 => Some(I32_TY),
        S::I64 => Some(I64_TY),
        S::I128 => Some(I128_TY),
        S::I256 => Some(I256_TY),
        S::Address => Some(ADDRESS_TY),
        S::Signer => Some(SIGNER_TY),
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
    pub(super) const FUNCTION: u8 = 19;
    pub(super) const TYPE_PARAM: u8 = 20;
}

/// Wraps allocated type pointer to implement structural hash and equality.
///
/// # Safety
///
/// Constructor must enforce the pointer points to the valid data and can be
/// safely dereferenced.
pub(super) struct TypeInternerKey(InternedType);

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
                ty_args,
                layout: _,
            }
            | Enum {
                executable_id,
                name,
                ty_args,
            } => {
                // SAFETY: It is safe to dereference pointers because the
                // caller ensures they remain valid during the lifetime of
                // the key.
                let executable_id = unsafe { executable_id.as_ref_unchecked() };
                let executable_name = unsafe { executable_id.name().as_ref_unchecked() };
                let name = unsafe { name.as_ref_unchecked() };
                let ty_args = unsafe { ty_args.as_ref_unchecked() };

                // Must use structural hash because it is compared against the
                // hash of lookup key (e.g., signature token). Enums reuse the
                // same discriminant as structs because type identity is based
                // on address, executable name, name and type arguments.
                type_discriminant::STRUCT.hash(state);
                executable_id.address().hash(state);
                executable_name.hash(state);
                name.hash(state);
                ty_args.len().hash(state);
                for ty_arg in ty_args {
                    Self(*ty_arg).hash(state);
                }
            },
            Function {
                args,
                results,
                abilities,
            } => {
                type_discriminant::FUNCTION.hash(state);
                // SAFETY: It is safe to dereference pointers because the
                // caller ensures they remain valid during the lifetime of
                // the key.
                let args = unsafe { args.as_ref_unchecked() };
                let results = unsafe { results.as_ref_unchecked() };

                args.len().hash(state);
                for arg in args {
                    Self(*arg).hash(state);
                }
                results.len().hash(state);
                for result in results {
                    Self(*result).hash(state);
                }
                abilities.hash(state);
            },
            TypeParam { idx } => {
                type_discriminant::TYPE_PARAM.hash(state);
                idx.hash(state);
            },
        }
    }
}

// Inner `InternedType` fields are compared via `==`, which dispatches to
// `GlobalArenaPtr::PartialEq` (pointer equality). This is sound because the
// interner only stores canonical pointers — structurally equal inner types are
// deduplicated at insertion, so pointer equality coincides with structural
// equality.
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
                    *inner == *other_inner
                } else {
                    false
                }
            },
            MutRef { inner } => {
                if let MutRef { inner: other_inner } = other {
                    *inner == *other_inner
                } else {
                    false
                }
            },
            Vector { elem } => {
                if let Vector { elem: other_elem } = other {
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
                    executable_id == other_executable_id
                        && name == other_name
                        && ty_args == other_ty_args
                } else {
                    false
                }
            },
            Function {
                args,
                results,
                abilities,
            } => {
                if let Function {
                    args: other_args,
                    results: other_results,
                    abilities: other_abilities,
                } = other
                {
                    args == other_args && results == other_results && abilities == other_abilities
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
            StructInstantiation(idx, ty_args) => {
                hash_struct_signature_token(state, *idx, ty_args, self.1);
            },
            Function(args, results, abilities) => {
                type_discriminant::FUNCTION.hash(state);
                args.len().hash(state);
                for arg in args {
                    Self(arg, self.1).hash(state);
                }
                results.len().hash(state);
                for result in results {
                    Self(result, self.1).hash(state);
                }
                abilities.hash(state);
            },
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

/// Returns true if [`Type`] is equivalent to a [`SignatureToken`] struct or
/// an enum (identified by handle index and type arguments).
///
/// # Safety
///
/// All pointers inside the interned type must be safe to dereference.
fn equivalent_struct_types(
    ty: &Type,
    idx: StructHandleIndex,
    ty_args: &[SignatureToken],
    module: &CompiledModule,
) -> bool {
    let (other_executable_id, other_name, other_ty_args) = match ty {
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
        | Type::Function { .. }
        | Type::TypeParam { .. } => {
            return false;
        },
    };

    // SAFETY: It is safe to dereference pointers because the caller ensures
    // they remain valid during the lifetime of the key.
    let other_executable_id = unsafe { other_executable_id.as_ref_unchecked() };
    let other_executable_name = unsafe { other_executable_id.name().as_ref_unchecked() };
    let other_name = unsafe { other_name.as_ref_unchecked() };
    let other_ty_args = unsafe { other_ty_args.as_ref_unchecked() };

    let (address, module_name, struct_name) = struct_info_at(module, idx);
    address == other_executable_id.address()
        && module_name.as_str() == other_executable_name
        && struct_name.as_str() == other_name
        && ty_args.len() == other_ty_args.len()
        && ty_args
            .iter()
            .zip(other_ty_args.iter())
            .all(|(ty_arg, other_ty_arg)| {
                SignatureTokenKey(ty_arg, module).equivalent(&TypeInternerKey(*other_ty_arg))
            })
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
            Struct(idx) => equivalent_struct_types(ty, *idx, &[], self.1),
            StructInstantiation(idx, ty_args) => equivalent_struct_types(ty, *idx, ty_args, self.1),
            Function(args, results, abilities) => {
                if let Type::Function {
                    args: other_args,
                    results: other_results,
                    abilities: other_abilities,
                } = ty
                {
                    // SAFETY: It is safe to dereference pointers because the
                    // caller ensures they remain valid during the lifetime of
                    // the key.
                    let other_args = unsafe { other_args.as_ref_unchecked() };
                    let other_results = unsafe { other_results.as_ref_unchecked() };

                    if args.len() != other_args.len()
                        || results.len() != other_results.len()
                        || abilities != other_abilities
                    {
                        return false;
                    }

                    args.iter()
                        .zip(other_args.iter())
                        .chain(results.iter().zip(other_results.iter()))
                        .all(|(tok, other_ty)| {
                            Self(tok, self.1).equivalent(&TypeInternerKey(*other_ty))
                        })
                } else {
                    false
                }
            },
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
pub fn struct_info_at(
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

/// Wraps allocated type list pointer to implement structural hash and
/// equality.
///
/// # Safety
///
/// Constructor must enforce the pointer points to the valid data and can be
/// safely dereferenced.
pub(super) struct TypeListInternerKey(InternedTypeList);

impl Hash for TypeListInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        let tys = unsafe { self.0.as_ref_unchecked() };
        tys.len().hash(state);
        for ty_ptr in tys {
            TypeInternerKey(*ty_ptr).hash(state);
        }
    }
}

impl PartialEq for TypeListInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: It is safe to dereference the pointer because the caller
        // ensures it remains valid during the lifetime of the key.
        let this = unsafe { self.0.as_ref_unchecked() };
        let other = unsafe { other.0.as_ref_unchecked() };

        if this.len() != other.len() {
            return false;
        }
        this.iter().zip(other.iter()).all(|(ty, other_ty)| {
            // SAFETY: These pointers are already canonical, so using pointer
            // equality is sufficient.
            ty == other_ty
        })
    }
}

// PartialEq implementation above is a full equivalence relation.
impl Eq for TypeListInternerKey {}
