// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{config::VMConfig, RuntimeEnvironment};
use hashbrown::{hash_map::Entry, HashMap};
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
    vm_status::StatusCode,
};
use move_vm_types::loaded_data::{runtime_types::Type, struct_name_indexing::StructNameIndex};
use parking_lot::RwLock;
use std::hash::{Hash, Hasher};

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

/// Key type for [TypeTagCache] that corresponds to a fully-instantiated struct.
#[derive(Clone, Eq, PartialEq)]
struct StructKey {
    idx: StructNameIndex,
    ty_args: Vec<Type>,
}

#[derive(Eq, PartialEq)]
struct StructKeyRef<'a> {
    idx: &'a StructNameIndex,
    ty_args: &'a [Type],
}

impl StructKey {
    #[cfg(test)]
    fn as_ref(&self) -> StructKeyRef<'_> {
        StructKeyRef {
            idx: &self.idx,
            ty_args: self.ty_args.as_slice(),
        }
    }
}

impl<'a> hashbrown::Equivalent<StructKeyRef<'a>> for StructKey {
    fn equivalent(&self, other: &StructKeyRef<'a>) -> bool {
        &self.idx == other.idx && self.ty_args.as_slice() == other.ty_args
    }
}

impl hashbrown::Equivalent<StructKey> for StructKeyRef<'_> {
    fn equivalent(&self, other: &StructKey) -> bool {
        self.idx == &other.idx && self.ty_args == other.ty_args.as_slice()
    }
}

// Ensure hash is the same as for StructKeyRef.
impl Hash for StructKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.ty_args.hash(state);
    }
}

