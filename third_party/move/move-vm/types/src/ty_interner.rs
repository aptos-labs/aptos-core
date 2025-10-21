// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Data structures and caches for interning types as unique compact identifiers. The lifetime of
//! these caches is tied to the code cache, and is managed externally.

use crate::loaded_data::{
    runtime_types::{StructType, Type, TypeBuilder},
    struct_name_indexing::StructNameIndex,
};
use arc_swap::ArcSwap;
use crossbeam::utils::CachePadded;
use dashmap::{DashMap, Entry};
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::SignatureToken,
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    vm_status::sub_status::unknown_invariant_violation::EPARANOID_FAILURE,
};
use parking_lot::Mutex;
use std::sync::{
    atomic::{AtomicU64, AtomicUsize, Ordering},
    Arc, OnceLock,
};

/// Compactly represents a loaded type.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TypeId(u32);

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

    /// Return
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
        abilities: AbilitySet,
    },
}

impl TypeInterner {
    fn clear(&self) {
        self.rev.clear();
        self.fwd.clear_and_reinit();
    }

    fn intern_vector(&self, elem: TypeId, abilities: AbilitySet) -> TypeId {
        let repr = TypeRepr::Vector(elem);
        if let Some(id) = self.rev.get(&repr) {
            return *id;
        }

        match self.rev.entry(repr) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let idx = self.fwd.append_row(|snap, idx| {
                    // publish order: write payloads, then header, then bump used.
                    let m0 = pack_meta0(RowKind::Vector, abilities, elem.0);
                    snap.metadata_0[idx as usize].store(m0, Ordering::Release);
                });
                let id = TypeId(idx);
                let _ = entry.insert(id);
                id
            },
        }
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
        if let Some(id) = self.rev.get(&repr) {
            return *id;
        }

        match self.rev.entry(repr) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let idx = self.fwd.append_row(|snap, idx| {
                    snap.metadata_1[idx as usize].store(pack_meta1(ty_args.0), Ordering::Release);
                    let m0 = pack_meta0(RowKind::Struct, abilities, name_idx.0);
                    snap.metadata_0[idx as usize].store(m0, Ordering::Release);
                });
                let id = TypeId(idx);
                let _ = entry.insert(id);
                id
            },
        }
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
        if let Some(id) = self.rev.get(&repr) {
            return *id;
        }

        match self.rev.entry(repr) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => {
                let idx = self.fwd.append_row(|snap, idx| {
                    snap.metadata_1[idx as usize].store(pack_meta1(results.0), Ordering::Release);
                    let m0 = pack_meta0(RowKind::Function, abilities, args.0);
                    snap.metadata_0[idx as usize].store(m0, Ordering::Release);
                });
                let id = TypeId(idx);
                let _ = entry.insert(id);
                id
            },
        }
    }
}

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
    let abil4 = abilities.into_u8() & 0xF;
    ((kind as u64) & 0b11)
        | (((abil4 as u64) & 0xF) << 2)
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

struct ForwardSnapshot {
    capacity: usize,
    used: CachePadded<AtomicUsize>,
    // Stores type kind, abilities, and 32-bit payload for primitive and vector types. For structs
    // also stores the struct name index.
    metadata_0: Vec<AtomicU64>,
    /// Stores additional payload.
    //   - for structs types: type arguments.
    //   - for function types: arguments and results.
    metadata_1: Vec<AtomicU64>,
}

impl ForwardSnapshot {
    fn with_capacity(capacity: usize) -> Self {
        let init = || (0..capacity).map(|_| AtomicU64::new(0)).collect::<Vec<_>>();
        Self {
            capacity,
            used: CachePadded::new(AtomicUsize::new(0)),
            metadata_0: init(),
            metadata_1: init(),
        }
    }

