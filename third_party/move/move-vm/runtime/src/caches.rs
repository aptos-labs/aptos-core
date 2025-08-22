// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{frame_type_cache::FrameTypeCache, Function, LoadedFunction};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_vm_types::loaded_data::runtime_types::Type;
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{Hash, Hasher},
    rc::Rc,
    sync::Arc,
};

/// Stable pointer identity for a Function within a process.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct FunctionPtr(*const Function);

impl FunctionPtr {
    pub fn from_loaded(function: &LoadedFunction) -> Self {
        // Pointer identity of the Arc<Function> inside LoadedFunction.
        FunctionPtr(Arc::as_ptr(&function.function))
    }
}

impl Hash for FunctionPtr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.0 as usize);
    }
}

/// Fingerprint over a fully-resolved type-argument vector.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct TyArgsFingerprint {
    pub hash: u64,
    pub len: u16,
}

impl TyArgsFingerprint {
    pub fn new(hash: u64, len: usize) -> Self {
        TyArgsFingerprint {
            hash,
            len: len as u16,
        }
    }
}

/// Generic function cache key (function identity plus ty-args fingerprint).
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub struct GenericFunctionKey {
    pub function: FunctionPtr,
    pub fingerprint: TyArgsFingerprint,
}

/// Interpreter-level caches for per-function data.
pub struct InterpreterCaches {
    non_generic: HashMap<FunctionPtr, Rc<RefCell<FrameTypeCache>>>,
    generic: HashMap<GenericFunctionKey, Rc<RefCell<FrameTypeCache>>>,
}

impl InterpreterCaches {
    pub fn new() -> Self {
        Self {
            non_generic: HashMap::new(),
            generic: HashMap::new(),
        }
    }

    /// Get or create the frame cache for a non-generic function.
    pub fn get_or_create_frame_cache(
        &mut self,
        function: &LoadedFunction,
    ) -> Rc<RefCell<FrameTypeCache>> {
        debug_assert!(function.ty_args().is_empty());
        let fptr = FunctionPtr::from_loaded(function);
        self.non_generic
            .entry(fptr)
            .or_insert_with(|| FrameTypeCache::make_rc_for_function(function))
            .clone()
    }

    /// Get or create the frame cache for a generic function (ty-args must be fully instantiated).
    pub fn get_or_create_frame_cache_generic(
        &mut self,
        function: &LoadedFunction,
    ) -> PartialVMResult<Rc<RefCell<FrameTypeCache>>> {
        debug_assert!(!function.ty_args().is_empty());
        let fptr = FunctionPtr::from_loaded(function);
        let fp = function.ty_args_fingerprint.as_ref().copied().ok_or_else(|| PartialVMError::new_invariant_violation("missing ty_args_fingerprint for generic function"))?;
        let key = GenericFunctionKey {
            function: fptr,
            fingerprint: fp,
        };
        Ok(self
            .generic
            .entry(key)
            .or_insert_with(|| FrameTypeCache::make_rc_for_function(function))
            .clone())
    }
}

/// Utility: compute a simple structural hash over type arguments.
/// NOTE: placeholder API; final implementation will live alongside the interner
/// and avoid allocations.
pub fn fingerprint_ty_args(tys: &[Type]) -> PartialVMResult<TyArgsFingerprint> {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    tys.len().hash(&mut hasher);
    for ty in tys {
        hash_type(ty, &mut hasher)?;
    }
    Ok(TyArgsFingerprint::new(hasher.finish(), tys.len()))
}

