// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{config::VMConfig, RuntimeEnvironment};
use hashbrown::{hash_map::Entry, HashMap};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_types::{
    loaded_data::struct_name_indexing::StructNameIndex,
    ty_interner::{TypeId, TypeRepr},
};
use parking_lot::RwLock;

struct PseudoGasContext {
    // Parameters for metering type tag construction:
    //   - maximum allowed cost,
    //   - base cost for any type to tag conversion,
    //   - cost for size of a struct tag.
    max_cost: u64,
    cost: u64,
    cost_base: u64,
    cost_per_byte: u64,
}

impl PseudoGasContext {
    fn new(vm_config: &VMConfig) -> Self {
        Self {
            max_cost: vm_config.type_max_cost,
            cost: 0,
            cost_base: vm_config.type_base_cost,
            cost_per_byte: vm_config.type_byte_cost,
        }
    }

    fn current_cost(&mut self) -> u64 {
        self.cost
    }

    fn charge_base(&mut self) -> PartialVMResult<()> {
        self.charge(self.cost_base)
    }

    fn charge_struct_tag(&mut self, struct_tag: &StructTag) -> PartialVMResult<()> {
        let size =
            (struct_tag.address.len() + struct_tag.module.len() + struct_tag.name.len()) as u64;
        self.charge(size * self.cost_per_byte)
    }

    fn charge(&mut self, amount: u64) -> PartialVMResult<()> {
        self.cost += amount;
        if self.cost > self.max_cost {
            Err(
                PartialVMError::new(StatusCode::TYPE_TAG_LIMIT_EXCEEDED).with_message(format!(
                    "Exceeded maximum type tag limit of {} when charging {}",
                    self.max_cost, amount
                )),
            )
        } else {
            Ok(())
        }
    }
}

/// An entry in [TypeTagCache] that also stores a "cost" of the tag. The cost is proportional to
/// the size of the tag, which includes the number of inner nodes and the sum of the sizes in bytes
/// of addresses and identifiers.
#[derive(Debug, Clone)]
pub(crate) struct PricedStructTag {
    pub(crate) struct_tag: StructTag,
    pub(crate) pseudo_gas_cost: u64,
}

/// Cache for struct tags, that can be used safely for concurrent and speculative execution.
///
/// # Speculative execution safety
///
/// A struct name corresponds to a unique [StructNameIndex]. So all non-generic structs with same
/// names have the same struct tags. If structs are generic, the number of type parameters cannot
/// be changed by the upgrade, so the tags stay the same for different "upgraded" struct versions.
/// The type parameters themselves (vector of [Type]s in this cache used as keys) are also not
/// changing.
///
/// Note: even if we allow to add more type parameters (e.g., for enums), it still does not affect
/// safety because different number of type parameters will correspond to a different entries in
/// cache. For example, suppose we have 3 threads, where threads 1 and 2 cache different versions
/// of an enum (see below).
/// ```
/// // Tag cached by thread 1.
/// enum Foo<A> { V1(A), }
/// ```
/// ```
/// // Tag cached by thread 2, where the version of Foo is upgraded with a new variant and new
/// // type parameter (artificial example).
/// enum Foo<A, B> { V1(A), V2(B), }
/// ```
/// If thread 3 reads the tag of this enum, the read result is always **deterministic** for the
/// fixed type parameters used by thread 3.
pub(crate) struct TypeTagCache {
    cache: RwLock<HashMap<TypeId, PricedStructTag>>,
}

impl TypeTagCache {
    /// Creates a new empty cache without any entries.
    pub(crate) fn empty() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    /// Removes all entries from the cache.
    pub(crate) fn flush(&self) {
        self.cache.write().clear();
    }

    /// Returns cached struct tag and its pseudo-gas cost if it exists, and [None] otherwise.
    pub(crate) fn get_struct_tag(&self, ty: TypeId) -> Option<PricedStructTag> {
        self.cache.read().get(&ty).cloned()
    }

    /// Inserts the struct tag and its pseudo-gas cost ([PricedStructTag]) into the cache. Returns
    /// true if the tag was not cached before, and false otherwise.
    pub(crate) fn insert_struct_tag(
        &self,
        ty: TypeId,
        priced_struct_tag: &PricedStructTag,
    ) -> bool {
        // Check if already cached.
        if self.cache.read().contains_key(&ty) {
            return false;
        }

        let priced_struct_tag = priced_struct_tag.clone();

        // Otherwise, we need to insert. We did the clones outside the lock, and also avoid the
        // double insertion.
        let mut cache = self.cache.write();
        if let Entry::Vacant(entry) = cache.entry(ty) {
            entry.insert(priced_struct_tag);
            true
        } else {
            false
        }
    }
}