    fn copy_from(old: &Self, capacity: usize) -> Self {
        let new = ForwardSnapshot::with_capacity(capacity);
        let used = old.used.load(Ordering::Acquire);

        #[allow(clippy::needless_range_loop)]
        for i in 0..used {
            let m0 = old.metadata_0[i].load(Ordering::Acquire);
            let m1 = old.metadata_1[i].load(Ordering::Acquire);
            new.metadata_0[i].store(m0, Ordering::Release);
            new.metadata_1[i].store(m1, Ordering::Release);
        }
        new.used.store(used, Ordering::Release);
        new
    }
}

struct ForwardTables {
    snap: ArcSwap<ForwardSnapshot>,
    grow_lock: Mutex<()>,
}

impl ForwardTables {
    fn new(capacity: usize) -> Self {
        let snap = ForwardSnapshot::with_capacity(capacity);

        // Warm-up for primitive types.
        let prim_used = (TypeId::SIGNER.0 + 1) as usize;
        for i in 0..prim_used {
            let m0 = pack_meta0(RowKind::Primitive, AbilitySet::PRIMITIVES, 0);
            snap.metadata_0[i].store(m0, Ordering::Relaxed);
        }
        snap.used.store(prim_used, Ordering::Release);

        Self {
            snap: ArcSwap::from_pointee(snap),
            grow_lock: Mutex::new(()),
        }
    }

    fn append_row<F>(&self, write_row: F) -> u32
    where
        F: FnOnce(&ForwardSnapshot, u32),
    {
        let _g = self.grow_lock.lock();

        // load current
        let mut snap = self.snap.load();
        let mut idx = snap.used.load(Ordering::Acquire) as u32;

        // grow if needed
        if (idx as usize) >= snap.capacity {
            let new_cap = snap.capacity.saturating_mul(2);
            let new_snap = ForwardSnapshot::copy_from(&snap, new_cap);
            self.snap.store(Arc::new(new_snap));
            snap = self.snap.load();
            idx = snap.used.load(Ordering::Acquire) as u32;
        }

        // write row (payloads then header is handled inside write_row)
        write_row(&snap, idx);

        // publish row index
        snap.used.fetch_add(1, Ordering::Release);
        idx
    }

    fn clear_and_reinit(&self) {
        let _g = self.grow_lock.lock();
        let fresh = ForwardSnapshot::with_capacity(1024);
        let prim_used = (TypeId::SIGNER.0 + 1) as usize;
        for i in 0..prim_used {
            let m0 = pack_meta0(RowKind::Primitive, AbilitySet::PRIMITIVES, 0);
            fresh.metadata_0[i].store(m0, Ordering::Relaxed);
        }
        fresh.used.store(prim_used, Ordering::Release);
        self.snap.store(Arc::new(fresh));
    }
}

struct TypeInterner {
    rev: DashMap<TypeRepr, TypeId>,
    fwd: ForwardTables,
}

impl Default for TypeInterner {
    fn default() -> Self {
        Self {
            rev: DashMap::with_capacity(1024),
            fwd: ForwardTables::new(1024),
        }
    }
}

struct TypeVecForwardSnapshot {
    capacity: usize,
    used: CachePadded<AtomicUsize>,
    // Per-row atomic pointer to the actual vector type payload.
    values: Vec<OnceLock<Arc<[TypeId]>>>,
}

impl TypeVecForwardSnapshot {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            used: CachePadded::new(AtomicUsize::new(0)),
            values: (0..capacity).map(|_| OnceLock::new()).collect(),
        }
    }

    fn copy_from(old: &Self, capacity: usize) -> Self {
        let used = old.used.load(Ordering::Acquire);
        let values = (0..capacity).map(|_| OnceLock::new()).collect::<Vec<_>>();
        #[allow(clippy::needless_range_loop)]
        for i in 0..used {
            let v = old.values[i]
                .get()
                .expect("Type vector row is not initialized in old snapshot")
                .clone();
            let _ = values[i].set(v);
        }
        Self {
            capacity,
            used: CachePadded::new(AtomicUsize::new(used)),
            values,
        }
    }
}