// Ensure hash is the same as for StructKey.
impl Hash for StructKeyRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
        self.ty_args.hash(state);
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
    cache: RwLock<HashMap<StructKey, PricedStructTag>>,
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
    pub(crate) fn get_struct_tag(
        &self,
        idx: &StructNameIndex,
        ty_args: &[Type],
    ) -> Option<PricedStructTag> {
        self.cache
            .read()
            .get(&StructKeyRef { idx, ty_args })
            .cloned()
    }

    /// Inserts the struct tag and its pseudo-gas cost ([PricedStructTag]) into the cache. Returns
    /// true if the tag was not cached before, and false otherwise.
    pub(crate) fn insert_struct_tag(
        &self,
        idx: &StructNameIndex,
        ty_args: &[Type],
        priced_struct_tag: &PricedStructTag,
    ) -> bool {
        // Check if already cached.
        if self
            .cache
            .read()
            .contains_key(&StructKeyRef { idx, ty_args })
        {
            return false;
        }

        let key = StructKey {
            idx: *idx,
            ty_args: ty_args.to_vec(),
        };
        let priced_struct_tag = priced_struct_tag.clone();

        // Otherwise, we need to insert. We did the clones outside the lock, and also avoid the
        // double insertion.
        let mut cache = self.cache.write();
        if let Entry::Vacant(entry) = cache.entry(key) {
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
    pub(crate) fn ty_to_ty_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        let mut gas_context = PseudoGasContext::new(self.runtime_environment.vm_config());
        self.ty_to_ty_tag_impl(ty, &mut gas_context)
    }

    /// Converts the struct type (based on its indexed name and type arguments) into a struct tag.
    /// If the tag has not been previously cached, it will be cached. Just like for types, if the
    /// type arguments are too complex, etc. the tag construction fails.
    pub(crate) fn struct_name_idx_to_struct_tag(
        &self,
        struct_name_idx: &StructNameIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<StructTag> {
        let mut gas_context = PseudoGasContext::new(self.runtime_environment.vm_config());
        self.struct_name_idx_to_struct_tag_impl(struct_name_idx, ty_args, &mut gas_context)
    }

    fn ty_to_ty_tag_impl(
        &self,
        ty: &Type,
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<TypeTag> {
        // Charge base cost at the start.
        gas_context.charge_base()?;

        Ok(match ty {
            // Primitive types.
            Type::Bool => TypeTag::Bool,
            Type::U8 => TypeTag::U8,
            Type::U16 => TypeTag::U16,
            Type::U32 => TypeTag::U32,
            Type::U64 => TypeTag::U64,
            Type::U128 => TypeTag::U128,
            Type::U256 => TypeTag::U256,
            Type::Address => TypeTag::Address,
            Type::Signer => TypeTag::Signer,

            // Vector types: recurse.
            Type::Vector(elem_ty) => {
                let elem_ty_tag = self.ty_to_ty_tag_impl(elem_ty, gas_context)?;
                TypeTag::Vector(Box::new(elem_ty_tag))
            },

            // Structs: we need to convert indices to names, possibly caching struct tags.
            Type::Struct { idx, .. } => {
                let struct_tag = self.struct_name_idx_to_struct_tag_impl(idx, &[], gas_context)?;
                TypeTag::Struct(Box::new(struct_tag))
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                let struct_tag =
                    self.struct_name_idx_to_struct_tag_impl(idx, ty_args, gas_context)?;
                TypeTag::Struct(Box::new(struct_tag))
            },

            // Functions: recursively construct tags for argument and return types. Note that these
            // can be references, unlike regular tags.
            Type::Function {
                args,
                results,
                abilities,
            } => {
                let to_vec = |ts: &[Type],
                              gas_ctx: &mut PseudoGasContext|
                 -> PartialVMResult<Vec<FunctionParamOrReturnTag>> {
                    ts.iter()
                        .map(|t| {
                            Ok(match t {
                                Type::Reference(t) => FunctionParamOrReturnTag::Reference(
                                    self.ty_to_ty_tag_impl(t, gas_ctx)?,
                                ),
                                Type::MutableReference(t) => {
                                    FunctionParamOrReturnTag::MutableReference(
                                        self.ty_to_ty_tag_impl(t, gas_ctx)?,
                                    )
                                },
                                t => FunctionParamOrReturnTag::Value(
                                    self.ty_to_ty_tag_impl(t, gas_ctx)?,
                                ),
                            })
                        })
                        .collect()
                };
                TypeTag::Function(Box::new(FunctionTag {
                    args: to_vec(args, gas_context)?,
                    results: to_vec(results, gas_context)?,
                    abilities: *abilities,
                }))
            },

            // References and type parameters cannot be converted to tags.
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type tag for {:?}", ty)),
                );
            },
        })
    }

    fn struct_name_idx_to_struct_tag_impl(
        &self,
        struct_name_idx: &StructNameIndex,
        ty_args: &[Type],
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<StructTag> {
        let ty_tag_cache = self.runtime_environment.ty_tag_cache();

        // If cached, charge pseudo-gas cost and return.
        if let Some(priced_tag) = ty_tag_cache.get_struct_tag(struct_name_idx, ty_args) {
            gas_context.charge(priced_tag.pseudo_gas_cost)?;
            return Ok(priced_tag.struct_tag);
        }

        // If not cached, record the current cost and construct tags for type arguments.
        let cur_cost = gas_context.current_cost();

        let type_args = ty_args
            .iter()
            .map(|ty| self.ty_to_ty_tag_impl(ty, gas_context))
            .collect::<PartialVMResult<Vec<_>>>()?;

        // Construct the struct tag as well.
        let struct_name_index_map = self.runtime_environment.struct_name_index_map();
        let struct_tag = struct_name_index_map.idx_to_struct_tag(*struct_name_idx, type_args)?;
        gas_context.charge_struct_tag(&struct_tag)?;

        // Cache the struct tag. Record its gas cost as well.
        let priced_tag = PricedStructTag {
            struct_tag,
            pseudo_gas_cost: gas_context.current_cost() - cur_cost,
        };
        ty_tag_cache.insert_struct_tag(struct_name_idx, ty_args, &priced_tag);

        Ok(priced_tag.struct_tag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::VMConfig;
    use claims::{assert_err, assert_none, assert_ok, assert_ok_eq, assert_some};
    use hashbrown::Equivalent;
    use move_binary_format::file_format::StructTypeParameter;
    use move_core_types::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
        identifier::Identifier,
        language_storage::ModuleId,
    };
    use move_vm_types::loaded_data::runtime_types::{
        AbilityInfo, StructIdentifier, StructLayout, StructType, TypeBuilder,
    };
    use smallbitvec::SmallBitVec;
    use std::{collections::hash_map::DefaultHasher, str::FromStr};
    use triomphe::Arc;

    fn calculate_hash<T: Hash>(t: &T) -> u64 {
        let mut s = DefaultHasher::new();
        t.hash(&mut s);
        s.finish()
    }

    #[test]
    fn test_struct_key_equivalence_and_hash() {
        let struct_keys = [
            StructKey {
                idx: StructNameIndex::new(0),
                ty_args: vec![],
            },
            StructKey {
                idx: StructNameIndex::new(1),
                ty_args: vec![Type::U8],
            },
            StructKey {
                idx: StructNameIndex::new(2),
                ty_args: vec![Type::Bool, Type::Vector(Arc::new(Type::Bool))],
            },
            StructKey {
                idx: StructNameIndex::new(3),
                ty_args: vec![
                    Type::Struct {
                        idx: StructNameIndex::new(0),
                        ability: AbilityInfo::struct_(AbilitySet::singleton(Ability::Key)),
                    },
                    Type::StructInstantiation {
                        idx: StructNameIndex::new(1),
                        ty_args: Arc::new(vec![Type::Address, Type::Struct {
                            idx: StructNameIndex::new(2),
                            ability: AbilityInfo::struct_(AbilitySet::singleton(Ability::Copy)),
                        }]),
                        ability: AbilityInfo::generic_struct(
                            AbilitySet::singleton(Ability::Drop),
                            SmallBitVec::new(),
                        ),
                    },
                ],
            },
        ];

        for struct_key in struct_keys {
            let struct_key_ref = struct_key.as_ref();
            assert!(struct_key.equivalent(&struct_key_ref));
            assert!(struct_key_ref.equivalent(&struct_key));
            assert_eq!(calculate_hash(&struct_key), calculate_hash(&struct_key_ref));
        }
    }

    #[test]
    fn test_type_tag_cache() {
        let cache = TypeTagCache::empty();
        assert!(cache.cache.read().is_empty());
        assert!(cache
            .get_struct_tag(&StructNameIndex::new(0), &[])
            .is_none());

        let tag = PricedStructTag {
            struct_tag: StructTag::from_str("0x1::foo::Foo").unwrap(),
            pseudo_gas_cost: 10,
        };
        assert!(cache.insert_struct_tag(&StructNameIndex::new(0), &[], &tag));

        let tag = PricedStructTag {
            struct_tag: StructTag::from_str("0x1::foo::Foo").unwrap(),
            // Set different cost to check.
            pseudo_gas_cost: 100,
        };
        assert!(!cache.insert_struct_tag(&StructNameIndex::new(0), &[], &tag));

        assert_eq!(cache.cache.read().len(), 1);
        let cost = cache
            .get_struct_tag(&StructNameIndex::new(0), &[])
            .unwrap()
            .pseudo_gas_cost;
        assert_eq!(cost, 10);
    }

    #[test]
    fn test_ty_to_ty_tag() {
        let ty_builder = TypeBuilder::with_limits(10, 10);

        let runtime_environment = RuntimeEnvironment::new(vec![]);
        let ty_tag_converter = TypeTagConverter::new(&runtime_environment);

        let disallowed_tys = [
            Type::TyParam(0),
            ty_builder
                .create_ref_ty(&ty_builder.create_u8_ty(), true)
                .unwrap(),
            ty_builder
                .create_ref_ty(&ty_builder.create_u8_ty(), false)
                .unwrap(),
        ];
        for ty in disallowed_tys {
            assert_err!(ty_tag_converter.ty_to_ty_tag(&ty));
        }

        let allowed_primitive_tys = [
            (ty_builder.create_bool_ty(), TypeTag::Bool),
            (ty_builder.create_u8_ty(), TypeTag::U8),
            (ty_builder.create_u16_ty(), TypeTag::U16),
            (ty_builder.create_u32_ty(), TypeTag::U32),
            (ty_builder.create_u64_ty(), TypeTag::U64),
            (ty_builder.create_u128_ty(), TypeTag::U128),
            (ty_builder.create_u256_ty(), TypeTag::U256),
            (ty_builder.create_address_ty(), TypeTag::Address),
            (ty_builder.create_signer_ty(), TypeTag::Signer),
        ];
        for (ty, expected_tag) in allowed_primitive_tys {
            let actual_tag = assert_ok!(ty_tag_converter.ty_to_ty_tag(&ty));
            assert_eq!(actual_tag, expected_tag);
        }

        // Vectors.
        let bool_vec_ty = ty_builder
            .create_vec_ty(&ty_builder.create_bool_ty())
            .unwrap();
        let bool_vec_tag = TypeTag::Vector(Box::new(TypeTag::Bool));
        assert_ok_eq!(
            ty_tag_converter.ty_to_ty_tag(&bool_vec_ty),
            bool_vec_tag.clone()
        );

        // Structs.
        let bar_idx = runtime_environment
            .struct_name_index_map()
            .struct_name_to_idx(&StructIdentifier {
                module: ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap()),
                name: Identifier::new("Bar").unwrap(),
            })
            .unwrap();
        let foo_idx = runtime_environment
            .struct_name_index_map()
            .struct_name_to_idx(&StructIdentifier {
                module: ModuleId::new(AccountAddress::TWO, Identifier::new("foo").unwrap()),
                name: Identifier::new("Foo").unwrap(),
            })
            .unwrap();

        let struct_ty =
            ty_builder.create_struct_ty(bar_idx, AbilityInfo::struct_(AbilitySet::EMPTY));
        let struct_tag = StructTag::from_str("0x1::foo::Bar").unwrap();
        assert_ok_eq!(
            ty_tag_converter.ty_to_ty_tag(&struct_ty),
            TypeTag::Struct(Box::new(struct_tag))
        );

        let struct_ty = StructType {
            idx: foo_idx,
            layout: StructLayout::Single(vec![(
                Identifier::new("field").unwrap(),
                Type::TyParam(0),
            )]),
            phantom_ty_params_mask: Default::default(),
            abilities: AbilitySet::EMPTY,
            ty_params: vec![StructTypeParameter {
                constraints: AbilitySet::EMPTY,
                is_phantom: false,
            }],
        };
        let generic_struct_ty = ty_builder
            .create_struct_instantiation_ty(&struct_ty, &[Type::TyParam(0)], &[bool_vec_ty])
            .unwrap();
        let struct_tag = StructTag::from_str("0x2::foo::Foo<vector<bool>>").unwrap();
        assert_ok_eq!(
            ty_tag_converter.ty_to_ty_tag(&generic_struct_ty),
            TypeTag::Struct(Box::new(struct_tag))
        );
    }

    #[test]
    fn test_ty_to_ty_tag_too_complex() {
        let ty_builder = TypeBuilder::with_limits(10, 10);

        let vm_config = VMConfig {
            type_base_cost: 1,
            type_max_cost: 2,
            ..Default::default()
        };
        let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
        let ty_tag_converter = TypeTagConverter::new(&runtime_environment);

        let bool_ty = ty_builder.create_bool_ty();
        assert_ok_eq!(ty_tag_converter.ty_to_ty_tag(&bool_ty), TypeTag::Bool);

        let vec_ty = ty_builder.create_vec_ty(&bool_ty).unwrap();
        assert_ok_eq!(
            ty_tag_converter.ty_to_ty_tag(&vec_ty),
            TypeTag::Vector(Box::new(TypeTag::Bool))
        );

        let vec_ty = ty_builder.create_vec_ty(&vec_ty).unwrap();
        let err = assert_err!(ty_tag_converter.ty_to_ty_tag(&vec_ty));
        assert_eq!(err.major_status(), StatusCode::TYPE_TAG_LIMIT_EXCEEDED);
    }

    #[test]
    fn test_ty_to_ty_tag_struct_metering() {
        let type_max_cost = 76;
        let vm_config = VMConfig {
            type_base_cost: 1,
            type_byte_cost: 2,
            type_max_cost,
            ..Default::default()
        };
        let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
        let ty_tag_converter = TypeTagConverter::new(&runtime_environment);

        let id = StructIdentifier {
            module: ModuleId::new(AccountAddress::ONE, Identifier::new("foo").unwrap()),
            name: Identifier::new("Foo").unwrap(),
        };
        let idx = runtime_environment
            .struct_name_index_map()
            .struct_name_to_idx(&id)
            .unwrap();
        let struct_tag = StructTag::from_str("0x1::foo::Foo").unwrap();

        let mut gas_context = PseudoGasContext::new(runtime_environment.vm_config());
        assert_ok_eq!(
            ty_tag_converter.struct_name_idx_to_struct_tag_impl(&idx, &[], &mut gas_context),
            struct_tag.clone()
        );

        // Address size, plus module name and struct name each taking 3 characters.
        let expected_cost = 2 * (32 + 3 + 3);
        assert_eq!(gas_context.current_cost(), expected_cost);

        let priced_tag = assert_some!(runtime_environment.ty_tag_cache().get_struct_tag(&idx, &[]));
        assert_eq!(priced_tag.pseudo_gas_cost, expected_cost);
        assert_eq!(priced_tag.struct_tag, struct_tag);

        // Now
        let vm_config = VMConfig {
            type_base_cost: 1,
            type_byte_cost: 2,
            // Use smaller limit, to test metering.
            type_max_cost: type_max_cost - 1,
            ..Default::default()
        };
        let runtime_environment = RuntimeEnvironment::new_with_config(vec![], vm_config);
        let mut gas_context = PseudoGasContext::new(runtime_environment.vm_config());

        let err = assert_err!(ty_tag_converter.struct_name_idx_to_struct_tag_impl(
            &idx,
            &[],
            &mut gas_context
        ));
        assert_eq!(err.major_status(), StatusCode::TYPE_TAG_LIMIT_EXCEEDED);
        assert_none!(runtime_environment.ty_tag_cache().get_struct_tag(&idx, &[]));
    }
}