/// Responsible for building type tags, while also doing the metering in order to bound space and
/// time complexity.
pub(crate) struct TypeTagConverter<'a> {
    /// Stores caches for struct names and tags, as well as pseudo-gas metering configs.
    runtime_environment: &'a RuntimeEnvironment,
}

impl<'a> TypeTagConverter<'a> {
    /// Creates a new converter for the specified environment and configs.
    pub(crate) fn new(runtime_environment: &'a RuntimeEnvironment) -> Self {
        Self {
            runtime_environment,
        }
    }

    /// Converts a runtime type into a type tag. If the type is too complex (e.g., struct name size
    /// too large, or type too deeply nested), an error is returned.
    pub(crate) fn ty_to_ty_tag(&self, ty: TypeId) -> PartialVMResult<TypeTag> {
        let mut gas_context = PseudoGasContext::new(self.runtime_environment.vm_config());
        self.ty_to_ty_tag_impl(ty, &mut gas_context)
    }

    fn ty_to_ty_tag_impl(
        &self,
        ty: TypeId,
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<TypeTag> {
        // Charge base cost at the start.
        gas_context.charge_base()?;

        Ok(match ty {
            // Primitive types.
            TypeId::BOOL => TypeTag::Bool,
            TypeId::U8 => TypeTag::U8,
            TypeId::U16 => TypeTag::U16,
            TypeId::U32 => TypeTag::U32,
            TypeId::U64 => TypeTag::U64,
            TypeId::U128 => TypeTag::U128,
            TypeId::U256 => TypeTag::U256,
            TypeId::I8 => TypeTag::I8,
            TypeId::I16 => TypeTag::I16,
            TypeId::I32 => TypeTag::I32,
            TypeId::I64 => TypeTag::I64,
            TypeId::I128 => TypeTag::I128,
            TypeId::I256 => TypeTag::I256,
            TypeId::ADDRESS => TypeTag::Address,
            TypeId::SIGNER => TypeTag::Signer,
            ty => {
                let ty_pool = self.runtime_environment.ty_pool();
                match ty_pool.type_repr(ty) {
                    TypeRepr::Vector(elem) => {
                        let elem_ty_tag = self.ty_to_ty_tag_impl(elem, gas_context)?;
                        TypeTag::Vector(Box::new(elem_ty_tag))
                    },

                    // Structs: we need to convert indices to names, possibly caching struct tags.
                    TypeRepr::Struct { idx, ty_args } => {
                        let ty_tag_cache = self.runtime_environment.ty_tag_cache();
                        // If cached, charge pseudo-gas cost and return.
                        if let Some(priced_tag) = ty_tag_cache.get_struct_tag(ty) {
                            gas_context.charge(priced_tag.pseudo_gas_cost)?;
                            return Ok(TypeTag::Struct(Box::new(priced_tag.struct_tag)));
                        }

                        let ty_args_vec = ty_pool.get_type_vec(ty_args);
                        let priced_tag = self.struct_name_idx_to_struct_tag_impl(
                            &idx,
                            &ty_args_vec,
                            gas_context,
                        )?;
                        ty_tag_cache.insert_struct_tag(ty, &priced_tag);
                        TypeTag::Struct(Box::new(priced_tag.struct_tag))
                    },

                    // Functions: recursively construct tags for argument and return types. Note that these
                    // can be references, unlike regular tags.
                    TypeRepr::Function {
                        args,
                        results,
                        abilities,
                    } => {
                        let args_vec = ty_pool.get_type_vec(args);
                        let results_vec = ty_pool.get_type_vec(results);

                        let to_vec = |ts: &[TypeId],
                                      gas_ctx: &mut PseudoGasContext|
                                      -> PartialVMResult<Vec<FunctionParamOrReturnTag>> {
                            ts.iter()
                                .map(|&t| {
                                    Ok(match ty_pool.type_repr(t) {
                                        TypeRepr::Reference(inner) => FunctionParamOrReturnTag::Reference(
                                            self.ty_to_ty_tag_impl(inner, gas_ctx)?,
                                        ),
                                        TypeRepr::MutableReference(inner) => {
                                            FunctionParamOrReturnTag::MutableReference(
                                                self.ty_to_ty_tag_impl(inner, gas_ctx)?,
                                            )
                                        },
                                        _ => FunctionParamOrReturnTag::Value(
                                            self.ty_to_ty_tag_impl(t, gas_ctx)?,
                                        ),
                                    })
                                })
                                .collect()
                        };
                        TypeTag::Function(Box::new(FunctionTag {
                            args: to_vec(&args_vec, gas_context)?,
                            results: to_vec(&results_vec, gas_context)?,
                            abilities,
                        }))
                    },

                    // References cannot be converted to tags.
                    TypeRepr::Reference(_) | TypeRepr::MutableReference(_) => {
                        return Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message(format!("No type tag for TypeId {:?}", ty)));
                    },
                    _ => unreachable!(),
                }
            },
        })
    }