struct TypeVecForwardTables {
    snap: ArcSwap<TypeVecForwardSnapshot>,
    grow_lock: Mutex<()>,
}

impl TypeVecForwardTables {
    fn new(init_cap: usize) -> Self {
        let snap = TypeVecForwardSnapshot::with_capacity(init_cap);
        Self {
            snap: ArcSwap::from_pointee(snap),
            grow_lock: Mutex::new(()),
        }
    }

    fn append_row(&self, payload: Arc<[TypeId]>) -> u32 {
        let _g = self.grow_lock.lock();

        let mut cur = self.snap.load();
        let mut idx = cur.used.load(Ordering::Acquire) as u32;

        if (idx as usize) >= cur.capacity {
            let new_cap = cur.capacity.saturating_mul(2);
            let next = TypeVecForwardSnapshot::copy_from(&cur, new_cap);
            self.snap.store(Arc::new(next));
            cur = self.snap.load();
            idx = cur.used.load(Ordering::Acquire) as u32;
        }

        let cell = &cur.values[idx as usize];
        assert!(
            cell.set(payload).is_ok(),
            "Type vector row is initialized twice at index {}",
            idx
        );
        cur.used.fetch_add(1, Ordering::Release);
        idx
    }

    fn clear_and_reinit(&self) {
        let _g = self.grow_lock.lock();
        self.snap
            .store(Arc::new(TypeVecForwardSnapshot::with_capacity(1024)));
    }
}

/// Interns vector of types (e.g., list of type arguments).
struct TypeVecInterner {
    rev: DashMap<Arc<[TypeId]>, TypeVecId>,
    fwd: TypeVecForwardTables,
}

impl Default for TypeVecInterner {
    fn default() -> Self {
        Self {
            rev: DashMap::with_capacity(1024),
            fwd: TypeVecForwardTables::new(1024),
        }
    }
}

impl TypeVecInterner {
    fn clear(&self) {
        self.rev.clear();
        self.fwd.clear_and_reinit();
    }

    /// Borrowed-slice path: zero-copy lookup; allocates only on miss.
    fn intern(&self, tys: &[TypeId]) -> TypeVecId {
        if let Some(id) = self.rev.get(tys) {
            return *id;
        }

        let arc: Arc<[TypeId]> = Arc::from(tys);
        match self.rev.entry(arc.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(v) => {
                let idx = self.fwd.append_row(arc);
                let id = TypeVecId(idx);
                v.insert(id);
                id
            },
        }
    }

    fn intern_vec(&self, tys: Vec<TypeId>) -> TypeVecId {
        if let Some(id) = self.rev.get(tys.as_slice()) {
            return *id;
        }

        let arc: Arc<[TypeId]> = Arc::from(tys.into_boxed_slice());
        match self.rev.entry(arc.clone()) {
            Entry::Occupied(e) => *e.get(),
            Entry::Vacant(v) => {
                let idx = self.fwd.append_row(arc);
                let id = TypeVecId(idx);
                v.insert(id);
                id
            },
        }
    }

    fn get_vec_arc(&self, id: TypeVecId) -> Arc<[TypeId]> {
        let snap = self.fwd.snap.load();
        let idx = id.0 as usize;
        let used = snap.used.load(Ordering::Acquire);
        assert!(idx < used, "Invalid type vector index: {} >= {}", idx, used);
        snap.values[idx]
            .get()
            .expect("Type vector row must be initialized")
            .clone()
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
        self.ty_interner
            .fwd
            .snap
            .load()
            .used
            .load(Ordering::Acquire)
    }

    /// Returns how many distinct vectors of types are instantiated.
    pub fn num_interned_ty_vecs(&self) -> usize {
        self.ty_vec_interner
            .fwd
            .snap
            .load()
            .used
            .load(Ordering::Acquire)
    }

    /// Clears all interned data, and then warm-ups the cache for common types. Should be called if
    /// type IDs are no longer used, e.g., when flushing module cache at block boundaries.
    pub fn flush(&self) {
        self.flush_impl();
        self.warmup();
    }

