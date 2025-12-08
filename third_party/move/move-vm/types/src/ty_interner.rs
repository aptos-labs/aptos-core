// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Data structures and caches for interning types as unique compact identifiers. The lifetime of
//! these caches is tied to the code cache, and is managed externally.

use crate::loaded_data::{
    runtime_types::{StructType, Type, TypeBuilder},
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
use std::{cell::UnsafeCell, collections::HashMap};
use triomphe::Arc;

/// Compactly represents a loaded type.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TypeId(u32);

/// Known primitive types.
#[rustfmt::skip]
impl TypeId {
    pub const BOOL: TypeId = TypeId(0);
    pub const U8: TypeId = TypeId(1);
    pub const U16: TypeId = TypeId(2);
    pub const U32: TypeId = TypeId(3);
    pub const U64: TypeId = TypeId(4);
    pub const U128: TypeId = TypeId(5);
    pub const U256: TypeId = TypeId(6);
    pub const I8: TypeId = TypeId(7);
    pub const I16: TypeId = TypeId(8);
    pub const I32: TypeId = TypeId(9);
    pub const I64: TypeId = TypeId(10);
    pub const I128: TypeId = TypeId(11);
    pub const I256: TypeId = TypeId(12);
    pub const ADDRESS: TypeId = TypeId(13);
    pub const SIGNER: TypeId = TypeId(14);
}

// For dealing with references.
impl TypeId {
    const TAG_BASE: u32 = 0 << Self::TAG_SHIFT;
    const TAG_MASK: u32 = 0b11 << Self::TAG_SHIFT;
    const TAG_MUT_REF: u32 = 2 << Self::TAG_SHIFT;
    const TAG_REF: u32 = 1 << Self::TAG_SHIFT;
    const TAG_SHIFT: u32 = 30;

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
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum TypeRepr {
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
    Vector(TypeId),
    Reference(TypeId),
    MutableReference(TypeId),
    Struct {
        idx: StructNameIndex,
        ty_args: TypeVecId,
    },
    Function {
        args: TypeVecId,
        results: TypeVecId,
        // Function types MUST carry abilities in order to be used correctly as type arguments.
        // That is, `|| has drop` and `||` are different types.
        abilities: AbilitySet,
    },
}

/// Defines what kind of pre-computed metadata is stored for the given [TypeId].
#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RowKind {
    Primitive = 0,
    Vector = 1,
    Struct = 2,
    Function = 3,
}

impl From<u8> for RowKind {
    fn from(v: u8) -> Self {
        match v {
            0 => RowKind::Primitive,
            1 => RowKind::Vector,
            2 => RowKind::Struct,
            3 => RowKind::Function,
            _ => panic!("invalid row kind discriminant: {}", v),
        }
    }
}

#[inline]
fn pack_meta0(kind: RowKind, abilities: AbilitySet, payload0_u32: u32) -> u64 {
    let abilities = abilities.into_u8() & 0xF;
    ((kind as u64) & 0b11)
        | (((abilities as u64) & 0xF) << 2)
        | (((payload0_u32 as u64) & 0xFFFF_FFFF) << 6)
}

#[inline]
fn unpack_kind_from_meta0(meta0: u64) -> RowKind {
    RowKind::from((meta0 & 0b11) as u8)
}

#[inline(always)]
fn abilities_from_meta0(meta0: u64) -> AbilitySet {
    AbilitySet::from_u8(((meta0 >> 2) & 0xF) as u8).expect("Ability bits should always be valid")
}

#[inline]
fn unpack_payload_from_meta0(meta0: u64) -> u32 {
    ((meta0 >> 6) & 0xFFFF_FFFF) as u32
}

#[inline]
fn pack_meta1(payload1_u32: u32) -> u64 {
    (payload1_u32 as u64) & 0xFFFF_FFFF
}

#[inline]
fn unpack_payload1(meta1: u64) -> u32 {
    (meta1 & 0xFFFF_FFFF) as u32
}

struct InternMap<T, I> {
    interned: HashMap<T, I>,
    data: Vec<T>,
    // Stores type kind, abilities, and 32-bit payload for primitive and vector types. For structs
    // also stores the struct name index.
    metadata_0: Vec<u64>,
    // Stores additional payload.
    //   - for structs types: type arguments.
    //   - for function types: arguments and results.
    metadata_1: Vec<u64>,
}

impl<T, I> Default for InternMap<T, I> {
    fn default() -> Self {
        Self {
            interned: HashMap::new(),
            data: Vec::with_capacity(16),
            metadata_0: Vec::with_capacity(16),
            metadata_1: Vec::with_capacity(16),
        }
    }
}

impl<T, I> InternMap<T, I> {
    fn clear(&mut self) {
        self.interned.clear();
        self.data.clear();
        self.metadata_0.clear();
        self.metadata_1.clear();
    }
}

/// Interns single types.
struct TypeInterner {
    hot: UnsafeCell<InternMap<TypeRepr, TypeId>>,
    cold: RwLock<InternMap<TypeRepr, TypeId>>,
}

// Safety: `hot` is only mutated via `publish_cold_to_hot_unchecked` at block boundaries
// when the caller ensures quiescence. `cold` guards all concurrent mutations within a block.
unsafe impl Sync for TypeInterner {}

impl Default for TypeInterner {
    fn default() -> Self {
        Self {
            hot: UnsafeCell::new(InternMap::default()),
            cold: RwLock::new(InternMap::default()),
        }
    }
}

impl TypeInterner {
    #[inline]
    fn hot_ref(&self) -> &InternMap<TypeRepr, TypeId> {
        // Safe for read-only access as long as `hot` is not mutated concurrently.
        unsafe { &*self.hot.get() }
    }

    fn intern_vector(&self, elem: TypeId, abilities: AbilitySet) -> TypeId {
        let repr = TypeRepr::Vector(elem);

        // Fast path: lock-free hit in hot tier.
        if let Some(id) = self.hot_ref().interned.get(&repr) {
            return *id;
        }

        // Try cold tier under shared read lock.
        if let Some(id) = self.cold.read().interned.get(&repr) {
            return *id;
        }

        // Insert into cold tier under write lock.
        let mut cold = self.cold.write();
        if let Some(id) = cold.interned.get(&repr) {
            return *id;
        }

        let id = TypeId((self.hot_ref().data.len() + cold.data.len()) as u32);
        cold.data.push(repr);
        cold.interned.insert(repr, id);
        let m0 = pack_meta0(RowKind::Vector, abilities, elem.0);
        cold.metadata_0.push(m0);
        // Dummy placeholder for metadata 1. TODO: change to u128.
        cold.metadata_1.push(0);
        id
    }

    fn intern_struct(
        &self,
        name_idx: StructNameIndex,
        ty_args: TypeVecId,
        abilities: AbilitySet,
    ) -> TypeId {
        let repr = TypeRepr::Struct {
            idx: name_idx,
            ty_args,
        };

        // Fast path: lock-free hit in hot tier.
        if let Some(id) = self.hot_ref().interned.get(&repr) {
            return *id;
        }

        // Try cold tier under shared read lock.
        if let Some(id) = self.cold.read().interned.get(&repr) {
            return *id;
        }

        // Insert into cold tier under write lock.
        let mut cold = self.cold.write();
        if let Some(id) = cold.interned.get(&repr) {
            return *id;
        }

        let id = TypeId((self.hot_ref().data.len() + cold.data.len()) as u32);
        cold.data.push(repr);
        cold.interned.insert(repr, id);
        cold.metadata_0
            .push(pack_meta0(RowKind::Struct, abilities, name_idx.0));
        cold.metadata_1.push(pack_meta1(ty_args.0));
        id
    }

    fn intern_function(
        &self,
        args: TypeVecId,
        results: TypeVecId,
        abilities: AbilitySet,
    ) -> TypeId {
        let repr = TypeRepr::Function {
            args,
            results,
            abilities,
        };

        // Fast path: lock-free hit in hot tier.
        if let Some(id) = self.hot_ref().interned.get(&repr) {
            return *id;
        }

        // Try cold tier under shared read lock.
        if let Some(id) = self.cold.read().interned.get(&repr) {
            return *id;
        }

        // Insert into cold tier under write lock.
        let mut cold = self.cold.write();
        if let Some(id) = cold.interned.get(&repr) {
            return *id;
        }

        let id = TypeId((self.hot_ref().data.len() + cold.data.len()) as u32);
        cold.data.push(repr);
        cold.interned.insert(repr, id);
        cold.metadata_0
            .push(pack_meta0(RowKind::Function, abilities, args.0));
        cold.metadata_1.push(pack_meta1(results.0));
        id
    }

    /// Unsafely publish cold tier into hot tier without locking hot.
    ///
    /// # Safety
    /// Caller must ensure global quiescence (no concurrent readers/writers) for this interner
    /// while this function runs. Mutates the `hot` tier without synchronization.
    pub unsafe fn publish_cold_to_hot_unchecked(&self) {
        let mut cold = self.cold.write();
        if cold.interned.is_empty() && cold.data.is_empty() {
            return;
        }
        // Move out cold state to minimize lock hold time.
        let data = std::mem::take(&mut cold.data);
        let m0 = std::mem::take(&mut cold.metadata_0);
        let m1 = std::mem::take(&mut cold.metadata_1);
        let map = std::mem::take(&mut cold.interned);
        drop(cold);

        let hot: &mut InternMap<TypeRepr, TypeId> = unsafe { &mut *self.hot.get() };
        hot.data.extend(data);
        hot.metadata_0 .extend(m0);
        hot.metadata_1.extend(m1);
        hot.interned.extend(map);
    }

    /// Unsafely clear both tiers. Resets indices back to 0.
    ///
    /// # Safety
    /// Caller must ensure global quiescence (no concurrent readers/writers) for this interner.
    pub unsafe fn clear_all_unchecked(&self) {
        {
            let mut cold = self.cold.write();
            cold.clear();
        }
        let hot = unsafe { &mut *self.hot.get() };
        hot.clear();
    }
}

/// Interns vector of types (e.g., list of type arguments).
struct TypeVecInterner {
    hot: UnsafeCell<InternMap<Arc<[TypeId]>, TypeVecId>>,
    cold: RwLock<InternMap<Arc<[TypeId]>, TypeVecId>>,
}

// Safety: same reasoning as for TypeInterner.
unsafe impl Sync for TypeVecInterner {}

impl Default for TypeVecInterner {
    fn default() -> Self {
        Self {
            hot: UnsafeCell::new(InternMap::default()),
            cold: RwLock::new(InternMap::default()),
        }
    }
}

impl TypeVecInterner {
    #[inline]
    fn hot_ref(&self) -> &InternMap<Arc<[TypeId]>, TypeVecId> {
        // Safe for read-only access as long as `hot` is not mutated concurrently.
        unsafe { &*self.hot.get() }
    }

    fn intern(&self, tys: &[TypeId]) -> TypeVecId {
        // Fast path: lock-free hit in hot tier (borrowed lookup by slice).
        if let Some(id) = self.hot_ref().interned.get(tys) {
            return *id;
        }

        // Try cold tier under shared read lock.
        if let Some(id) = self.cold.read().interned.get(tys) {
            return *id;
        }

        let tys_arced: Arc<[TypeId]> = Arc::from(tys);
        let tys_arced_key = tys_arced.clone();

        // Insert into cold tier under write lock.
        let mut cold = self.cold.write();
        if let Some(id) = cold.interned.get(tys) {
            return *id;
        }
        let id = TypeVecId((self.hot_ref().data.len() + cold.data.len()) as u32);
        cold.data.push(tys_arced);
        cold.interned.insert(tys_arced_key, id);
        id
    }

    fn intern_vec(&self, tys: Vec<TypeId>) -> TypeVecId {
        // Fast path: lock-free hit in hot tier (borrowed lookup by slice).
        if let Some(id) = self.hot_ref().interned.get(tys.as_slice()) {
            return *id;
        }

        let tys: Arc<[TypeId]> = tys.into();
        let tys_key = tys.clone();

        // Try cold tier under shared read lock.
        if let Some(id) = self.cold.read().interned.get(&tys) {
            return *id;
        }

        // Insert into cold tier under write lock.
        let mut cold = self.cold.write();
        if let Some(id) = cold.interned.get(&tys) {
            return *id;
        }
        let id = TypeVecId((self.hot_ref().data.len() + cold.data.len()) as u32);
        cold.data.push(tys);
        cold.interned.insert(tys_key, id);
        id
    }

    /// Unsafely publish cold tier into hot tier without locking hot.
    ///
    /// # Safety
    /// Caller must ensure global quiescence (no concurrent readers/writers) for this interner
    /// while this function runs. Mutates the `hot` tier without synchronization.
    pub unsafe fn publish_cold_to_hot_unchecked(&self) {
        let mut cold = self.cold.write();
        if cold.interned.is_empty() && cold.data.is_empty() {
            return;
        }
        // Move out cold state to minimize lock hold time.
        let data = std::mem::take(&mut cold.data);
        let map = std::mem::take(&mut cold.interned);
        drop(cold);

        let hot: &mut InternMap<Arc<[TypeId]>, TypeVecId> = unsafe { &mut *self.hot.get() };
        hot.data.extend(data);
        hot.interned.extend(map);
        // metadata does not matter
    }

    /// Unsafely clear both tiers. Resets indices back to 0.
    ///
    /// # Safety
    /// Caller must ensure global quiescence (no concurrent readers/writers) for this interner.
    pub unsafe fn clear_all_unchecked(&self) {
        {
            let mut cold = self.cold.write();
            cold.clear();
        }
        let hot = unsafe { &mut *self.hot.get() };
        hot.clear();
    }

    fn get_vec_arc(&self, id: TypeVecId) -> Arc<[TypeId]> {
        let data = &self.hot_ref().data;
        if data.len() > id.0 as usize {
            data[id.0 as usize].clone()
        } else {
            let cold_idx = (id.0 as usize) - data.len();
            self.cold.read().data[cold_idx].clone()
        }
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
        self.ty_interner.hot_ref().data.len() + self.ty_interner.cold.read().data.len()
    }

    /// Returns how many distinct vectors of types are instantiated.
    pub fn num_interned_ty_vecs(&self) -> usize {
        self.ty_vec_interner.hot_ref().data.len() + self.ty_vec_interner.cold.read().data.len()
    }

    /// Clears all interned data, and then warm-ups the cache for common types. Should be called if
    /// type IDs are no longer used, e.g., when flushing module cache at block boundaries.
    pub fn flush(&self) {
        self.flush_impl();
        self.warmup();
    }

    /// Flushes all cached data without warming up the cache.
    fn flush_impl(&self) {
        // Safety: caller ensures quiescence when flushing interners in tests.
        unsafe {
            self.ty_interner.clear_all_unchecked();
            self.ty_vec_interner.clear_all_unchecked();
        }
    }

    /// Interns common type representations.
    fn warmup(&self) {
        {
            let mut cold = self.ty_interner.cold.write();
            assert!(cold.interned.is_empty());
            assert!(cold.data.is_empty());

            // ID 0: Bool
            cold.data.push(TypeRepr::Bool);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::Bool, TypeId::BOOL);

            // ID 1: U8
            cold.data.push(TypeRepr::U8);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::U8, TypeId::U8);

            // ID 2: U16
            cold.data.push(TypeRepr::U16);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::U16, TypeId::U16);

            // ID 3: U32
            cold.data.push(TypeRepr::U32);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::U32, TypeId::U32);

            // ID 4: U64
            cold.data.push(TypeRepr::U64);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::U64, TypeId::U64);

            // ID 5: U128
            cold.data.push(TypeRepr::U128);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::U128, TypeId::U128);

            // ID 6: U256
            cold.data.push(TypeRepr::U256);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::U256, TypeId::U256);

            // ID 7: I8
            cold.data.push(TypeRepr::I8);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::I8, TypeId::I8);

            // ID 8: I16
            cold.data.push(TypeRepr::I16);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::I16, TypeId::I16);

            // ID 9: I32
            cold.data.push(TypeRepr::I32);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::I32, TypeId::I32);

            // ID 10: I64
            cold.data.push(TypeRepr::I64);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::I64, TypeId::I64);

            // ID 11: I128
            cold.data.push(TypeRepr::I128);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::I128, TypeId::I128);

            // ID 12: I256
            cold.data.push(TypeRepr::I256);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::I256, TypeId::I256);

            // ID 13: Address
            cold.data.push(TypeRepr::Address);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::Address, TypeId::ADDRESS);

            // ID 14: Signer
            cold.data.push(TypeRepr::Signer);
            cold.metadata_0.push(0);
            cold.metadata_1.push(0);
            cold.interned.insert(TypeRepr::Signer, TypeId::SIGNER);
        }

        self.ty_vec_interner.intern(&[]);
        self.ty_vec_interner.intern(&[TypeId::U8]);
        self.ty_vec_interner.intern(&[TypeId::U64]);
        // Publish warm entries into hot tier for lock-free reads.
        unsafe {
            self.ty_interner.publish_cold_to_hot_unchecked();
            self.ty_vec_interner.publish_cold_to_hot_unchecked();
        }
    }

    /// Given a vector if fully-instantiated type arguments, returns the corresponding [TypeVecId].
    ///
    /// Panics if there are non-instantiated type arguments.
    pub fn intern_ty_args(&self, ty_args: &[Type]) -> TypeVecId {
        let ty_args = ty_args
            .iter()
            .map(|t| self.instantiate_and_intern(t, &[]))
            .collect::<Vec<_>>();
        self.ty_vec_interner.intern_vec(ty_args)
    }

    pub fn intern_ty_slice(&self, tys: &[TypeId]) -> TypeVecId {
        self.ty_vec_interner.intern(tys)
    }

    // TODO: check bound at load-time (to bound type / constant size).
    pub fn create_constant_ty(&self, constant_tok: &SignatureToken) -> TypeId {
        use SignatureToken as S;

        match constant_tok {
            S::Bool => TypeId::BOOL,
            S::U8 => TypeId::U8,
            S::U16 => TypeId::U16,
            S::U32 => TypeId::U32,
            S::U64 => TypeId::U64,
            S::U128 => TypeId::U128,
            S::U256 => TypeId::U256,
            S::I8 => TypeId::I8,
            S::I16 => TypeId::I16,
            S::I32 => TypeId::I32,
            S::I64 => TypeId::I64,
            S::I128 => TypeId::I128,
            S::I256 => TypeId::I256,
            S::Address => TypeId::ADDRESS,
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
                self.ty_interner.intern_vector(elem, abilities)
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

    // TODO: add counts + return a result
    pub fn instantiate_and_intern(&self, ty: &Type, subst: &[TypeId]) -> TypeId {
        use Type::*;
        match ty {
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
                let elem_abilities = self.abilities(elem_id);
                let abilities =
                    AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                        elem_abilities,
                    ])
                    .expect("Vector ability computation should not fail");
                self.ty_interner.intern_vector(elem_id, abilities)
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
                let abilities = ability.base_ability_set;
                self.ty_interner
                    .intern_struct(*idx, self.ty_vec_interner.intern(&[]), abilities)
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
                self.ty_interner.intern_struct(
                    *idx,
                    self.ty_vec_interner.intern_vec(ty_arg_ids),
                    abilities,
                )
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
                let args_id = self.ty_vec_interner.intern_vec(arg_ids);
                let results_id = self.ty_vec_interner.intern_vec(result_ids);
                self.ty_interner
                    .intern_function(args_id, results_id, *abilities)
            },
        }
    }

    // todo: refactor
    pub fn intern_struct_instantiation(
        &self,
        struct_ty: &StructType,
        ty_params: &[Type],
        ty_args: &[TypeId],
    ) -> TypeId {
        let ty_arg_ids = ty_params
            .iter()
            .map(|ty| self.instantiate_and_intern(ty, ty_args))
            .collect::<Vec<_>>();

        let type_argument_abilities = ty_arg_ids
            .iter()
            .map(|&ty_id| self.abilities(ty_id))
            .collect::<Vec<_>>();

        let abilities = AbilitySet::polymorphic_abilities(
            struct_ty.abilities,
            struct_ty.phantom_ty_params_mask.iter(),
            type_argument_abilities,
        )
        .expect("StructInstantiation ability computation should not fail");
        self.ty_interner.intern_struct(
            struct_ty.idx,
            self.ty_vec_interner.intern_vec(ty_arg_ids),
            abilities,
        )
    }

    #[inline]
    pub fn vec_of(&self, elem: TypeId) -> TypeId {
        let elem_abilities = self.abilities(elem);
        let abilities = AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
            elem_abilities,
        ])
        .expect("Vector ability computation should not fail");
        self.ty_interner.intern_vector(elem, abilities)
    }

    #[inline]
    fn metadata_0(&self, ty: TypeId) -> u64 {
        let metadata_0 = &self.ty_interner.hot_ref().metadata_0;
        if metadata_0.len() > ty.0 as usize {
            metadata_0[ty.0 as usize]
        } else {
            let cold_idx = (ty.0 as usize) - metadata_0.len();
            self.ty_interner.cold.read().metadata_0[cold_idx]
        }
    }

    #[inline]
    fn metadata_1(&self, ty: TypeId) -> u64 {
        let metadata_1 = &self.ty_interner.hot_ref().metadata_1;
        if metadata_1.len() > ty.0 as usize {
            metadata_1[ty.0 as usize]
        } else {
            let cold_idx = (ty.0 as usize) - metadata_1.len();
            self.ty_interner.cold.read().metadata_1[cold_idx]
        }
    }

    #[inline]
    pub fn type_repr(&self, ty: TypeId) -> TypeRepr {
        if ty.is_ref() {
            return TypeRepr::Reference(ty.payload());
        }
        if ty.is_mut_ref() {
            return TypeRepr::MutableReference(ty.payload());
        }

        match ty {
            TypeId::BOOL => TypeRepr::Bool,
            TypeId::U8 => TypeRepr::U8,
            TypeId::U16 => TypeRepr::U16,
            TypeId::U32 => TypeRepr::U32,
            TypeId::U64 => TypeRepr::U64,
            TypeId::U128 => TypeRepr::U128,
            TypeId::U256 => TypeRepr::U256,
            TypeId::I8 => TypeRepr::I8,
            TypeId::I16 => TypeRepr::I16,
            TypeId::I32 => TypeRepr::I32,
            TypeId::I64 => TypeRepr::I64,
            TypeId::I128 => TypeRepr::I128,
            TypeId::I256 => TypeRepr::I256,
            TypeId::ADDRESS => TypeRepr::Address,
            TypeId::SIGNER => TypeRepr::Signer,
            ty => {
                let m0 = self.metadata_0(ty);
                match unpack_kind_from_meta0(m0) {
                    RowKind::Primitive => unreachable!("composite ids should not be primitive"),
                    RowKind::Vector => {
                        let elem = TypeId(unpack_payload_from_meta0(m0));
                        TypeRepr::Vector(elem)
                    },
                    RowKind::Struct => {
                        let sidx = StructNameIndex(unpack_payload_from_meta0(m0));
                        let m1 = self.metadata_1(ty);
                        let args = TypeVecId(unpack_payload1(m1));
                        TypeRepr::Struct {
                            idx: sidx,
                            ty_args: args,
                        }
                    },
                    RowKind::Function => {
                        let args = TypeVecId(unpack_payload_from_meta0(m0));
                        let m1 = self.metadata_1(ty);
                        let results = TypeVecId(unpack_payload1(m1));
                        TypeRepr::Function {
                            args,
                            results,
                            abilities: abilities_from_meta0(m0),
                        }
                    },
                }
            },
        }
    }

    #[inline]
    pub fn function_of(
        &self,
        args: &[TypeId],
        results: &[TypeId],
        abilities: AbilitySet,
    ) -> TypeId {
        let args_id = self.ty_vec_interner.intern(args);
        let results_id = self.ty_vec_interner.intern(results);
        self.ty_interner
            .intern_function(args_id, results_id, abilities)
    }

    #[inline]
    pub fn function_of_vec(
        &self,
        args: Vec<TypeId>,
        results: Vec<TypeId>,
        abilities: AbilitySet,
    ) -> TypeId {
        let args_id = self.ty_vec_interner.intern_vec(args);
        let results_id = self.ty_vec_interner.intern_vec(results);
        self.ty_interner
            .intern_function(args_id, results_id, abilities)
    }

    /// Unsafely publish cold tiers of both interners into their hot tiers.
    /// Safety: Caller must ensure global quiescence (e.g., at block boundary).
    ///
    /// # Safety
    /// The caller must guarantee that no other threads are reading from or writing to
    /// the underlying interners while this function executes. This should only be
    /// invoked at a block boundary or other global quiescence point.
    pub unsafe fn publish_unchecked(&self) {
        unsafe {
            self.ty_interner.publish_cold_to_hot_unchecked();
            self.ty_vec_interner.publish_cold_to_hot_unchecked();
        }
    }

    /// Unsafely clear both interners and re-warm caches. Resets indices back to 0.
    /// Safety: Caller must ensure global quiescence (e.g., at block boundary).
    ///
    /// # Safety
    /// The caller must guarantee global quiescence across all users of this pool.
    /// No concurrent readers or writers may access the interners while the clear and
    /// subsequent warmup are in progress.
    pub unsafe fn clear_all_unchecked(&self) {
        unsafe {
            self.ty_interner.clear_all_unchecked();
            self.ty_vec_interner.clear_all_unchecked();
        }
        self.warmup();
    }

    #[inline]
    pub fn get_vec_elem_ty(&self, ty: TypeId) -> Option<TypeId> {
        match self.type_repr(ty) {
            TypeRepr::Vector(elem) => Some(elem),
            _ => None,
        }
    }

    #[inline]
    pub fn get_type_vec(&self, id: TypeVecId) -> Arc<[TypeId]> {
        self.ty_vec_interner.get_vec_arc(id)
    }

    #[inline]
    pub fn abilities(&self, ty: TypeId) -> AbilitySet {
        if ty >= TypeId::BOOL && ty <= TypeId::ADDRESS {
            return AbilitySet::PRIMITIVES;
        }
        if ty == TypeId::SIGNER {
            return AbilitySet::SIGNER;
        }
        if ty.is_ref() || ty.is_mut_ref() {
            return AbilitySet::REFERENCES;
        }

        let m0 = self.metadata_0(ty);
        abilities_from_meta0(m0)
    }

    #[inline]
    pub fn has_ability(&self, ty: TypeId, ability: Ability) -> bool {
        self.abilities(ty).has_ability(ability)
    }

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

    #[inline]
    pub fn paranoid_check_is_vec_ty(
        &self,
        ty: TypeId,
        expected_elem: TypeId,
    ) -> PartialVMResult<()> {
        let m0 = self.metadata_0(ty);
        if unpack_kind_from_meta0(m0) == RowKind::Vector {
            let elem = unpack_payload_from_meta0(m0);
            if elem == expected_elem.0 {
                return Ok(());
            }
        }

        Err(PartialVMError::new_invariant_violation(format!(
            "Paranoid mode: expected vector<{:?}> type, got {:?}",
            self.type_repr(expected_elem),
            self.type_repr(ty)
        ))
        .with_sub_status(EPARANOID_FAILURE))
    }

    #[inline]
    pub fn paranoid_check_is_vec_ref_ty<const IS_MUT: bool>(
        &self,
        ty: TypeId,
        expected_elem: TypeId,
    ) -> PartialVMResult<()> {
        if ty.is_mut_ref() {
            return self.paranoid_check_is_vec_ty(ty.payload(), expected_elem);
        }

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

#[cfg(test)]
mod test {
    use super::*;
    use crate::loaded_data::{runtime_types::AbilityInfo, struct_name_indexing::StructNameIndex};
    use move_core_types::ability::AbilitySet;
    use std::{collections::HashSet, thread};

    #[test]
    fn test_primitive_types() {
        let ctx = InternedTypePool::new();

        let tys = [
            Type::Bool,
            Type::U8,
            Type::U16,
            Type::U32,
            Type::U64,
            Type::U128,
            Type::U256,
            Type::I8,
            Type::I16,
            Type::I32,
            Type::I64,
            Type::I128,
            Type::I256,
            Type::Address,
            Type::Signer,
        ];

        for ty in &tys {
            let id1 = ctx.instantiate_and_intern(ty, &[]);
            let id2 = ctx.instantiate_and_intern(ty, &[]);
            assert_eq!(id1, id2);
        }
    }

    #[test]
    fn test_vector_types() {
        let ctx = InternedTypePool::new();

        let vec_u8 = Type::Vector(Arc::new(Type::U8));
        let vec_u64 = Type::Vector(Arc::new(Type::U64));

        let u8_id = ctx.instantiate_and_intern(&Type::U8, &[]);
        let vec_u8_id1 = ctx.instantiate_and_intern(&vec_u8, &[]);
        let vec_u8_id2 = ctx.instantiate_and_intern(&vec_u8, &[]);
        let vec_u64_id = ctx.instantiate_and_intern(&vec_u64, &[]);

        assert_eq!(vec_u8_id1, vec_u8_id2);
        assert_ne!(vec_u8_id1, vec_u64_id);
        assert_ne!(vec_u8_id1, u8_id);
    }

    #[test]
    fn test_reference_types() {
        let ctx = InternedTypePool::new();

        let u64_ref = Type::Reference(Box::new(Type::U64));
        let u64_mut_ref = Type::MutableReference(Box::new(Type::U64));
        let u8_ref = Type::Reference(Box::new(Type::U8));

        let u64_ref_id1 = ctx.instantiate_and_intern(&u64_ref, &[]);
        let u64_ref_id2 = ctx.instantiate_and_intern(&u64_ref, &[]);
        let u64_mut_ref_id = ctx.instantiate_and_intern(&u64_mut_ref, &[]);
        let u8_ref_id = ctx.instantiate_and_intern(&u8_ref, &[]);

        assert_eq!(u64_ref_id1, u64_ref_id2);
        assert_ne!(u64_ref_id1, u64_mut_ref_id);
        assert_ne!(u64_ref_id1, u8_ref_id);
    }

    #[test]
    fn test_struct_types() {
        let ctx = InternedTypePool::new();

        let struct_type = Type::Struct {
            idx: StructNameIndex::new(0),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        };

        let id1 = ctx.instantiate_and_intern(&struct_type, &[]);
        let id2 = ctx.instantiate_and_intern(&struct_type, &[]);

        assert_eq!(id1, id2);
    }

    #[test]
    fn test_structs() {
        let ctx = InternedTypePool::new();

        let struct_ty = Type::StructInstantiation {
            idx: StructNameIndex::new(0),
            ty_args: Arc::new(vec![Type::U64, Type::Bool]),
            // Irrelevant for tests.
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        };

        let id1 = ctx.instantiate_and_intern(&struct_ty, &[]);
        let id2 = ctx.instantiate_and_intern(&struct_ty, &[]);
        assert_eq!(id1, id2);

        let struct_inst2 = Type::StructInstantiation {
            idx: StructNameIndex::new(0),
            ty_args: Arc::new(vec![Type::Bool, Type::U64]),
            // Irrelevant for tests.
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        };

        let id3 = ctx.instantiate_and_intern(&struct_inst2, &[]);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_function_types() {
        let ctx = InternedTypePool::new();

        let func_ty = Type::Function {
            args: vec![Type::U64, Type::Bool],
            results: vec![Type::U8],
            abilities: AbilitySet::EMPTY,
        };

        let id1 = ctx.instantiate_and_intern(&func_ty, &[]);
        let id2 = ctx.instantiate_and_intern(&func_ty, &[]);

        assert_eq!(id1, id2);

        let func_ty = Type::Function {
            args: vec![Type::U64],
            results: vec![Type::U8],
            abilities: AbilitySet::EMPTY,
        };

        let id3 = ctx.instantiate_and_intern(&func_ty, &[]);
        assert_ne!(id1, id3);

        let func_ty = Type::Function {
            args: vec![Type::U64],
            results: vec![Type::U8],
            abilities: AbilitySet::ALL,
        };
        let id4 = ctx.instantiate_and_intern(&func_ty, &[]);
        assert_ne!(id3, id4);
    }

    #[test]
    fn test_deeply_nested_type() {
        let ctx = InternedTypePool::new();

        let mut ty = Type::U64;
        for _ in 0..10 {
            ty = Type::Vector(Arc::new(ty));
        }

        let id1 = ctx.instantiate_and_intern(&ty, &[]);
        let id2 = ctx.instantiate_and_intern(&ty, &[]);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_type_parameter_substitution() {
        let ctx = InternedTypePool::new();

        let u64_id = ctx.instantiate_and_intern(&Type::U64, &[]);
        let substituted_id = ctx.instantiate_and_intern(&Type::TyParam(0), &[u64_id]);
        assert_eq!(substituted_id, u64_id);
    }

    #[test]
    fn test_empty_type_vectors() {
        let ctx = InternedTypePool::new();
        let id1 = ctx.intern_ty_args(&[]);
        let id2 = ctx.intern_ty_args(&[]);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_type_vector_consistency() {
        let ctx = InternedTypePool::new();

        let u64_ty = Type::U64;
        let bool_ty = Type::Bool;

        let mut tys = vec![u64_ty, bool_ty];
        let id1 = ctx.intern_ty_args(&tys);
        let id2 = ctx.intern_ty_args(&tys);
        assert_eq!(id1, id2);

        tys.reverse();
        let id3 = ctx.intern_ty_args(&tys);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_flush_clears_cache() {
        let ctx = InternedTypePool::new();

        ctx.intern_ty_args(&[Type::U64]);
        ctx.instantiate_and_intern(&Type::U64, &[]);

        let initial_ty_count = ctx.num_interned_tys();
        let initial_ty_vec_count = ctx.num_interned_ty_vecs();

        assert!(initial_ty_count > 0);
        assert!(initial_ty_vec_count > 0);

        ctx.flush_impl();

        assert_eq!(ctx.num_interned_tys(), 0);
        assert_eq!(ctx.num_interned_ty_vecs(), 0);
    }

    #[test]
    fn test_concurrent_interning_same_type() {
        let ctx = Arc::new(InternedTypePool::new());

        let mut handles = Vec::new();
        for _ in 0..10 {
            let ctx = Arc::clone(&ctx);
            let handle = thread::spawn(move || ctx.instantiate_and_intern(&Type::U64, &[]));
            handles.push(handle);
        }

        let mut ids = HashSet::new();
        for handle in handles {
            let id = handle.join().unwrap();
            ids.insert(id);
        }
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn test_concurrent_interning_different_types() {
        let ctx = Arc::new(InternedTypePool::new());
        let tys = Arc::new(vec![
            Type::Bool,
            Type::U8,
            Type::U16,
            Type::U32,
            Type::U64,
            Type::U128,
            Type::U256,
            Type::I8,
            Type::I16,
            Type::I32,
            Type::I64,
            Type::I128,
            Type::I256,
            Type::Address,
            Type::Signer,
        ]);

        let mut handles = Vec::new();
        for i in 0..tys.len() {
            let tys = Arc::clone(&tys);
            let ctx = Arc::clone(&ctx);
            let handle = thread::spawn(move || ctx.instantiate_and_intern(&tys[i], &[]));
            handles.push(handle);
        }

        let mut ids = HashSet::new();
        for handle in handles {
            let id = handle.join().unwrap();
            ids.insert(id);
        }
        assert_eq!(ids.len(), tys.len());
    }
}
