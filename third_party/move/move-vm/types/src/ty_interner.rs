// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Data structures and caches for interning types as unique compact identifiers. The lifetime of
//! these caches is tied to the code cache, and is managed externally.

use crate::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndex};
use move_core_types::ability::AbilitySet;
use parking_lot::RwLock;
use std::{cell::UnsafeCell, collections::HashMap};
use triomphe::Arc;

/// Compactly represents a loaded type.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct TypeId(u32);

/// Compactly represents a vector of types.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct TypeVecId(u32);

/// Partially-interned representation containing top-level information.
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
enum TypeRepr {
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

    fn intern(&self, repr: TypeRepr) -> TypeId {
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
        // Assign id as hot_len + cold_len to preserve TypeId==index after publish append.
        let id = TypeId((self.hot_ref().data.len() + cold.data.len()) as u32);
        cold.data.push(repr);
        cold.interned.insert(repr, id);
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
        let moved_data = std::mem::take(&mut cold.data);
        let moved_map = std::mem::take(&mut cold.interned);
        drop(cold);

        let hot: &mut InternMap<TypeRepr, TypeId> = unsafe { &mut *self.hot.get() };
        hot.data.reserve(moved_data.len());
        for k in moved_data {
            hot.data.push(k);
        }
        hot.interned.reserve(moved_map.len());
        for (k, v) in moved_map {
            // assert!(!hot.interned.contains_key(&k));
            hot.interned.insert(k, v);
        }
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
        let moved_data = std::mem::take(&mut cold.data);
        let moved_map = std::mem::take(&mut cold.interned);
        drop(cold);

        let hot: &mut InternMap<Arc<[TypeId]>, TypeVecId> = unsafe { &mut *self.hot.get() };
        hot.data.reserve(moved_data.len());
        for k in moved_data {
            hot.data.push(k);
        }
        hot.interned.reserve(moved_map.len());
        for (k, v) in moved_map {
            // assert!(!hot.interned.contains_key(&k));
            hot.interned.insert(k, v);
        }
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
        self.ty_interner.intern(TypeRepr::Bool);
        let u8_id = self.ty_interner.intern(TypeRepr::U8);
        self.ty_interner.intern(TypeRepr::U16);
        self.ty_interner.intern(TypeRepr::U32);
        let u64_id = self.ty_interner.intern(TypeRepr::U64);
        self.ty_interner.intern(TypeRepr::U128);
        self.ty_interner.intern(TypeRepr::U256);
        self.ty_interner.intern(TypeRepr::I8);
        self.ty_interner.intern(TypeRepr::I16);
        self.ty_interner.intern(TypeRepr::I32);
        self.ty_interner.intern(TypeRepr::I64);
        self.ty_interner.intern(TypeRepr::I128);
        self.ty_interner.intern(TypeRepr::I256);
        self.ty_interner.intern(TypeRepr::Address);
        self.ty_interner.intern(TypeRepr::Signer);

        self.ty_vec_interner.intern(&[]);
        self.ty_vec_interner.intern(&[u8_id]);
        self.ty_vec_interner.intern(&[u64_id]);
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

    /// Given a type containing type parameters, and a fully-interned type arguments, performs
    /// type substitution with interning.
    pub fn instantiate_and_intern(&self, ty: &Type, subst: &[TypeId]) -> TypeId {
        use Type::*;
        match ty {
            Bool => self.ty_interner.intern(TypeRepr::Bool),
            U8 => self.ty_interner.intern(TypeRepr::U8),
            U16 => self.ty_interner.intern(TypeRepr::U16),
            U32 => self.ty_interner.intern(TypeRepr::U32),
            U64 => self.ty_interner.intern(TypeRepr::U64),
            U128 => self.ty_interner.intern(TypeRepr::U128),
            U256 => self.ty_interner.intern(TypeRepr::U256),
            I8 => self.ty_interner.intern(TypeRepr::I8),
            I16 => self.ty_interner.intern(TypeRepr::I16),
            I32 => self.ty_interner.intern(TypeRepr::I32),
            I64 => self.ty_interner.intern(TypeRepr::I64),
            I128 => self.ty_interner.intern(TypeRepr::I128),
            I256 => self.ty_interner.intern(TypeRepr::I256),
            Address => self.ty_interner.intern(TypeRepr::Address),
            Signer => self.ty_interner.intern(TypeRepr::Signer),
            TyParam(idx) => subst[*idx as usize],
            Vector(elem_ty) => {
                let id = self.instantiate_and_intern(elem_ty, subst);
                self.vec_of(id)
            },
            Reference(inner_ty) => {
                let id = self.instantiate_and_intern(inner_ty, subst);
                self.ref_of(id)
            },
            MutableReference(inner_ty) => {
                let id = self.instantiate_and_intern(inner_ty, subst);
                self.ref_mut_of(id)
            },
            Struct { idx, .. } => self.struct_of(*idx),
            StructInstantiation { idx, ty_args, .. } => {
                let ty_args = ty_args
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                self.instantiated_struct_of(*idx, ty_args)
            },
            Function {
                args,
                results,
                abilities,
            } => {
                let args = args
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                let results = results
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                self.ty_interner.intern(TypeRepr::Function {
                    args: self.ty_vec_interner.intern_vec(args),
                    results: self.ty_vec_interner.intern_vec(results),
                    abilities: *abilities,
                })
            },
        }
    }

    fn ref_of(&self, t: TypeId) -> TypeId {
        self.ty_interner.intern(TypeRepr::Reference(t))
    }

    fn ref_mut_of(&self, t: TypeId) -> TypeId {
        self.ty_interner.intern(TypeRepr::MutableReference(t))
    }

    fn vec_of(&self, t: TypeId) -> TypeId {
        self.ty_interner.intern(TypeRepr::Vector(t))
    }

    fn struct_of(&self, idx: StructNameIndex) -> TypeId {
        self.ty_interner.intern(TypeRepr::Struct {
            idx,
            ty_args: self.ty_vec_interner.intern(&[]),
        })
    }

    fn instantiated_struct_of(&self, idx: StructNameIndex, ty_args: Vec<TypeId>) -> TypeId {
        let ty_args = self.ty_vec_interner.intern_vec(ty_args);
        self.ty_interner.intern(TypeRepr::Struct { idx, ty_args })
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
