// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::loaded_data::runtime_types::Type;
use once_cell::sync::Lazy;
use std::hash::{BuildHasher, Hash, Hasher};

/// Fingerprint over a fully-resolved type-argument vector.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub struct TyArgsFingerprint {
    hash: u64,
    num_args: u64,
}

/// For fingerprints, we ensure that on each machine they are different. This way in case of a hash
/// collision over type arguments, this salt will be different.
static TY_ARGS_SALT: Lazy<u64> = Lazy::new(|| {
    use std::collections::hash_map::RandomState;
    RandomState::new().build_hasher().finish()
});

impl TyArgsFingerprint {
    /// Given function type arguments, constructs its fingerprint. Panics if any of the type
    /// arguments are not fully resolved.
    pub fn from_ty_args(ty_args: &[Type]) -> Self {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        TY_ARGS_SALT.hash(&mut hasher);

        ty_args.len().hash(&mut hasher);
        for ty in ty_args {
            Self::hash_ty_arg(ty, &mut hasher);
        }
        Self {
            hash: hasher.finish(),
            num_args: ty_args.len() as u64,
        }
    }

    fn hash_ty_arg<H: Hasher>(ty: &Type, state: &mut H) {
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
                Self::hash_ty_arg(elem, state);
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
                    Self::hash_ty_arg(a, state);
                }
            },
            Type::Reference(inner) => {
                state.write_u8(T_REF);
                Self::hash_ty_arg(inner, state);
            },
            Type::MutableReference(inner) => {
                state.write_u8(T_MUT_REF);
                Self::hash_ty_arg(inner, state);
            },
            Type::TyParam(_) => unreachable!("Type arguments are always fully-resolved"),
            Type::Function {
                args,
                results,
                abilities,
            } => {
                state.write_u8(T_FUNCTION);
                args.len().hash(state);
                for a in args {
                    Self::hash_ty_arg(a, state);
                }
                results.len().hash(state);
                for r in results {
                    Self::hash_ty_arg(r, state);
                }
                // Note: we hash abilities because types Foo<|| has key> and Foo<||> are different.
                abilities.hash(state);
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loaded_data::{runtime_types::AbilityInfo, struct_name_indexing::StructNameIndex};
    use move_core_types::ability::AbilitySet;
    use proptest::prelude::*;
    use std::collections::HashSet;
    use triomphe::Arc;

    fn make_generic_struct_ty(idx: usize, ty_args: Vec<Type>) -> Type {
        Type::StructInstantiation {
            idx: StructNameIndex::new(idx),
            ty_args: Arc::new(ty_args),
            ability: AbilityInfo::struct_(AbilitySet::EMPTY),
        }
    }

    #[test]
    fn test_primitive_types_have_different_fingerprints() {
        let types = vec![
            vec![Type::Bool],
            vec![Type::U8],
            vec![Type::U16],
            vec![Type::U32],
            vec![Type::U64],
            vec![Type::U128],
            vec![Type::U256],
            vec![Type::Address],
            vec![Type::Signer],
        ];

        let mut fingerprints = HashSet::new();
        for ty_args in types {
            let fingerprint = TyArgsFingerprint::from_ty_args(&ty_args);
            assert!(fingerprints.insert(fingerprint));
        }
    }

    #[test]
    fn test_vector_nesting_produces_different_fingerprints() {
        let vec_u64 = vec![Type::Vector(Arc::new(Type::U64))];
        let vec_vec_u64 = vec![Type::Vector(Arc::new(Type::Vector(Arc::new(Type::U64))))];

        let fp1 = TyArgsFingerprint::from_ty_args(&vec_u64);
        let fp2 = TyArgsFingerprint::from_ty_args(&vec_vec_u64);

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_reference_types_are_distinct() {
        let types = vec![
            vec![Type::U64],
            vec![Type::Reference(Box::new(Type::U64))],
            vec![Type::MutableReference(Box::new(Type::U64))],
        ];

        let mut fingerprints = HashSet::new();
        for ty_args in types {
            let fingerprint = TyArgsFingerprint::from_ty_args(&ty_args);
            assert!(fingerprints.insert(fingerprint));
        }
    }

    #[test]
    fn test_struct_instantiations_are_distinct() {
        let types = vec![
            vec![make_generic_struct_ty(1, vec![Type::U64])],
            vec![make_generic_struct_ty(1, vec![Type::U128])],
            vec![make_generic_struct_ty(2, vec![Type::U64])],
        ];

        let mut fingerprints = HashSet::new();
        for ty_args in types {
            let fingerprint = TyArgsFingerprint::from_ty_args(&ty_args);
            assert!(fingerprints.insert(fingerprint));
        }
    }

    #[test]
    fn test_function_types_are_distinct() {
        let types = vec![
            vec![Type::Function {
                args: vec![Type::U64],
                results: vec![Type::U64],
                abilities: AbilitySet::EMPTY,
            }],
            vec![Type::Function {
                args: vec![Type::U64],
                results: vec![Type::U64],
                abilities: AbilitySet::ALL,
            }],
            vec![Type::Function {
                args: vec![Type::U64, Type::U64],
                results: vec![Type::U64],
                abilities: AbilitySet::EMPTY,
            }],
            vec![Type::Function {
                args: vec![Type::U64],
                results: vec![Type::U128],
                abilities: AbilitySet::EMPTY,
            }],
        ];

        let mut fingerprints = HashSet::new();
        for ty_args in types {
            let fingerprint = TyArgsFingerprint::from_ty_args(&ty_args);
            assert!(fingerprints.insert(fingerprint));
        }
    }

    #[test]
    fn test_order_sensitivity() {
        let types1 = vec![Type::U64, Type::U128];
        let types2 = vec![Type::U128, Type::U64];

        let fp1 = TyArgsFingerprint::from_ty_args(&types1);
        let fp2 = TyArgsFingerprint::from_ty_args(&types2);

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_length_sensitivity() {
        let types1 = vec![Type::U64];
        let types2 = vec![Type::U64, Type::U64];

        let fp1 = TyArgsFingerprint::from_ty_args(&types1);
        let fp2 = TyArgsFingerprint::from_ty_args(&types2);

        assert_ne!(fp1, fp2);
    }

    #[test]
    #[should_panic]
    fn test_type_param_panics() {
        let _ = TyArgsFingerprint::from_ty_args(&[Type::TyParam(0)]);
    }

    prop_compose! {
        fn arb_primitive_type()(
            variant in 0u8..9
        ) -> Type {
            match variant {
                0 => Type::Bool,
                1 => Type::U8,
                2 => Type::U16,
                3 => Type::U32,
                4 => Type::U64,
                5 => Type::U128,
                6 => Type::U256,
                7 => Type::Address,
                8 => Type::Signer,
                _ => unreachable!(),
            }
        }
    }

    prop_compose! {
        fn arb_simple_type(depth: u32)(
            base in arb_primitive_type(),
            make_vector in any::<bool>(),
            make_ref in 0u8..3,
            depth in Just(depth)
        ) -> Type {
            if depth == 0 {
                base
            } else {
                match (make_vector, make_ref) {
                    (true, _) => Type::Vector(Arc::new(base)),
                    (false, 1) => Type::Reference(Box::new(base)),
                    (false, 2) => Type::MutableReference(Box::new(base)),
                    _ => base,
                }
            }
        }
    }

    prop_compose! {
        fn arb_type_args()(
            types in prop::collection::vec(arb_simple_type(2), 0..5)
        ) -> Vec<Type> {
            types
        }
    }

    proptest! {
        #[test]
        fn prop_same_types_same_fingerprint(ty_args in arb_type_args()) {
            let fp1 = TyArgsFingerprint::from_ty_args(&ty_args);
            let fp2 = TyArgsFingerprint::from_ty_args(&ty_args);
            assert_eq!(fp1, fp2);
        }

        #[test]
        fn prop_different_types_different_fingerprints(
            ty_args1 in arb_type_args(),
            ty_args2 in arb_type_args()
        ) {
            prop_assume!(ty_args1 != ty_args2);

            let fp1 = TyArgsFingerprint::from_ty_args(&ty_args1);
            let fp2 = TyArgsFingerprint::from_ty_args(&ty_args2);
            assert_ne!(fp1, fp2);
        }
    }
}
