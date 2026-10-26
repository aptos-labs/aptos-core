// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Data structures and caches for interning types as unique compact identifiers. The lifetime of
//! these caches is tied to the code cache, and is managed externally.

use crate::loaded_data::{
    runtime_types::{InternedNode, Type},
    struct_name_indexing::StructNameIndex,
};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::ability::AbilitySet;
use parking_lot::RwLock;
use std::collections::HashMap;
use triomphe::Arc;

/// Compactly represents a loaded type.
#[repr(transparent)]
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
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
    }

    /// Given a vector of fully-instantiated type arguments, returns the corresponding
    /// [TypeVecId]. Returns an invariant violation if there are non-instantiated type arguments.
    pub fn intern_ty_args(&self, ty_args: &[Type]) -> PartialVMResult<TypeVecId> {
        if ty_args.iter().any(|t| t.type_info().has_ty_params()) {
            return Err(PartialVMError::new_invariant_violation(format!(
                "Cannot intern type arguments with type parameters: {:?}",
                ty_args
            )));
        }
        let ty_args = ty_args
            .iter()
            .map(|t| self.instantiate_and_intern(t, &[]))
            .collect::<Vec<_>>();
        Ok(self.ty_vec_interner.intern_vec(ty_args))
    }

    /// Returns `None` for types with type parameters.
    pub fn intern_ty(&self, ty: &Type) -> Option<TypeId> {
        if ty.type_info().has_ty_params() {
            return None;
        }
        Some(self.instantiate_and_intern(ty, &[]))
    }

    /// Fills the id cells of a type so that later equality checks are O(1).
    pub fn warm_ty(&self, ty: &Type) {
        use Type::*;
        match ty {
            Vector(_) | StructInstantiation(_) | Function(_) => {
                self.intern_ty(ty);
            },
            Reference(node) | MutableReference(node) => self.warm_ty(node.ty()),
            Bool
            | U8
            | U16
            | U32
            | U64
            | U128
            | U256
            | I8
            | I16
            | I32
            | I64
            | I128
            | I256
            | Address
            | Signer
            | Struct { .. }
            | TyParam(_) => {},
        }
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
            Vector(node) => self.id_of_node(node.as_ref(), || {
                let id = self.instantiate_and_intern(node.ty(), subst);
                self.vec_of(id)
            }),
            // No cell: mutable and immutable references can share one node.
            Reference(node) => {
                let id = self.instantiate_and_intern(node.ty(), subst);
                self.ref_of(id)
            },
            MutableReference(node) => {
                let id = self.instantiate_and_intern(node.ty(), subst);
                self.ref_mut_of(id)
            },
            Struct { idx, .. } => self.struct_of(*idx),
            StructInstantiation(node) => self.id_of_node(node.as_ref(), || {
                let ids = node
                    .ty_args()
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                self.instantiated_struct_of(node.idx(), ids)
            }),
            Function(node) => self.id_of_node(node.as_ref(), || {
                let args = node
                    .args()
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                let results = node
                    .results()
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                self.ty_interner.intern(TypeRepr::Function {
                    args: self.ty_vec_interner.intern_vec(args),
                    results: self.ty_vec_interner.intern_vec(results),
                    abilities: node.abilities(),
                })
            }),
        }
    }

    /// Reads or fills the node's id cell; generic nodes are never filled.
    #[inline(always)]
    fn id_of_node<N: InternedNode>(&self, node: &N, compute: impl FnOnce() -> TypeId) -> TypeId {
        if node.interned_info().has_ty_params() {
            return compute();
        }
        if let Some(id) = node.cached_id() {
            return id;
        }
        let id = compute();
        node.fill_id(id);
        id
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
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::loaded_data::{runtime_types::AbilityInfo, struct_name_indexing::StructNameIndex};
    use move_core_types::ability::{Ability, AbilitySet};
    use std::{
        collections::{hash_map::DefaultHasher, HashSet},
        hash::Hasher,
        thread,
    };

    // ---- Invariants of the lazily-interned id cells on compound nodes ----

    fn deep_ty() -> Type {
        Type::struct_instantiation(
            StructNameIndex::new(7),
            vec![Type::vector(Type::vector(Type::U8)), Type::Bool],
            AbilityInfo::struct_(AbilitySet::EMPTY),
        )
    }

    fn hash_of<T: std::hash::Hash>(t: &T) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }

    #[test]
    fn interning_fills_id_cell_and_keeps_eq_hash_consistent() {
        let ctx = InternedTypePool::new();
        // Structurally equal, separately allocated; neither interned yet.
        let a = deep_ty();
        let b = deep_ty();
        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));
        assert!(
            !matches!(&a, Type::StructInstantiation(node) if node.cached_ty_id().is_some()),
            "id cell must start empty"
        );

        // Intern only `a`: Eq/Hash must be unaffected by asymmetric interning.
        let id_a = ctx.instantiate_and_intern(&a, &[]);
        assert!(
            matches!(&a, Type::StructInstantiation(node) if node.cached_ty_id() == Some(id_a)),
            "interning must fill the cell with the returned id"
        );
        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));

        // Both interned: Eq is now the id comparison.
        let id_b = ctx.instantiate_and_intern(&b, &[]);
        assert_eq!(id_a, id_b);
        assert_eq!(a, b);

        // Re-interning reads the cell: same id, no new entries.
        let n = ctx.num_interned_tys();
        assert_eq!(ctx.instantiate_and_intern(&a, &[]), id_a);
        assert_eq!(ctx.num_interned_tys(), n);
    }

    #[test]
    fn interning_does_not_fill_id_cell_for_generic_nodes() {
        let ctx = InternedTypePool::new();
        // vector<T>: its id depends on the substitution.
        let generic = Type::vector(Type::TyParam(0));
        let Type::Vector(node) = &generic else {
            panic!("expected a vector")
        };

        let u8_id = ctx.instantiate_and_intern(&Type::U8, &[]);
        let bool_id = ctx.instantiate_and_intern(&Type::Bool, &[]);
        let as_u8 = ctx.instantiate_and_intern(&generic, &[u8_id]);
        let as_bool = ctx.instantiate_and_intern(&generic, &[bool_id]);
        assert_ne!(
            as_u8, as_bool,
            "different substitutions must intern differently"
        );
        assert!(
            node.cached_ty_id().is_none(),
            "cell must stay empty for generic nodes"
        );

        // Same ids as the concrete types.
        assert_eq!(
            as_u8,
            ctx.instantiate_and_intern(&Type::vector(Type::U8), &[])
        );
        assert_eq!(
            as_bool,
            ctx.instantiate_and_intern(&Type::vector(Type::Bool), &[])
        );

        // `intern_ty` leaves generic nodes untouched too.
        ctx.intern_ty(&generic);
        assert!(node.cached_ty_id().is_none());
    }

    #[test]
    fn function_nodes_with_different_abilities_are_distinct() {
        let ctx = InternedTypePool::new();
        // Same signature, different abilities: distinct types, nodes and ids.
        let f_copy = Type::function(
            vec![Type::U64],
            vec![Type::U8],
            AbilitySet::singleton(Ability::Copy),
        );
        let f_drop = Type::function(
            vec![Type::U64],
            vec![Type::U8],
            AbilitySet::singleton(Ability::Drop),
        );
        assert_ne!(f_copy, f_drop, "abilities are part of the function type");
        let id_copy = ctx.instantiate_and_intern(&f_copy, &[]);
        let id_drop = ctx.instantiate_and_intern(&f_drop, &[]);
        assert_ne!(id_copy, id_drop, "interning must distinguish the two");
        // Abilities live in the node, so each node caches its own id.
        let (Type::Function(n1), Type::Function(n2)) = (&f_copy, &f_drop) else {
            panic!("expected function types")
        };
        assert_eq!(n1.cached_ty_id(), Some(id_copy));
        assert_eq!(n2.cached_ty_id(), Some(id_drop));
        assert_ne!(n1, n2, "nodes with different abilities are distinct");
    }

    #[test]
    fn frozen_reference_equals_separately_constructed_reference_across_interning() {
        let ctx = InternedTypePool::new();

        // Freeze first, then intern both sides.
        let mut_ref = Type::mut_reference(Type::vector(Type::U8));
        let frozen = mut_ref
            .clone()
            .paranoid_freeze_ref_ty()
            .expect("freeze must succeed on a mutable reference");
        let imm_ref = Type::reference(Type::vector(Type::U8));
        ctx.intern_ty(&mut_ref);
        ctx.intern_ty(&frozen);
        ctx.intern_ty(&imm_ref);
        assert_eq!(
            frozen, imm_ref,
            "frozen and separately built references are equal"
        );

        // Intern the mutable view first, then freeze.
        let mut_ref2 = Type::mut_reference(Type::vector(Type::U8));
        ctx.intern_ty(&mut_ref2);
        let frozen2 = mut_ref2
            .paranoid_freeze_ref_ty()
            .expect("freeze must succeed on a mutable reference");
        assert_eq!(
            frozen2, imm_ref,
            "the frozen view must equal a fresh immutable reference"
        );

        // The wrappers remain distinct types.
        assert_ne!(mut_ref, frozen);
    }

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

        let vec_u8 = Type::vector(Type::U8);
        let vec_u64 = Type::vector(Type::U64);

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

        let u64_ref = Type::reference(Type::U64);
        let u64_mut_ref = Type::mut_reference(Type::U64);
        let u8_ref = Type::reference(Type::U8);

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

        let struct_ty = Type::struct_instantiation(
            StructNameIndex::new(0),
            vec![Type::U64, Type::Bool],
            // Irrelevant for tests.
            AbilityInfo::struct_(AbilitySet::EMPTY),
        );

        let id1 = ctx.instantiate_and_intern(&struct_ty, &[]);
        let id2 = ctx.instantiate_and_intern(&struct_ty, &[]);
        assert_eq!(id1, id2);

        let struct_inst2 = Type::struct_instantiation(
            StructNameIndex::new(0),
            vec![Type::Bool, Type::U64],
            // Irrelevant for tests.
            AbilityInfo::struct_(AbilitySet::EMPTY),
        );

        let id3 = ctx.instantiate_and_intern(&struct_inst2, &[]);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_function_types() {
        let ctx = InternedTypePool::new();

        let func_ty = Type::function(
            vec![Type::U64, Type::Bool],
            vec![Type::U8],
            AbilitySet::EMPTY,
        );

        let id1 = ctx.instantiate_and_intern(&func_ty, &[]);
        let id2 = ctx.instantiate_and_intern(&func_ty, &[]);

        assert_eq!(id1, id2);

        let func_ty = Type::function(vec![Type::U64], vec![Type::U8], AbilitySet::EMPTY);

        let id3 = ctx.instantiate_and_intern(&func_ty, &[]);
        assert_ne!(id1, id3);

        let func_ty = Type::function(vec![Type::U64], vec![Type::U8], AbilitySet::ALL);
        let id4 = ctx.instantiate_and_intern(&func_ty, &[]);
        assert_ne!(id3, id4);
    }

    #[test]
    fn test_deeply_nested_type() {
        let ctx = InternedTypePool::new();

        let mut ty = Type::U64;
        for _ in 0..10 {
            ty = Type::vector(ty);
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
    fn interning_generic_ty_args_is_an_error() {
        let ctx = InternedTypePool::new();
        assert!(ctx.intern_ty_args(&[Type::TyParam(0)]).is_err());
        assert!(ctx
            .intern_ty_args(&[Type::vector(Type::TyParam(1))])
            .is_err());
    }

    #[test]
    fn test_empty_type_vectors() {
        let ctx = InternedTypePool::new();
        let id1 = ctx.intern_ty_args(&[]).unwrap();
        let id2 = ctx.intern_ty_args(&[]).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_type_vector_consistency() {
        let ctx = InternedTypePool::new();

        let u64_ty = Type::U64;
        let bool_ty = Type::Bool;

        let mut tys = vec![u64_ty, bool_ty];
        let id1 = ctx.intern_ty_args(&tys).unwrap();
        let id2 = ctx.intern_ty_args(&tys).unwrap();
        assert_eq!(id1, id2);

        tys.reverse();
        let id3 = ctx.intern_ty_args(&tys).unwrap();
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_flush_clears_cache() {
        let ctx = InternedTypePool::new();

        ctx.intern_ty_args(&[Type::U64]).unwrap();
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
