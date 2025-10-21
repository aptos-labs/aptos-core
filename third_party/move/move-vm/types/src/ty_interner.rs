// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Data structures and caches for interning types as unique compact identifiers. The lifetime of
//! these caches is tied to the code cache, and is managed externally.

use crate::loaded_data::{
    runtime_types::{Type, TypeBuilder},
    struct_name_indexing::StructNameIndex,
};
use move_binary_format::{
    errors::{PartialVMError, PartialVMResult},
    file_format::SignatureToken,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use triomphe::Arc;

/// Compactly represents a loaded type.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TypeId(u32);

/// Well-known types (primitives and more).
impl TypeId {
    pub const ADDRESS: TypeId = TypeId(13);
    pub const BOOL: TypeId = TypeId(0);
    pub const I128: TypeId = TypeId(11);
    pub const I16: TypeId = TypeId(8);
    pub const I256: TypeId = TypeId(12);
    pub const I32: TypeId = TypeId(9);
    pub const I64: TypeId = TypeId(10);
    pub const I8: TypeId = TypeId(7);
    pub const SIGNER: TypeId = TypeId(14);
    const TAG_BASE: u32 = 0 << Self::TAG_SHIFT;
    const TAG_MASK: u32 = 0b11 << Self::TAG_SHIFT;
    const TAG_MUT_REF: u32 = 2 << Self::TAG_SHIFT;
    const TAG_REF: u32 = 1 << Self::TAG_SHIFT;
    const TAG_SHIFT: u32 = 30;
    pub const U128: TypeId = TypeId(5);
    pub const U16: TypeId = TypeId(2);
    pub const U256: TypeId = TypeId(6);
    pub const U32: TypeId = TypeId(3);
    pub const U64: TypeId = TypeId(4);
    pub const U8: TypeId = TypeId(1);

    #[inline(always)]
    fn tag(self) -> u32 {
        self.0 & Self::TAG_MASK
    }

    #[inline]
    pub fn payload(self) -> TypeId {
        TypeId(self.0 & !Self::TAG_MASK)
    }

    #[inline]
    pub fn ref_of(inner: TypeId) -> TypeId {
        assert_eq!(inner.tag(), Self::TAG_BASE);
        TypeId(inner.0 | Self::TAG_REF)
    }

    #[inline]
    pub fn ref_mut_of(inner: TypeId) -> TypeId {
        assert_eq!(inner.tag(), Self::TAG_BASE);
        TypeId(inner.0 | Self::TAG_MUT_REF)
    }

    #[inline]
    pub fn is_ref(self) -> bool {
        self.tag() == Self::TAG_REF
    }

    #[inline]
    pub fn is_mut_ref(self) -> bool {
        self.tag() == Self::TAG_MUT_REF
    }

    #[inline]
    pub fn is_any_ref(self) -> bool {
        self.is_ref() || self.is_mut_ref()
    }
}

/// Compactly represents a vector of types.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TypeVecId(u32);

/// Partially-interned representation containing top-level information.
/// Abilities are cached for composite types (Vector, Struct, Function) to avoid recomputation.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum TypeRepr {
    // Primitive types (abilities are constant)
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

    // Composite types (abilities are cached)
    Vector {
        elem: TypeId,
        abilities: AbilitySet,
    },

    // References always have REFERENCES abilities
    Reference(TypeId),
    MutableReference(TypeId),

    Struct {
        idx: StructNameIndex,
        ty_args: TypeVecId,
        abilities: AbilitySet,
    },

    Function {
        args: TypeVecId,
        results: TypeVecId,
        abilities: AbilitySet,
    },
}

struct InternMap<T, I> {
    interned: HashMap<T, I>,
    data: Vec<T>,
}

impl<T, I> Default for InternMap<T, I> {
    fn default() -> Self {
        Self {
            interned: HashMap::new(),
            data: Vec::with_capacity(16),
        }
    }
}

impl<T, I> InternMap<T, I> {
    fn clear(&mut self) {
        self.interned.clear();
        self.data.clear();
    }
}