fn hash_type<H: Hasher>(ty: &Type, state: &mut H) -> PartialVMResult<()> {
    // Distinct tags per constructor
    const T_BOOL: u8 = 1;
    const T_U8: u8 = 2;
    const T_U16: u8 = 3;
    const T_U32: u8 = 4;
    const T_U64: u8 = 5;
    const T_U128: u8 = 6;
    const T_U256: u8 = 7;
    const T_ADDRESS: u8 = 8;
    const T_SIGNER: u8 = 9;
    const T_VECTOR: u8 = 10;
    const T_STRUCT: u8 = 11;
    const T_STRUCT_INST: u8 = 12;
    const T_REF: u8 = 13;
    const T_MUT_REF: u8 = 14;
    const T_FUNCTION: u8 = 15;

    match ty {
        Type::Bool => state.write_u8(T_BOOL),
        Type::U8 => state.write_u8(T_U8),
        Type::U16 => state.write_u8(T_U16),
        Type::U32 => state.write_u8(T_U32),
        Type::U64 => state.write_u8(T_U64),
        Type::U128 => state.write_u8(T_U128),
        Type::U256 => state.write_u8(T_U256),
        Type::Address => state.write_u8(T_ADDRESS),
        Type::Signer => state.write_u8(T_SIGNER),
        Type::Vector(elem) => {
            state.write_u8(T_VECTOR);
            hash_type(elem, state)?;
        },
        Type::Struct { idx, .. } => {
            state.write_u8(T_STRUCT);
            idx.hash(state);
        },
        Type::StructInstantiation { idx, ty_args, .. } => {
            state.write_u8(T_STRUCT_INST);
            idx.hash(state);
            ty_args.len().hash(state);
            for a in ty_args.iter() {
                hash_type(a, state)?;
            }
        },
        Type::Reference(inner) => {
            state.write_u8(T_REF);
            hash_type(inner, state)?;
        },
        Type::MutableReference(inner) => {
            state.write_u8(T_MUT_REF);
            hash_type(inner, state)?;
        },
        Type::TyParam(_) => {
            return Err(PartialVMError::new_invariant_violation(
                "Type parameter encountered in fingerprint_ty_args; expected fully-instantiated types",
            ));
        },
        Type::Function {
            args,
            results,
            abilities,
        } => {
            state.write_u8(T_FUNCTION);
            abilities.hash(state);
            args.len().hash(state);
            for a in args {
                hash_type(a, state)?;
            }
            results.len().hash(state);
            for r in results {
                hash_type(r, state)?;
            }
        },
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::ability::AbilitySet;
    use move_vm_types::loaded_data::{
        runtime_types::{AbilityInfo, TypeBuilder},
        struct_name_indexing::StructNameIndex,
    };
    use triomphe::Arc as TriArc;

    #[test]
    fn fingerprint_basic_equality_and_variation() {
        let v1 = vec![Type::U8, Type::U64];
        let v2 = vec![Type::U8, Type::U64];
        let v3 = vec![Type::U64, Type::U8];
        let f1 = fingerprint_ty_args(&v1).unwrap();
        let f2 = fingerprint_ty_args(&v2).unwrap();
        let f3 = fingerprint_ty_args(&v3).unwrap();
        assert_eq!(f1, f2);
        assert_ne!(f1, f3);
    }

    #[test]
    fn fingerprint_vectors_and_refs() {
        let vec_u8 = Type::Vector(TriArc::new(Type::U8));
        let vec_u64 = Type::Vector(TriArc::new(Type::U64));
        assert_ne!(
            fingerprint_ty_args(&[vec_u8.clone()]).unwrap(),
            fingerprint_ty_args(&[vec_u64]).unwrap()
        );

        let r1 = Type::Reference(Box::new(Type::U8));
        let r2 = Type::MutableReference(Box::new(Type::U8));
        assert_ne!(
            fingerprint_ty_args(&[r1]).unwrap(),
            fingerprint_ty_args(&[r2]).unwrap()
        );
    }

    #[test]
    fn fingerprint_structs_and_instantiations() {
        let tb = TypeBuilder::with_limits(64, 16);
        let idx_a = StructNameIndex::new(0);
        let idx_b = StructNameIndex::new(1);
        let s_a = tb.create_struct_ty(idx_a, AbilityInfo::struct_(AbilitySet::EMPTY));
        let s_b = tb.create_struct_ty(idx_b, AbilityInfo::struct_(AbilitySet::EMPTY));
        assert_ne!(
            fingerprint_ty_args(&[s_a.clone()]).unwrap(),
            fingerprint_ty_args(&[s_b]).unwrap()
        );

        // A<T> with T = u8 vs u64 must differ
        let a_u8 = tb
            .create_struct_instantiation_ty(&s_a, &[Type::TyParam(0)], &[Type::U8])
            .unwrap();
        let a_u64 = tb
            .create_struct_instantiation_ty(&s_a, &[Type::TyParam(0)], &[Type::U64])
            .unwrap();
        assert_ne!(
            fingerprint_ty_args(&[a_u8]).unwrap(),
            fingerprint_ty_args(&[a_u64]).unwrap()
        );

        // TyParam should error
        assert!(fingerprint_ty_args(&[Type::TyParam(0)]).is_err());
    }
}