    /// Flushes all cached data without warming up the cache.
    fn flush_impl(&self) {
        self.ty_interner.clear();
        self.ty_vec_interner.clear();
    }

    /// Interns common type representations.
    fn warmup(&self) {
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

    pub fn intern_ty_slice(&self, tys: &[TypeId]) -> TypeVecId {
        self.ty_vec_interner.intern(tys)
    }

    // TODO: check bound at load-time.
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
                // Get abilities from the struct's ability info
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

    #[allow(dead_code)]
    pub(crate) fn instantiate_and_intern_tok(
        &self,
        view: &BinaryIndexedView,
        interned_struct_names: &[StructNameIndex],
        ty: &SignatureToken,
        subst: &[TypeId],
    ) -> TypeId {
        use SignatureToken::*;
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
            TypeParameter(idx) => subst[*idx as usize],
            Vector(elem_ty) => {
                let elem_id =
                    self.instantiate_and_intern_tok(view, interned_struct_names, elem_ty, subst);
                let elem_abilities = self.abilities(elem_id);
                let abilities =
                    AbilitySet::polymorphic_abilities(AbilitySet::VECTOR, vec![false], vec![
                        elem_abilities,
                    ])
                    .expect("Vector ability computation should not fail");
                self.ty_interner.intern_vector(elem_id, abilities)
            },
            Reference(inner_ty) => {
                let inner_id =
                    self.instantiate_and_intern_tok(view, interned_struct_names, inner_ty, subst);
                TypeId::ref_of(inner_id)
            },
            MutableReference(inner_ty) => {
                let inner_id =
                    self.instantiate_and_intern_tok(view, interned_struct_names, inner_ty, subst);
                TypeId::ref_mut_of(inner_id)
            },
            Struct(idx) => {
                let struct_handle = view.struct_handle_at(*idx);
                let abilities = struct_handle.abilities;
                self.ty_interner.intern_struct(
                    interned_struct_names[idx.0 as usize],
                    self.ty_vec_interner.intern(&[]),
                    abilities,
                )
            },
            StructInstantiation(idx, ty_args) => {
                let ty_arg_ids = ty_args
                    .iter()
                    .map(|t| self.instantiate_and_intern_tok(view, interned_struct_names, t, subst))
                    .collect::<Vec<_>>();
                let ty_arg_abilities = ty_arg_ids
                    .iter()
                    .map(|&ty_id| self.abilities(ty_id))
                    .collect::<Vec<_>>();

                let struct_handle = view.struct_handle_at(*idx);
                let abilities = AbilitySet::polymorphic_abilities(
                    struct_handle.abilities,
                    struct_handle.type_parameters.iter().map(|ty| ty.is_phantom),
                    ty_arg_abilities,
                )
                .expect("TODO: should propagate error");
                self.ty_interner.intern_struct(
                    interned_struct_names[idx.0 as usize],
                    self.ty_vec_interner.intern_vec(ty_arg_ids),
                    abilities,
                )
            },
            Function(args, results, abilities) => {
                let arg_ids = args
                    .iter()
                    .map(|t| self.instantiate_and_intern_tok(view, interned_struct_names, t, subst))
                    .collect::<Vec<_>>();
                let result_ids = results
                    .iter()
                    .map(|t| self.instantiate_and_intern_tok(view, interned_struct_names, t, subst))
                    .collect::<Vec<_>>();
                let args_id = self.ty_vec_interner.intern_vec(arg_ids);
                let results_id = self.ty_vec_interner.intern_vec(result_ids);
                self.ty_interner
                    .intern_function(args_id, results_id, *abilities)
            },
        }
    }