/// Interns single types.
struct TypeInterner {
    inner: RwLock<InternMap<TypeRepr, TypeId>>,
}

impl Default for TypeInterner {
    fn default() -> Self {
        Self {
            inner: RwLock::new(InternMap::default()),
        }
    }
}

impl TypeInterner {
    fn intern(&self, repr: TypeRepr) -> TypeId {
        if let Some(id) = self.inner.read().interned.get(&repr) {
            return *id;
        }

        let mut inner = self.inner.write();
        if let Some(id) = inner.interned.get(&repr) {
            return *id;
        }

        let id = TypeId(inner.data.len() as u32);
        inner.data.push(repr);
        inner.interned.insert(repr, id);
        id
    }

    fn warmup_if_empty(&self) {
        let mut inner = self.inner.write();
        if !inner.data.is_empty() {
            return;
        }

        // Pre-populate primitive types with well-known TypeIds.
        // The order must match the TypeId constants.
        let bool_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::Bool);
        inner.interned.insert(TypeRepr::Bool, bool_id);
        assert_eq!(bool_id, TypeId::BOOL);

        let u8_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::U8);
        inner.interned.insert(TypeRepr::U8, u8_id);
        assert_eq!(u8_id, TypeId::U8);

        let u16_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::U16);
        inner.interned.insert(TypeRepr::U16, u16_id);
        assert_eq!(u16_id, TypeId::U16);

        let u32_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::U32);
        inner.interned.insert(TypeRepr::U32, u32_id);
        assert_eq!(u32_id, TypeId::U32);

        let u64_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::U64);
        inner.interned.insert(TypeRepr::U64, u64_id);
        assert_eq!(u64_id, TypeId::U64);

        let u128_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::U128);
        inner.interned.insert(TypeRepr::U128, u128_id);
        assert_eq!(u128_id, TypeId::U128);

        let u256_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::U256);
        inner.interned.insert(TypeRepr::U256, u256_id);
        assert_eq!(u256_id, TypeId::U256);

        let i8_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::I8);
        inner.interned.insert(TypeRepr::I8, i8_id);
        assert_eq!(i8_id, TypeId::I8);

        let i16_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::I16);
        inner.interned.insert(TypeRepr::I16, i16_id);
        assert_eq!(i16_id, TypeId::I16);

        let i32_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::I32);
        inner.interned.insert(TypeRepr::I32, i32_id);
        assert_eq!(i32_id, TypeId::I32);

        let i64_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::I64);
        inner.interned.insert(TypeRepr::I64, i64_id);
        assert_eq!(i64_id, TypeId::I64);

        let i128_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::I128);
        inner.interned.insert(TypeRepr::I128, i128_id);
        assert_eq!(i128_id, TypeId::I128);

        let i256_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::I256);
        inner.interned.insert(TypeRepr::I256, i256_id);
        assert_eq!(i256_id, TypeId::I256);

        let address_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::Address);
        inner.interned.insert(TypeRepr::Address, address_id);
        assert_eq!(address_id, TypeId::ADDRESS);

        let signer_id = TypeId(inner.data.len() as u32);
        inner.data.push(TypeRepr::Signer);
        inner.interned.insert(TypeRepr::Signer, signer_id);
        assert_eq!(signer_id, TypeId::SIGNER);
    }
}

/// Interns vector of types (e.g., list of type arguments).
struct TypeVecInterner {
    inner: RwLock<InternMap<Arc<[TypeId]>, TypeVecId>>,
}

impl Default for TypeVecInterner {
    fn default() -> Self {
        Self {
            inner: RwLock::new(InternMap::default()),
        }
    }
}

impl TypeVecInterner {
    fn intern(&self, tys: &[TypeId]) -> TypeVecId {
        if let Some(id) = self.inner.read().interned.get(tys) {
            return *id;
        }

        let tys_arced: Arc<[TypeId]> = Arc::from(tys);
        let tys_arced_key = tys_arced.clone();

        let mut inner = self.inner.write();
        if let Some(id) = inner.interned.get(tys) {
            return *id;
        }

        let id = TypeVecId(inner.data.len() as u32);
        inner.data.push(tys_arced);
        inner.interned.insert(tys_arced_key, id);
        id
    }