    fn struct_name_idx_to_struct_tag_impl(
        &self,
        struct_name_idx: &StructNameIndex,
        ty_args: &[TypeId],
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<PricedStructTag> {
        // If not cached, record the current cost and construct tags for type arguments.
        let cur_cost = gas_context.current_cost();

        let type_args = ty_args
            .iter()
            .map(|&ty| self.ty_to_ty_tag_impl(ty, gas_context))
            .collect::<PartialVMResult<Vec<_>>>()?;

        // Construct the struct tag as well.
        let struct_name_index_map = self.runtime_environment.struct_name_index_map();
        let struct_tag = struct_name_index_map.idx_to_struct_tag(*struct_name_idx, type_args)?;
        gas_context.charge_struct_tag(&struct_tag)?;

        // Cache the struct tag. Record its gas cost as well.
        Ok(PricedStructTag {
            struct_tag,
            pseudo_gas_cost: gas_context.current_cost() - cur_cost,
        })
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::config::VMConfig;
//     use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some};
//     use hashbrown::Equivalent;
//     use move_binary_format::file_format::StructTypeParameter;
//     use move_core_types::{
//         ability::{Ability, AbilitySet},
//         account_address::AccountAddress,
//         identifier::Identifier,
//         language_storage::ModuleId,
//     };
//     use move_vm_types::loaded_data::runtime_types::{
//         AbilityInfo, StructIdentifier, StructLayout, StructType, TypeBuilder,
//     };
//     use smallbitvec::SmallBitVec;
//     use std::{collections::hash_map::DefaultHasher, str::FromStr};
//     use triomphe::Arc;
//
//     fn calculate_hash<T: Hash>(t: &T) -> u64 {
//         let mut s = DefaultHasher::new();
//         t.hash(&mut s);
//         s.finish()
//     }
//
//     #[test]
//     fn test_struct_key_equivalence_and_hash() {
//         let struct_keys = [
//             StructKey {
//                 idx: StructNameIndex::new(0),
//                 ty_args: vec![],
//             },
//             StructKey {
//                 idx: StructNameIndex::new(1),
//                 ty_args: vec![Type::U8],
//             },
//             StructKey {
//                 idx: StructNameIndex::new(2),
//                 ty_args: vec![Type::Bool, Type::Vector(Arc::new(Type::Bool))],
//             },
//             StructKey {
//                 idx: StructNameIndex::new(3),
//                 ty_args: vec![
//                     Type::Struct {
//                         idx: StructNameIndex::new(0),
//                         ability: AbilityInfo::struct_(AbilitySet::singleton(Ability::Key)),
//                     },
//                     Type::StructInstantiation {
//                         idx: StructNameIndex::new(1),
//                         ty_args: Arc::new(vec![Type::Address, Type::Struct {
//                             idx: StructNameIndex::new(2),
//                             ability: AbilityInfo::struct_(AbilitySet::singleton(Ability::Copy)),
//                         }]),
//                         ability: AbilityInfo::generic_struct(
//                             AbilitySet::singleton(Ability::Drop),
//                             SmallBitVec::new(),
//                         ),
//                     },
//                 ],
//             },
//         ];
//
//         for struct_key in struct_keys {
//             let struct_key_ref = struct_key.as_ref();
//             assert!(struct_key.equivalent(&struct_key_ref));
//             assert!(struct_key_ref.equivalent(&struct_key));
//             assert_eq!(calculate_hash(&struct_key), calculate_hash(&struct_key_ref));
//         }
//     }
//
//     #[test]
//     fn test_type_tag_cache() {
//         let cache = TypeTagCache::empty();
//         assert!(cache.cache.read().is_empty());
//         assert!(cache
//             .get_struct_tag(&StructNameIndex::new(0), &[])
//             .is_none());
//
//         let tag = PricedStructTag {
//             struct_tag: StructTag::from_str("0x1::foo::Foo").unwrap(),
//             pseudo_gas_cost: 10,
//         };
//         assert!(cache.insert_struct_tag(&StructNameIndex::new(0), &[], &tag));
//
//         let tag = PricedStructTag {
//             struct_tag: StructTag::from_str("0x1::foo::Foo").unwrap(),
//             // Set different cost to check.
//             pseudo_gas_cost: 100,
//         };
//         assert!(!cache.insert_struct_tag(&StructNameIndex::new(0), &[], &tag));
//
//         assert_eq!(cache.cache.read().len(), 1);
//         let cost = cache
//             .get_struct_tag(&StructNameIndex::new(0), &[])
//             .unwrap()
//             .pseudo_gas_cost;
//         assert_eq!(cost, 10);
//     }
//
//     #[test]
//     fn test_ty_to_ty_tag() {
//         let ty_builder = TypeBuilder::with_limits(10, 10);
//
//         let runtime_environment = RuntimeEnvironment::new(vec![]);
//         let ty_tag_converter = TypeTagConverter::new(&runtime_environment);
//
//         let disallowed_tys = [
//             Type::TyParam(0),
//             ty_builder
//                 .create_ref_ty(&ty_builder.create_u8_ty(), true)
//                 .unwrap(),
//             ty_builder
//                 .create_ref_ty(&ty_builder.create_u8_ty(), false)
//                 .unwrap(),
//         ];
//         for ty in disallowed_tys {
//             assert_err!(ty_tag_converter.ty_to_ty_tag(&ty));
//         }
//
//         let allowed_primitive_tys = [
//             (ty_builder.create_bool_ty(), TypeTag::Bool),
//             (ty_builder.create_u8_ty(), TypeTag::U8),
//             (ty_builder.create_u16_ty(), TypeTag::U16),
//             (ty_builder.create_u32_ty(), TypeTag::U32),
//             (ty_builder.create_u64_ty(), TypeTag::U64),
//             (ty_builder.create_u128_ty(), TypeTag::U128),
//             (ty_builder.create_u256_ty(), TypeTag::U256),
//             (ty_builder.create_address_ty(), TypeTag::Address),
//             (ty_builder.create_signer_ty(), TypeTag::Signer),
//         ];
//         for (ty, expected_tag) in allowed_primitive_tys {
//             let actual_tag = assert_ok!(ty_tag_converter.ty_to_ty_tag(&ty));
//             assert_eq!(actual_tag, expected_tag);
//         }
//
//         // Vectors.
//         let bool_vec_ty = ty_builder
//             .create_vec_ty(&ty_builder.create_bool_ty())
//             .unwrap();
//         let bool_vec_tag = TypeTag::Vector(Box::new(TypeTag::Bool));
//         assert_ok_eq!(
//             ty_tag_converter.ty_to_ty_tag(&bool_vec_ty),
//             bool_vec_tag.clone()
//         );
//
//         // Structs.
//         let module_id = ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap());
//         let bar_idx = runtime_environment
//             .struct_name_index_map()
//             .struct_name_to_idx(&StructIdentifier::new(
//                 runtime_environment.module_id_pool(),
//                 module_id,
//                 Identifier::new("Bar").unwrap(),
//             ))
//             .unwrap();
//         let module_id = ModuleId::new(AccountAddress::TWO, Identifier::new("foo").unwrap());
//         let foo_idx = runtime_environment
//             .struct_name_index_map()
//             .struct_name_to_idx(&StructIdentifier::new(
//                 runtime_environment.module_id_pool(),
//                 module_id,
//                 Identifier::new("Foo").unwrap(),
//             ))
//             .unwrap();
//
//         let struct_ty =
//             ty_builder.create_struct_ty(bar_idx, AbilityInfo::struct_(AbilitySet::EMPTY));
//         let struct_tag = StructTag::from_str("0x1::foo::Bar").unwrap();
//         assert_ok_eq!(
//             ty_tag_converter.ty_to_ty_tag(&struct_ty),
//             TypeTag::Struct(Box::new(struct_tag))
//         );
//
//         let struct_ty = StructType {
//             idx: foo_idx,
//             layout: StructLayout::Single(vec![(
//                 Identifier::new("field").unwrap(),
//                 Type::TyParam(0),
//             )]),
//             phantom_ty_params_mask: Default::default(),
//             abilities: AbilitySet::EMPTY,
//             ty_params: vec![StructTypeParameter {
//                 constraints: AbilitySet::EMPTY,
//                 is_phantom: false,
//             }],
//         };
//         let generic_struct_ty = ty_builder
//             .create_struct_instantiation_ty(&struct_ty, &[Type::TyParam(0)], &[bool_vec_ty])
//             .unwrap();
//         let struct_tag = StructTag::from_str("0x2::foo::Foo<vector<bool>>").unwrap();
//         assert_ok_eq!(
//             ty_tag_converter.ty_to_ty_tag(&generic_struct_ty),
//             TypeTag::Struct(Box::new(struct_tag))
//         );
//     }
//
//     #[test]
//     fn test_ty_to_ty_tag_too_complex() {
//         let ty_builder = TypeBuilder::with_limits(10, 10);
//
//         let vm_config = VMConfig {
//             type_base_cost: 1,
//             type_max_cost: 2,
//             ..Default::default()
//         };
//         let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
//         let ty_tag_converter = TypeTagConverter::new(&runtime_environment);
//
//         let bool_ty = ty_builder.create_bool_ty();
//         assert_ok_eq!(ty_tag_converter.ty_to_ty_tag(&bool_ty), TypeTag::Bool);
//
//         let vec_ty = ty_builder.create_vec_ty(&bool_ty).unwrap();
//         assert_ok_eq!(
//             ty_tag_converter.ty_to_ty_tag(&vec_ty),
//             TypeTag::Vector(Box::new(TypeTag::Bool))
//         );
//
//         let vec_ty = ty_builder.create_vec_ty(&vec_ty).unwrap();
//         let err = assert_err!(ty_tag_converter.ty_to_ty_tag(&vec_ty));
//         assert_eq!(err.major_status(), StatusCode::TYPE_TAG_LIMIT_EXCEEDED);
//     }
//
//     #[test]
//     fn test_ty_to_ty_tag_struct_metering() {
//         let type_max_cost = 76;
//         let vm_config = VMConfig {
//             type_base_cost: 1,
//             type_byte_cost: 2,
//             type_max_cost,
//             ..Default::default()
//         };
//         let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
//         let ty_tag_converter = TypeTagConverter::new(&runtime_environment);
//
//         let module_id = ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap());
//         let id = StructIdentifier::new(
//             runtime_environment.module_id_pool(),
//             module_id,
//             Identifier::new("Foo").unwrap(),
//         );
//         let idx = runtime_environment
//             .struct_name_index_map()
//             .struct_name_to_idx(&id)
//             .unwrap();
//         let struct_tag = StructTag::from_str("0x1::foo::Foo").unwrap();
//
//         let mut gas_context = PseudoGasContext::new(runtime_environment.vm_config());
//         assert_ok_eq!(
//             ty_tag_converter.struct_name_idx_to_struct_tag_impl(&idx, &[], &mut gas_context),
//             struct_tag.clone()
//         );
//
//         // Address size, plus module name and struct name each taking 3 characters.
//         let expected_cost = 2 * (32 + 3 + 3);
//         assert_eq!(gas_context.current_cost(), expected_cost);
//
//         let priced_tag = assert_some!(runtime_environment.ty_tag_cache().get_struct_tag(&idx, &[]));
//         assert_eq!(priced_tag.pseudo_gas_cost, expected_cost);
//         assert_eq!(priced_tag.struct_tag, struct_tag);
//
//         // Now
//         let vm_config = VMConfig {
//             type_base_cost: 1,
//             type_byte_cost: 2,
//             // Use smaller limit, to test metering.
//             type_max_cost: type_max_cost - 1,
//             ..Default::default()
//         };
//         let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
//         let mut gas_context = PseudoGasContext::new(runtime_environment.vm_config());
//
//         let err = assert_err!(ty_tag_converter.struct_name_idx_to_struct_tag_impl(
//             &idx,
//             &[],
//             &mut gas_context
//         ));
//         assert_eq!(err.major_status(), StatusCode::TYPE_TAG_LIMIT_EXCEEDED);
//         assert_none!(runtime_environment.ty_tag_cache().get_struct_tag(&idx, &[]));
//     }
// }