    /// Creates a vector type with the given element type.
    /// Returns the TypeId of the vector type.
    /// Abilities are computed based on the element type's abilities.
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
    pub fn type_repr(&self, ty: TypeId) -> TypeRepr {
        if ty.is_ref() {
            return TypeRepr::Reference(ty.payload());
        }
        if ty.is_mut_ref() {
            return TypeRepr::MutableReference(ty.payload());
        }

        // primitives fast path
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
            _ => {
                let snap = self.ty_interner.fwd.snap.load();
                let idx = ty.0 as usize;
                let used = snap.used.load(Ordering::Acquire);
                assert!(idx < used, "invalid TypeId payload {}", idx);

                let m0 = snap.metadata_0[idx].load(Ordering::Relaxed);
                match unpack_kind_from_meta0(m0) {
                    RowKind::Primitive => unreachable!("composite ids should not be primitive"),
                    RowKind::Vector => {
                        let elem = TypeId(unpack_payload_from_meta0(m0));
                        TypeRepr::Vector(elem)
                    },
                    RowKind::Struct => {
                        let sidx = StructNameIndex(unpack_payload_from_meta0(m0));
                        let m1 = snap.metadata_1[idx].load(Ordering::Relaxed);
                        let args = TypeVecId(unpack_payload1(m1));
                        TypeRepr::Struct {
                            idx: sidx,
                            ty_args: args,
                        }
                    },
                    RowKind::Function => {
                        let args = TypeVecId(unpack_payload_from_meta0(m0));
                        let m1 = snap.metadata_1[idx].load(Ordering::Relaxed);
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

    /// Gets the element type of a vector.
    /// Returns None if the type is not a vector.
    #[inline]
    pub fn get_vec_elem_ty(&self, ty: TypeId) -> Option<TypeId> {
        match self.type_repr(ty) {
            TypeRepr::Vector(elem) => Some(elem),
            _ => None,
        }
    }

    /// Gets the types from a TypeVecId.
    /// Returns a slice of TypeIds.
    ///
    /// # Panics
    /// Panics if the TypeVecId is invalid (not created through this pool).
    #[inline]
    pub fn get_type_vec(&self, id: TypeVecId) -> Arc<[TypeId]> {
        self.ty_vec_interner.get_vec_arc(id)
    }

    /// Returns the abilities for a type.
    /// Abilities are pre-computed during type interning, so this is a simple lookup.
    /// Optimized for primitives using direct TypeId comparisons.
    #[inline]
    pub fn abilities(&self, ty: TypeId) -> AbilitySet {
        // primitives (BOOL..=ADDRESS are contiguous)
        if ty >= TypeId::BOOL && ty <= TypeId::ADDRESS {
            return AbilitySet::PRIMITIVES;
        }
        if ty == TypeId::SIGNER {
            return AbilitySet::SIGNER;
        }
        if ty.is_ref() || ty.is_mut_ref() {
            return AbilitySet::REFERENCES;
        }

        let snap = self.ty_interner.fwd.snap.load();
        let idx = ty.0 as usize;
        let used = snap.used.load(Ordering::Acquire);
        assert!(idx < used, "invalid TypeId payload {} in abilities()", idx);
        // used was loaded with Acquire above; metadata can be Relaxed
        let m0 = snap.metadata_0[idx].load(Ordering::Relaxed);
        abilities_from_meta0(m0)
    }

    /// Checks if a type has a specific ability.
    /// This is a convenience wrapper around `abilities()`.
    #[inline]
    pub fn has_ability(&self, ty: TypeId, ability: Ability) -> bool {
        self.abilities(ty).has_ability(ability)
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
        self.ty_interner
            .intern_function(args_id, results_id, abilities)
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
        self.ty_interner
            .intern_function(args_id, results_id, abilities)
    }

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
        let snap = self.ty_interner.fwd.snap.load();
        let idx = ty.0 as usize;
        let used = snap.used.load(Ordering::Acquire);
        assert!(
            idx < used,
            "invalid TypeId payload {} in paranoid_check_is_vec_ty",
            idx
        );

        // used was loaded with Acquire above; metadata can be Relaxed
        let m0 = snap.metadata_0[idx].load(Ordering::Relaxed);
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