    fn intern_vec(&self, tys: Vec<TypeId>) -> TypeVecId {
        if let Some(id) = self.inner.read().interned.get(tys.as_slice()) {
            return *id;
        }

        let tys: Arc<[TypeId]> = tys.into();
        let tys_key = tys.clone();

        let mut inner = self.inner.write();
        if let Some(id) = inner.interned.get(&tys) {
            return *id;
        }

        let id = TypeVecId(inner.data.len() as u32);
        inner.data.push(tys);
        inner.interned.insert(tys_key, id);
        id
    }
}

/// Pool of all interned types. Users can query interned representations ([TypeId] for single types
/// or [TypeVecId] for vector of types) based on provided runtime types. Context does not manage
/// memory nor limit the number of types to intern - this has to be managed externally by the
/// client (to ensure eviction of interned data is safe).
pub struct InternedTypePool {
    ty_interner: TypeInterner,
    ty_vec_interner: TypeVecInterner,
}

impl InternedTypePool {
    /// Creates a new interning context. Context is warmed-up with common types.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let ty_interner = TypeInterner::default();
        let ty_vec_interner = TypeVecInterner::default();
        let ctx = Self {
            ty_interner,
            ty_vec_interner,
        };
        ctx.warmup();
        ctx
    }

    /// Returns how many distinct types are instantiated.
    pub fn num_interned_tys(&self) -> usize {
        self.ty_interner.inner.read().data.len()
    }

    /// Returns how many distinct vectors of types are instantiated.
    pub fn num_interned_ty_vecs(&self) -> usize {
        self.ty_vec_interner.inner.read().data.len()
    }

    /// Clears all interned data, and then warm-ups the cache for common types. Should be called if
    /// type IDs are no longer used, e.g., when flushing module cache at block boundaries.
    pub fn flush(&self) {
        self.flush_impl();
        self.warmup();
    }

    /// Flushes all cached data without warming up the cache.
    fn flush_impl(&self) {
        let mut ty_interner = self.ty_interner.inner.write();
        ty_interner.clear();
        drop(ty_interner);

        let mut ty_vec_interner = self.ty_vec_interner.inner.write();
        ty_vec_interner.clear();
        drop(ty_vec_interner);
    }

    /// Interns common type representations.
    fn warmup(&self) {
        self.ty_interner.warmup_if_empty();

        self.ty_vec_interner.intern(&[]);
        self.ty_vec_interner.intern(&[TypeId::U8]);
        self.ty_vec_interner.intern(&[TypeId::U64]);
    }

    /// Converts a slice of fully-instantiated Type arguments to a Vec of TypeIds.
    /// Does not intern the vector itself.
    ///
    /// Panics if there are non-instantiated type arguments.
    pub fn ty_args_to_ty_ids(&self, ty_args: &[Type]) -> Vec<TypeId> {
        ty_args
            .iter()
            .map(|t| self.instantiate_and_intern(t, &[]))
            .collect()
    }

    /// Given a vector if fully-instantiated type arguments, returns the corresponding [TypeVecId].
    ///
    /// Panics if there are non-instantiated type arguments.
    pub fn intern_ty_args(&self, ty_args: &[Type]) -> TypeVecId {
        let ty_arg_ids = self.ty_args_to_ty_ids(ty_args);
        self.ty_vec_interner.intern_vec(ty_arg_ids)
    }

    // TODO: check bound at load-time.
    pub fn create_constant_ty(&self, constant_tok: &SignatureToken) -> TypeId {
        use SignatureToken as S;

        match constant_tok {
            S::Bool => self.bool_ty(),
            S::U8 => self.u8_ty(),
            S::U16 => self.u16_ty(),
            S::U32 => self.u32_ty(),
            S::U64 => self.u64_ty(),
            S::U128 => self.u128_ty(),
            S::U256 => self.u256_ty(),
            S::I8 => self.i8_ty(),
            S::I16 => self.i16_ty(),
            S::I32 => self.i32_ty(),
            S::I64 => self.i64_ty(),
            S::I128 => self.i128_ty(),
            S::I256 => self.i256_ty(),
            S::Address => self.address_ty(),
            S::Vector(elem_tok) => {
                // TODO: optimize
                let elem_ty = TypeBuilder::with_limits(100, 100)
                    .create_constant_ty(elem_tok)
                    .unwrap();
                let elem = self.instantiate_and_intern(&elem_ty, &[]);
                let abilities =
                    AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                        self.abilities(elem)
                    ])
                    .expect("Vector ability computation should not fail");
                self.ty_interner
                    .intern(TypeRepr::Vector { elem, abilities })
            },
            S::Signer
            | S::Struct(_)
            | S::StructInstantiation(_, _)
            | S::Function(..)
            | S::Reference(_)
            | S::MutableReference(_)
            | S::TypeParameter(_) => unreachable!("Must be verified at load-time."),
        }
    }

    /// Given a type containing type parameters, and a fully-interned type arguments, performs
    /// type substitution with interning. Abilities are computed from the input Type and cached
    /// in the interned representation.
    ///
    /// # Panics
    /// Panics if ability computation fails (indicates malformed type).
    pub fn instantiate_and_intern(&self, ty: &Type, subst: &[TypeId]) -> TypeId {
        use Type::*;
        match ty {
            // Fast path: return well-known constants for primitives
            Bool => TypeId::BOOL,
            U8 => TypeId::U8,
            U16 => TypeId::U16,
            U32 => TypeId::U32,
            U64 => TypeId::U64,
            U128 => TypeId::U128,
            U256 => TypeId::U256,
            I8 => TypeId::I8,
            I16 => TypeId::I16,
            I32 => TypeId::I32,
            I64 => TypeId::I64,
            I128 => TypeId::I128,
            I256 => TypeId::I256,
            Address => TypeId::ADDRESS,
            Signer => TypeId::SIGNER,
            TyParam(idx) => subst[*idx as usize],
            Vector(elem_ty) => {
                let elem_id = self.instantiate_and_intern(elem_ty, subst);
                // Compute abilities from the substituted element type
                let elem_abilities = self.abilities(elem_id);
                let abilities =
                    AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                        elem_abilities,
                    ])
                    .expect("Vector ability computation should not fail");
                self.ty_interner.intern(TypeRepr::Vector {
                    elem: elem_id,
                    abilities,
                })
            },
            Reference(inner_ty) => {
                let inner_id = self.instantiate_and_intern(inner_ty, subst);
                TypeId::ref_of(inner_id)
            },
            MutableReference(inner_ty) => {
                let inner_id = self.instantiate_and_intern(inner_ty, subst);
                TypeId::ref_mut_of(inner_id)
            },
            Struct { idx, ability } => {
                // Get abilities from the struct's ability info
                let abilities = ability.base_ability_set;
                self.ty_interner.intern(TypeRepr::Struct {
                    idx: *idx,
                    ty_args: self.ty_vec_interner.intern(&[]),
                    abilities,
                })
            },
            StructInstantiation {
                idx,
                ty_args,
                ability,
            } => {
                let ty_arg_ids = ty_args
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                // Compute abilities from the substituted type arguments
                let type_argument_abilities = ty_arg_ids
                    .iter()
                    .map(|&ty_id| self.abilities(ty_id))
                    .collect::<Vec<_>>();
                let abilities = AbilitySet::polymorphic_abilities(
                    ability.base_ability_set,
                    ability.phantom_ty_args_mask.iter(),
                    type_argument_abilities,
                )
                .expect("StructInstantiation ability computation should not fail");
                self.ty_interner.intern(TypeRepr::Struct {
                    idx: *idx,
                    ty_args: self.ty_vec_interner.intern_vec(ty_arg_ids),
                    abilities,
                })
            },
            Function {
                args,
                results,
                abilities,
            } => {
                let arg_ids = args
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                let result_ids = results
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                self.ty_interner.intern(TypeRepr::Function {
                    args: self.ty_vec_interner.intern_vec(arg_ids),
                    results: self.ty_vec_interner.intern_vec(result_ids),
                    abilities: *abilities,
                })
            },
        }
    }

    // ===== Type Construction APIs =====

    /// Creates a vector type with the given element type.
    /// Returns the TypeId of the vector type.
    /// Abilities are computed based on the element type's abilities.
    #[inline]
    pub fn vec_of(&self, elem: TypeId) -> TypeId {
        // Get element abilities
        let elem_abilities = self.abilities(elem);

        // Compute vector abilities using AbilitySet::polymorphic_abilities
        // Vector's type parameter is not phantom, so we pass false
        let abilities = AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
            elem_abilities,
        ])
        .expect("Vector ability computation should not fail");

        self.ty_interner
            .intern(TypeRepr::Vector { elem, abilities })
    }

    // ===== Type Query APIs =====

    /// Gets the TypeRepr for a given TypeId.
    /// This is a lower-level API that other methods build upon.
    ///
    /// # Panics
    /// Panics if the TypeId is invalid (not created through this pool).
    #[inline]
    pub fn type_repr(&self, ty: TypeId) -> TypeRepr {
        if ty.is_ref() {
            return TypeRepr::Reference(ty.payload());
        }
        if ty.is_mut_ref() {
            return TypeRepr::MutableReference(ty.payload());
        }

        let inner = self.ty_interner.inner.read();
        // SAFETY: TypeId is only created through interning, so the index is always valid
        inner.data[ty.0 as usize]
    }

    /// Gets the element type of a vector.
    /// Returns None if the type is not a vector.
    #[inline]
    pub fn get_vec_elem_ty(&self, ty: TypeId) -> Option<TypeId> {
        match self.type_repr(ty) {
            TypeRepr::Vector { elem, .. } => Some(elem),
            _ => None,
        }
    }

    /// Checks if a type is a vector.
    #[inline]
    pub fn is_vec(&self, ty: TypeId) -> bool {
        matches!(self.type_repr(ty), TypeRepr::Vector { .. })
    }

    /// Gets the types from a TypeVecId.
    /// Returns a slice of TypeIds.
    ///
    /// # Panics
    /// Panics if the TypeVecId is invalid (not created through this pool).
    #[inline]
    pub fn get_type_vec(&self, id: TypeVecId) -> Arc<[TypeId]> {
        let inner = self.ty_vec_interner.inner.read();
        inner.data[id.0 as usize].clone()
    }

    // ===== Ability Query APIs =====

    /// Returns the abilities for a type.
    /// Abilities are pre-computed during type interning, so this is a simple lookup.
    /// Optimized for primitives using direct TypeId comparisons.
    #[inline]
    pub fn abilities(&self, ty: TypeId) -> AbilitySet {
        // Fast path for primitives using well-known TypeIds
        // All primitives except Signer are consecutive (BOOL=0 to ADDRESS=13)
        if ty >= TypeId::BOOL && ty <= TypeId::ADDRESS {
            return AbilitySet::PRIMITIVES;
        }
        if ty == TypeId::SIGNER {
            return AbilitySet::SIGNER;
        }

        if ty.is_ref() || ty.is_mut_ref() {
            return AbilitySet::REFERENCES;
        }

        // Slow path for composite types - need to fetch TypeRepr
        match self.type_repr(ty) {
            // Composite types: abilities are cached
            TypeRepr::Vector { abilities, .. } => abilities,
            TypeRepr::Struct { abilities, .. } => abilities,
            TypeRepr::Function { abilities, .. } => abilities,
            // Primitives are handled above
            _ => unreachable!("All primitive types should be handled by fast path"),
        }
    }

    /// Checks if a type has a specific ability.
    /// This is a convenience wrapper around `abilities()`.
    #[inline]
    pub fn has_ability(&self, ty: TypeId, ability: Ability) -> bool {
        self.abilities(ty).has_ability(ability)
    }

    // ===== Primitive Type Creation Methods =====

    /// Returns TypeId for Bool primitive type.
    #[inline]
    pub fn bool_ty(&self) -> TypeId {
        TypeId::BOOL
    }

    /// Returns TypeId for U8 primitive type.
    #[inline]
    pub fn u8_ty(&self) -> TypeId {
        TypeId::U8
    }

    /// Returns TypeId for U16 primitive type.
    #[inline]
    pub fn u16_ty(&self) -> TypeId {
        TypeId::U16
    }

    /// Returns TypeId for U32 primitive type.
    #[inline]
    pub fn u32_ty(&self) -> TypeId {
        TypeId::U32
    }

    /// Returns TypeId for U64 primitive type.
    #[inline]
    pub fn u64_ty(&self) -> TypeId {
        TypeId::U64
    }

    /// Returns TypeId for U128 primitive type.
    #[inline]
    pub fn u128_ty(&self) -> TypeId {
        TypeId::U128
    }

    /// Returns TypeId for U256 primitive type.
    #[inline]
    pub fn u256_ty(&self) -> TypeId {
        TypeId::U256
    }

    /// Returns TypeId for I8 primitive type.
    #[inline]
    pub fn i8_ty(&self) -> TypeId {
        TypeId::I8
    }

    /// Returns TypeId for I16 primitive type.
    #[inline]
    pub fn i16_ty(&self) -> TypeId {
        TypeId::I16
    }

    /// Returns TypeId for I32 primitive type.
    #[inline]
    pub fn i32_ty(&self) -> TypeId {
        TypeId::I32
    }

    /// Returns TypeId for I64 primitive type.
    #[inline]
    pub fn i64_ty(&self) -> TypeId {
        TypeId::I64
    }

    /// Returns TypeId for I128 primitive type.
    #[inline]
    pub fn i128_ty(&self) -> TypeId {
        TypeId::I128
    }

    /// Returns TypeId for I256 primitive type.
    #[inline]
    pub fn i256_ty(&self) -> TypeId {
        TypeId::I256
    }

    /// Returns TypeId for Address primitive type.
    #[inline]
    pub fn address_ty(&self) -> TypeId {
        TypeId::ADDRESS
    }

    /// Returns TypeId for Signer primitive type.
    #[inline]
    pub fn signer_ty(&self) -> TypeId {
        TypeId::SIGNER
    }

    /// Creates a Function type with given arguments, results, and abilities (slice version).
    #[inline]
    pub fn function_of(
        &self,
        args: &[TypeId],
        results: &[TypeId],
        abilities: AbilitySet,
    ) -> TypeId {
        let args_id = self.ty_vec_interner.intern(args);
        let results_id = self.ty_vec_interner.intern(results);
        self.ty_interner.intern(TypeRepr::Function {
            args: args_id,
            results: results_id,
            abilities,
        })
    }

    /// Creates a Function type with given arguments, results, and abilities (Vec version).
    #[inline]
    pub fn function_of_vec(
        &self,
        args: Vec<TypeId>,
        results: Vec<TypeId>,
        abilities: AbilitySet,
    ) -> TypeId {
        let args_id = self.ty_vec_interner.intern_vec(args);
        let results_id = self.ty_vec_interner.intern_vec(results);
        self.ty_interner.intern(TypeRepr::Function {
            args: args_id,
            results: results_id,
            abilities,
        })
    }

    // ===== Paranoid Type Check Methods (mirror Type paranoid methods) =====

    /// Paranoid check: verify type has a required ability.
    #[inline]
    pub fn paranoid_check_has_ability(&self, ty: TypeId, ability: Ability) -> PartialVMResult<()> {
        if !self.has_ability(ty, ability) {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: type {:?} missing required ability {:?}",
                self.type_repr(ty),
                ability
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type has all required abilities.
    #[inline]
    pub fn paranoid_check_abilities(
        &self,
        ty: TypeId,
        required: AbilitySet,
    ) -> PartialVMResult<()> {
        let ty_abilities = self.abilities(ty);
        if !required.is_subset(ty_abilities) {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: type {:?} missing required abilities {:?}",
                self.type_repr(ty),
                required
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type equality.
    #[inline]
    pub fn paranoid_check_eq(&self, ty: TypeId, expected: TypeId) -> PartialVMResult<()> {
        if ty != expected {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: type mismatch, expected {:?} but got {:?}",
                expected, ty
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type assignability (for functions and references).
    #[inline]
    pub fn paranoid_check_assignable(
        &self,
        given: TypeId,
        expected: TypeId,
    ) -> PartialVMResult<()> {
        let ok = match (self.type_repr(expected), self.type_repr(given)) {
            (
                TypeRepr::Function {
                    args: expected_args,
                    results: expected_results,
                    abilities: expected_abilities,
                },
                TypeRepr::Function {
                    args: given_args,
                    results: given_results,
                    abilities: given_abilities,
                },
            ) => {
                expected_args == given_args
                    && expected_results == given_results
                    && expected_abilities.is_subset(given_abilities)
            },
            (TypeRepr::Reference(expected_inner), TypeRepr::Reference(given_inner)) => {
                self.paranoid_check_assignable(given_inner, expected_inner)?;
                true
            },
            _ => expected == given,
        };

        if !ok {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: type mismatch, expected {:?} but got {:?}",
                self.type_repr(expected),
                self.type_repr(given)
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify that ty is a reference (mutable or immutable based on is_mut)
    /// to the expected_inner_ty.
    /// Matches the semantics of Type::paranoid_check_ref_eq.
    #[inline]
    pub fn paranoid_check_ref_eq(
        &self,
        ty: TypeId,
        expected_inner_ty: TypeId,
        is_mut: bool,
    ) -> PartialVMResult<()> {
        match self.type_repr(ty) {
            TypeRepr::MutableReference(inner) => {
                self.paranoid_check_eq(inner, expected_inner_ty)?;
                Ok(())
            },
            TypeRepr::Reference(inner) if !is_mut => {
                self.paranoid_check_eq(inner, expected_inner_ty)?;
                Ok(())
            },
            _ => Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected a (mutable: {}) reference type, got {:?}",
                is_mut,
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE)),
        }
    }

    /// Paranoid check: verify type is not a reference.
    #[inline]
    pub fn paranoid_check_is_no_ref(&self, ty: TypeId, msg: &str) -> PartialVMResult<()> {
        match self.type_repr(ty) {
            TypeRepr::Reference(_) | TypeRepr::MutableReference(_) => Err(
                PartialVMError::new_invariant_violation(format!("{}: type is a reference", msg))
                    .with_sub_status(EPARANOID_FAILURE),
            ),
            _ => Ok(()),
        }
    }

    /// Paranoid check: verify type is bool.
    #[inline]
    pub fn paranoid_check_is_bool_ty(&self, ty: TypeId) -> PartialVMResult<()> {
        if ty != TypeId::BOOL {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected bool type, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type is signed integer.
    /// Uses range check since all signed integers are consecutive (I8=7 to I256=12).
    #[inline]
    pub fn paranoid_check_is_sint_ty(&self, ty: TypeId) -> PartialVMResult<()> {
        if ty < TypeId::I8 || ty > TypeId::I256 {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected signed integer type, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type is u64.
    #[inline]
    pub fn paranoid_check_is_u64_ty(&self, ty: TypeId) -> PartialVMResult<()> {
        if ty != TypeId::U64 {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected u64 type, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type is address.
    #[inline]
    pub fn paranoid_check_is_address_ty(&self, ty: TypeId) -> PartialVMResult<()> {
        if ty != TypeId::ADDRESS {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected address type, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE));
        }
        Ok(())
    }

    /// Paranoid check: verify type is a reference to signer.
    #[inline]
    pub fn paranoid_check_is_signer_ref_ty(&self, ty: TypeId) -> PartialVMResult<()> {
        match self.type_repr(ty) {
            TypeRepr::Reference(inner) if inner == TypeId::SIGNER => Ok(()),
            _ => Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected signer reference type, got {:?}",
                ty
            ))
            .with_sub_status(EPARANOID_FAILURE)),
        }
    }

    /// Paranoid check: verify type is a vector with expected element type.
    #[inline]
    pub fn paranoid_check_is_vec_ty(
        &self,
        ty: TypeId,
        expected_elem: TypeId,
    ) -> PartialVMResult<()> {
        match self.type_repr(ty) {
            TypeRepr::Vector { elem, .. } if elem == expected_elem => Ok(()),
            _ => Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected vector<{:?}> type, got {:?}",
                self.type_repr(expected_elem),
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE)),
        }
    }

    /// Paranoid check: verify type is a reference to vector with expected element type.
    /// A mutable reference is always acceptable (can be used for both mutable and immutable operations).
    /// An immutable reference is only acceptable when IS_MUT is false.
    #[inline]
    pub fn paranoid_check_is_vec_ref_ty<const IS_MUT: bool>(
        &self,
        ty: TypeId,
        expected_elem: TypeId,
    ) -> PartialVMResult<()> {
        // Check for mutable reference to vector - always OK
        if ty.is_mut_ref() {
            return self.paranoid_check_is_vec_ty(ty.payload(), expected_elem);
        }

        // Check for immutable reference to vector - only OK if IS_MUT is false
        if ty.is_ref() {
            if !IS_MUT {
                return self.paranoid_check_is_vec_ty(ty.payload(), expected_elem);
            }
        }

        Err(PartialVMError::new_invariant_violation(format!(
            "Paranoid mode: expected a (mutable: {}) vector reference, got {:?}",
            IS_MUT,
            self.type_repr(ty)
        ))
        .with_sub_status(EPARANOID_FAILURE))
    }

    /// Paranoid check: read from a reference, returning the inner type.
    /// Checks that the inner type has Copy ability.
    #[inline]
    pub fn paranoid_read_ref(&self, ty: TypeId) -> PartialVMResult<TypeId> {
        if ty.is_any_ref() {
            self.paranoid_check_has_ability(ty.payload(), Ability::Copy)?;
            Ok(ty.payload())
        } else {
            Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected reference type for ReadRef, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE))
        }
    }

    /// Paranoid check: write to a mutable reference.
    #[inline]
    pub fn paranoid_write_ref(&self, ty: TypeId, val_ty: TypeId) -> PartialVMResult<()> {
        if ty.is_mut_ref() {
            self.paranoid_check_assignable(val_ty, ty.payload())?;
            self.paranoid_check_has_ability(ty.payload(), Ability::Drop)?;
            Ok(())
        } else {
            Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected mutable reference type for WriteRef, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE))
        }
    }

    /// Paranoid check: freeze a mutable reference to an immutable reference.
    #[inline]
    pub fn paranoid_freeze_ref_ty(&self, ty: TypeId) -> PartialVMResult<TypeId> {
        if ty.is_mut_ref() {
            Ok(TypeId::ref_of(ty.payload()))
        } else {
            Err(PartialVMError::new_invariant_violation(format!(
                "Paranoid mode: expected mutable reference type for FreezeRef, got {:?}",
                self.type_repr(ty)
            ))
            .with_sub_status(EPARANOID_FAILURE))
        }
    }

    /// Paranoid check: verify vector and get element reference type.
    #[inline]
    pub fn paranoid_check_and_get_vec_elem_ref_ty<const IS_MUT: bool>(
        &self,
        vec_ref_ty: TypeId,
        expected_elem: TypeId,
    ) -> PartialVMResult<TypeId> {
        self.paranoid_check_is_vec_ref_ty::<IS_MUT>(vec_ref_ty, expected_elem)?;
        if IS_MUT {
            Ok(TypeId::ref_mut_of(expected_elem))
        } else {
            Ok(TypeId::ref_of(expected_elem))
        }
    }

    /// Paranoid check: verify mutable vector reference and get element type.
    #[inline]
    pub fn paranoid_check_and_get_vec_elem_ty<const IS_MUT: bool>(
        &self,
        vec_ref_ty: TypeId,
        expected_elem: TypeId,
    ) -> PartialVMResult<TypeId> {
        self.paranoid_check_is_vec_ref_ty::<IS_MUT>(vec_ref_ty, expected_elem)?;
        Ok(expected_elem)
    }
}
