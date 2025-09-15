// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use crate::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndex};
use parking_lot::RwLock;

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
    },
}

struct InternMap<T, I> {
    map: HashMap<T, I>,
    store: Vec<T>,
}

impl<T, I> Default for InternMap<T, I> {
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            store: Vec::with_capacity(16),
        }
    }
}

struct TypeInterner {
    inner: RwLock<InternMap<TypeRepr, TypeId>>
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
        if let Some(id) = self.inner.read().map.get(&repr) {
            return *id;
        }

        let repr_key = repr.clone();
        let mut inner = self.inner.write();
        if let Some(id) = inner.map.get(&repr) {
            return *id;
        }

        let id = TypeId(inner.store.len() as u32);
        inner.store.push(repr);
        inner.map.insert(repr_key, id);
        id
    }
}

struct TypeVecInterner {
    inner: RwLock<InternMap<Vec<TypeId>, TypeVecId>>,
}

impl Default for TypeVecInterner {
    fn default() -> Self {
        Self {
            inner: RwLock::new(InternMap::default()),
        }
    }
}

impl TypeVecInterner {
    fn intern(&self, tys: Vec<TypeId>) -> TypeVecId {
        if let Some(id) = self.inner.read().map.get(&tys) {
            return *id;
        }

        let tys_key = tys.clone();
        let mut inner = self.inner.write();
        if let Some(id) = inner.map.get(&tys) {
            return *id;
        }

        let id = TypeVecId(inner.store.len() as u32);
        inner.store.push(tys);
        inner.map.insert(tys_key, id);
        id
    }
}

pub struct Prim {
    pub bool: TypeId,
    pub u8: TypeId,
    pub u16: TypeId,
    pub u32: TypeId,
    pub u64: TypeId,
    pub u128: TypeId,
    pub u256: TypeId,
    pub address: TypeId,
    pub signer: TypeId,
    pub no_ty_args: TypeVecId,
}

pub struct TypeContext {
    ty_interner: TypeInterner,
    ty_vec_interner: TypeVecInterner,
    prim: Prim,
}

impl TypeContext {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let ty_interner = TypeInterner::default();
        let ty_vec_interner = TypeVecInterner::default();
        let prim = Prim {
            bool: ty_interner.intern(TypeRepr::Bool),
            u8: ty_interner.intern(TypeRepr::U8),
            u16: ty_interner.intern(TypeRepr::U16),
            u32: ty_interner.intern(TypeRepr::U32),
            u64: ty_interner.intern(TypeRepr::U64),
            u128: ty_interner.intern(TypeRepr::U128),
            u256: ty_interner.intern(TypeRepr::U256),
            address: ty_interner.intern(TypeRepr::Address),
            signer: ty_interner.intern(TypeRepr::Signer),
            no_ty_args: ty_vec_interner.intern(vec![]),
        };
        Self {
            ty_interner,
            ty_vec_interner,
            prim,
        }
    }

    pub fn intern_ty_args(&self, ty_args: &[Type]) -> TypeVecId {
        let ty_args = ty_args
            .iter()
            .map(|t| self.instantiate_and_intern(t, &[]))
            .collect::<Vec<_>>();
        self.ty_vec_interner.intern(ty_args)
    }

    pub fn instantiate_and_intern(&self, ty: &Type, subst: &[TypeId]) -> TypeId {
        use Type::*;
        match ty {
            Bool => self.prim.bool,
            U8 => self.prim.u8,
            U16 => self.prim.u16,
            U32 => self.prim.u32,
            U64 => self.prim.u64,
            U128 => self.prim.u128,
            U256 => self.prim.u256,
            Address => self.prim.address,
            Signer => self.prim.signer,
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
            Function { args, results, .. } => {
                let args = args
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                let results = results
                    .iter()
                    .map(|t| self.instantiate_and_intern(t, subst))
                    .collect::<Vec<_>>();
                self.ty_interner.intern(TypeRepr::Function {
                    args: self.ty_vec_interner.intern(args),
                    results: self.ty_vec_interner.intern(results),
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
            ty_args: self.prim.no_ty_args,
        })
    }

    fn instantiated_struct_of(&self, idx: StructNameIndex, ty_args: Vec<TypeId>) -> TypeId {
        let ty_args = self.ty_vec_interner.intern(ty_args);
        self.ty_interner.intern(TypeRepr::Struct { idx, ty_args })
    }
}
