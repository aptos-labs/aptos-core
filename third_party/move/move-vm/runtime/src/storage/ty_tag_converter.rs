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
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub(crate) fn get_struct_tag(&self, ty: TypeId) -> Option<PricedStructTag> {
        self.cache.read().get(&ty).cloned()
    }

    /// Inserts the struct tag and its pseudo-gas cost ([PricedStructTag]) into the cache. Returns
    /// true if the tag was not cached before, and false otherwise.
    #[allow(dead_code)]
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

    /// Converts the struct type (based on its indexed name and type arguments) into a struct tag.
    /// If the tag has not been previously cached, it will be cached. Just like for types, if the
    /// type arguments are too complex, etc. the tag construction fails.
    pub(crate) fn struct_name_idx_to_struct_tag(
        &self,
        struct_name_idx: &StructNameIndex,
        ty_args: &[TypeId],
    ) -> PartialVMResult<StructTag> {
        // TODO: caches?
        let mut gas_context = PseudoGasContext::new(self.runtime_environment.vm_config());
        let priced_tag =
            self.struct_name_idx_to_struct_tag_impl(struct_name_idx, ty_args, &mut gas_context)?;
        Ok(priced_tag.struct_tag)
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
