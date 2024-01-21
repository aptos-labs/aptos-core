// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig, data_cache::TransactionDataCache, logging::expect_no_verification_errors,
    native_functions::NativeFunctions, session::LoadedFunctionInstantiation,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{verification_error, Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        AbilitySet, CompiledModule, CompiledScript, Constant, ConstantPoolIndex, FieldHandleIndex,
        FieldInstantiationIndex, FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructFieldInformation, TableIndex,
        TypeParameterIndex,
    },
    IndexKind,
};
use move_bytecode_verifier::{self, cyclic_dependencies, dependencies};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{IdentifierMappingKind, LayoutTag, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::loaded_data::runtime_types::{
    AbilityInfo, DepthFormula, StructIdentifier, StructNameIndex, StructType, Type,
};
use parking_lot::{MappedRwLockReadGuard, RwLock, RwLockReadGuard};
use sha3::{Digest, Sha3_256};
use std::{
    collections::{btree_map, BTreeMap, BTreeSet, HashMap},
    hash::Hash,
    sync::Arc,
};

mod function;
mod modules;
mod script;
mod type_loader;

pub(crate) use function::{Function, FunctionHandle, FunctionInstantiation, LoadedFunction, Scope};
pub(crate) use modules::{Module, ModuleCache, ModuleStorage, ModuleStorageAdapter};
pub(crate) use script::{Script, ScriptCache};
use type_loader::intern_type;

type ScriptHash = [u8; 32];

// A simple cache that offers both a HashMap and a Vector lookup.
// Values are forced into a `Arc` so they can be used from multiple thread.
// Access to this cache is always under a `RwLock`.
#[derive(Clone)]
pub(crate) struct BinaryCache<K, V> {
    id_map: HashMap<K, usize>,
    binaries: Vec<Arc<V>>,
}

impl<K, V> BinaryCache<K, V>
where
    K: Eq + Hash,
{
    fn new() -> Self {
        Self {
            id_map: HashMap::new(),
            binaries: vec![],
        }
    }

    fn insert(&mut self, key: K, binary: V) -> &Arc<V> {
        self.binaries.push(Arc::new(binary));
        let idx = self.binaries.len() - 1;
        self.id_map.insert(key, idx);
        self.binaries
            .last()
            .expect("BinaryCache: last() after push() impossible failure")
    }

    fn get(&self, key: &K) -> Option<&Arc<V>> {
        let index = self.id_map.get(key)?;
        self.binaries.get(*index)
    }
}

pub(crate) struct StructNameCache {
    data: RwLock<(
        BTreeMap<StructIdentifier, StructNameIndex>,
        Vec<StructIdentifier>,
    )>,
}

impl Clone for StructNameCache {
    fn clone(&self) -> Self {
        let inner = self.data.read();
        Self {
            data: RwLock::new((inner.0.clone(), inner.1.clone())),
        }
    }
}

impl StructNameCache {
    pub(crate) fn new() -> Self {
        Self {
            data: RwLock::new((BTreeMap::new(), vec![])),
        }
    }

    pub(crate) fn insert_or_get(&self, name: StructIdentifier) -> StructNameIndex {
        if let Some(idx) = self.data.read().0.get(&name) {
            return *idx;
        }
        let mut inner_data = self.data.write();
        let idx = StructNameIndex(inner_data.1.len());
        inner_data.0.insert(name.clone(), idx);
        inner_data.1.push(name);
        idx
    }

    pub(crate) fn idx_to_identifier(
        &self,
        idx: StructNameIndex,
    ) -> MappedRwLockReadGuard<StructIdentifier> {
        RwLockReadGuard::map(self.data.read(), |inner| &inner.1[idx.0])
    }
}

//
// Loader
//

// A Loader is responsible to load scripts and modules and holds the cache of all loaded
// entities. Each cache is protected by a `RwLock`. Operation in the Loader must be thread safe
// (operating on values on the stack) and when cache needs updating the mutex must be taken.
// The `pub(crate)` API is what a Loader offers to the runtime.
pub(crate) struct Loader {
    scripts: RwLock<ScriptCache>,
    type_cache: RwLock<TypeCache>,
    natives: NativeFunctions,
    pub(crate) name_cache: StructNameCache,

    // The below field supports a hack to workaround well-known issues with the
    // loader cache. This cache is not designed to support module upgrade or deletion.
    // This leads to situations where the cache does not reflect the state of storage:
    //
    // 1. On module upgrade, the upgraded module is in storage, but the old one still in the cache.
    // 2. On an abandoned code publishing transaction, the cache may contain a module which was
    //    never committed to storage by the adapter.
    //
    // The solution is to add a flag to Loader marking it as 'invalidated'. For scenario (1),
    // the VM sets the flag itself. For scenario (2), a public API allows the adapter to set
    // the flag.
    //
    // If the cache is invalidated, it can (and must) still be used until there are no more
    // sessions alive which are derived from a VM with this loader. This is because there are
    // internal data structures derived from the loader which can become inconsistent. Therefore
    // the adapter must explicitly call a function to flush the invalidated loader.
    //
    // This code (the loader) needs a complete refactoring. The new loader should
    //   performance. This is essential for a cache like this in a multi-tenant execution
    //   environment.
    // - should delegate lifetime ownership to the adapter. Code loading (including verification)
    //   is a major execution bottleneck. We should be able to reuse a cache for the lifetime of
    //   the adapter/node, not just a VM or even session (as effectively today).
    invalidated: RwLock<bool>,

    // Collects the cache hits on module loads. This information can be read and reset by
    // an adapter to reason about read/write conflicts of code publishing transactions and
    // other transactions.
    module_cache_hits: RwLock<BTreeSet<ModuleId>>,

    vm_config: VMConfig,
}

impl Clone for Loader {
    fn clone(&self) -> Self {
        Self {
            scripts: RwLock::new(self.scripts.read().clone()),
            type_cache: RwLock::new(self.type_cache.read().clone()),
            natives: self.natives.clone(),
            name_cache: self.name_cache.clone(),
            invalidated: RwLock::new(*self.invalidated.read()),
            module_cache_hits: RwLock::new(self.module_cache_hits.read().clone()),
            vm_config: self.vm_config.clone(),
        }
    }
}

impl Loader {
    pub(crate) fn new(natives: NativeFunctions, vm_config: VMConfig) -> Self {
        Self {
            scripts: RwLock::new(ScriptCache::new()),
            type_cache: RwLock::new(TypeCache::new()),
            name_cache: StructNameCache::new(),
            natives,
            invalidated: RwLock::new(false),
            module_cache_hits: RwLock::new(BTreeSet::new()),
            vm_config,
        }
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    /// Flush this cache if it is marked as invalidated.
    pub(crate) fn flush_if_invalidated(&self) {
        let mut invalidated = self.invalidated.write();
        if *invalidated {
            *self.scripts.write() = ScriptCache::new();
            *self.type_cache.write() = TypeCache::new();
            *invalidated = false;
        }
    }

    /// Mark this cache as invalidated.
    pub(crate) fn mark_as_invalid(&self) {
        *self.invalidated.write() = true;
    }

    /// Check whether this cache is invalidated.
    pub(crate) fn is_invalidated(&self) -> bool {
        *self.invalidated.read()
    }

    //
    // Script verification and loading
    //

    // Scripts are verified and dependencies are loaded.
    // Effectively that means modules are cached from leaf to root in the dependency DAG.
    // If a dependency error is found, loading stops and the error is returned.
    // However all modules cached up to that point stay loaded.

    // Entry point for script execution (`MoveVM::execute_script`).
    // Verifies the script if it is not in the cache of scripts loaded.
    // Type parameters are checked as well after every type is loaded.
    pub(crate) fn load_script(
        &self,
        script_blob: &[u8],
        ty_args: &[TypeTag],
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<(Arc<Function>, LoadedFunctionInstantiation)> {
        // retrieve or load the script
        let mut sha3_256 = Sha3_256::new();
        sha3_256.update(script_blob);
        let hash_value: [u8; 32] = sha3_256.finalize().into();

        let mut scripts = self.scripts.write();
        let (main, parameters, return_) = match scripts.get(&hash_value) {
            Some(cached) => cached,
            None => {
                let ver_script =
                    self.deserialize_and_verify_script(script_blob, data_store, module_store)?;
                let script = Script::new(ver_script, &hash_value, module_store, &self.name_cache)?;
                scripts.insert(hash_value, script)
            },
        };

        // verify type arguments
        let mut type_arguments = vec![];
        for ty in ty_args {
            type_arguments.push(self.load_type(ty, data_store, module_store)?);
        }

        if self.vm_config.type_size_limit
            && type_arguments
                .iter()
                .map(|loaded_ty| self.count_type_nodes(loaded_ty))
                .sum::<u64>()
                > MAX_TYPE_INSTANTIATION_NODES
        {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).finish(Location::Script)
            );
        };

        self.verify_ty_args(main.type_parameters(), &type_arguments)
            .map_err(|e| e.finish(Location::Script))?;
        let instantiation = LoadedFunctionInstantiation {
            type_arguments,
            parameters,
            return_,
        };
        Ok((main, instantiation))
    }

    // The process of deserialization and verification is not and it must not be under lock.
    // So when publishing modules through the dependency DAG it may happen that a different
    // thread had loaded the module after this process fetched it from storage.
    // Caching will take care of that by asking for each dependency module again under lock.
    fn deserialize_and_verify_script(
        &self,
        script: &[u8],
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<CompiledScript> {
        let script = match CompiledScript::deserialize_with_config(
            script,
            &self.vm_config.deserializer_config,
        ) {
            Ok(script) => script,
            Err(err) => {
                let msg = format!("[VM] deserializer for script returned error: {:?}", err);
                return Err(PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                    .with_message(msg)
                    .finish(Location::Script));
            },
        };

        match self.verify_script(&script) {
            Ok(_) => {
                // verify dependencies
                let loaded_deps = script
                    .immediate_dependencies()
                    .into_iter()
                    .map(|module_id| self.load_module(&module_id, data_store, module_store))
                    .collect::<VMResult<_>>()?;
                self.verify_script_dependencies(&script, loaded_deps)?;
                Ok(script)
            },
            Err(err) => Err(err),
        }
    }

    // Script verification steps.
    // See `verify_module()` for module verification steps.
    fn verify_script(&self, script: &CompiledScript) -> VMResult<()> {
        fail::fail_point!("verifier-failpoint-3", |_| { Ok(()) });

        move_bytecode_verifier::verify_script_with_config(&self.vm_config.verifier, script)
    }

    fn verify_script_dependencies(
        &self,
        script: &CompiledScript,
        dependencies: Vec<Arc<Module>>,
    ) -> VMResult<()> {
        let mut deps = vec![];
        for dep in &dependencies {
            deps.push(dep.module());
        }
        dependencies::verify_script(script, deps)
    }

    //
    // Module verification and loading
    //

    // Loading verifies the module if it was never loaded.
    fn load_function_without_type_args(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<(Arc<Module>, Arc<Function>, Vec<Type>, Vec<Type>)> {
        let module = self.load_module(module_id, data_store, module_store)?;
        let func = module_store
            .resolve_function_by_name(function_name, module_id)
            .map_err(|err| err.finish(Location::Undefined))?;

        let parameters = func.parameter_types().to_vec();

        let return_ = func.return_types().to_vec();

        Ok((module, func, parameters, return_))
    }

    // Matches the actual returned type to the expected type, binding any type args to the
    // necessary type as stored in the map. The expected type must be a concrete type (no TyParam).
    // Returns true if a successful match is made.
    fn match_return_type<'a>(
        returned: &Type,
        expected: &'a Type,
        map: &mut BTreeMap<u16, &'a Type>,
    ) -> bool {
        match (returned, expected) {
            // The important case, deduce the type params
            (Type::TyParam(idx), _) => match map.entry(*idx) {
                btree_map::Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(expected);
                    true
                },
                btree_map::Entry::Occupied(occupied_entry) => *occupied_entry.get() == expected,
            },
            // Recursive types we need to recurse the matching types
            (Type::Reference(ret_inner), Type::Reference(expected_inner))
            | (Type::MutableReference(ret_inner), Type::MutableReference(expected_inner)) => {
                Self::match_return_type(ret_inner, expected_inner, map)
            },
            (Type::Vector(ret_inner), Type::Vector(expected_inner)) => {
                Self::match_return_type(ret_inner, expected_inner, map)
            },
            // Abilities should not contribute to the equality check as they just serve for caching computations.
            // For structs the both need to be the same struct.
            (
                Type::Struct { idx: ret_idx, .. },
                Type::Struct {
                    idx: expected_idx, ..
                },
            ) => *ret_idx == *expected_idx,
            // For struct instantiations we need to additionally match all type arguments
            (
                Type::StructInstantiation {
                    idx: ret_idx,
                    ty_args: ret_fields,
                    ..
                },
                Type::StructInstantiation {
                    idx: expected_idx,
                    ty_args: expected_fields,
                    ..
                },
            ) => {
                *ret_idx == *expected_idx
                    && ret_fields.len() == expected_fields.len()
                    && ret_fields
                        .iter()
                        .zip(expected_fields.iter())
                        .all(|types| Self::match_return_type(types.0, types.1, map))
            },
            // For primitive types we need to assure the types match
            (Type::U8, Type::U8)
            | (Type::U16, Type::U16)
            | (Type::U32, Type::U32)
            | (Type::U64, Type::U64)
            | (Type::U128, Type::U128)
            | (Type::U256, Type::U256)
            | (Type::Bool, Type::Bool)
            | (Type::Address, Type::Address)
            | (Type::Signer, Type::Signer) => true,
            // Otherwise the types do not match and we can't match return type to the expected type.
            // Note we don't use the _ pattern but spell out all cases, so that the compiler will
            // bark when a case is missed upon future updates to the types.
            (Type::U8, _)
            | (Type::U16, _)
            | (Type::U32, _)
            | (Type::U64, _)
            | (Type::U128, _)
            | (Type::U256, _)
            | (Type::Bool, _)
            | (Type::Address, _)
            | (Type::Signer, _)
            | (Type::Struct { .. }, _)
            | (Type::StructInstantiation { .. }, _)
            | (Type::Vector(_), _)
            | (Type::MutableReference(_), _)
            | (Type::Reference(_), _) => false,
        }
    }

    // Loading verifies the module if it was never loaded.
    // Type parameters are inferred from the expected return type. Returns an error if it's not
    // possible to infer the type parameters or return type cannot be matched.
    // The type parameters are verified with capabilities.
    pub(crate) fn load_function_with_type_arg_inference(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        expected_return_type: &Type,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<(LoadedFunction, LoadedFunctionInstantiation)> {
        let (module, func, parameters, return_vec) = self.load_function_without_type_args(
            module_id,
            function_name,
            data_store,
            module_store,
        )?;

        if return_vec.len() != 1 {
            // For functions that are marked constructor this should not happen.
            return Err(PartialVMError::new(StatusCode::ABORTED).finish(Location::Undefined));
        }
        let return_type = &return_vec[0];

        let mut map = BTreeMap::new();
        if !Self::match_return_type(return_type, expected_return_type, &mut map) {
            // For functions that are marked constructor this should not happen.
            return Err(
                PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                    .finish(Location::Undefined),
            );
        }

        // Construct the type arguments from the match
        let mut type_arguments = vec![];
        let type_param_len = func.type_parameters().len();
        for i in 0..type_param_len {
            if let Option::Some(t) = map.get(&(i as u16)) {
                type_arguments.push((*t).clone());
            } else {
                // Unknown type argument we are not able to infer the type arguments.
                // For functions that are marked constructor this should not happen.
                return Err(
                    PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                        .finish(Location::Undefined),
                );
            }
        }

        // verify type arguments for capability constraints
        self.verify_ty_args(func.type_parameters(), &type_arguments)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

        let loaded = LoadedFunctionInstantiation {
            type_arguments,
            parameters,
            return_: return_vec,
        };
        Ok((
            LoadedFunction {
                module,
                function: func,
            },
            loaded,
        ))
    }

    // Entry point for function execution (`MoveVM::execute_function`).
    // Loading verifies the module if it was never loaded.
    // Type parameters are checked as well after every type is loaded.
    pub(crate) fn load_function(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<(Arc<Module>, Arc<Function>, LoadedFunctionInstantiation)> {
        let (module, func, parameters, return_) = self.load_function_without_type_args(
            module_id,
            function_name,
            data_store,
            module_store,
        )?;

        let type_arguments = ty_args
            .iter()
            .map(|ty| self.load_type(ty, data_store, module_store))
            .collect::<VMResult<Vec<_>>>()
            .map_err(|mut err| {
                // User provided type arguement failed to load. Set extra sub status to distinguish from internal type loading error.
                if StatusCode::TYPE_RESOLUTION_FAILURE == err.major_status() {
                    err.set_sub_status(move_core_types::vm_status::sub_status::type_resolution_failure::EUSER_TYPE_LOADING_FAILURE);
                }
                err
            })?;

        // verify type arguments
        self.verify_ty_args(func.type_parameters(), &type_arguments)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

        let loaded = LoadedFunctionInstantiation {
            type_arguments,
            parameters,
            return_,
        };
        Ok((module, func, loaded))
    }

    // Entry point for module publishing (`MoveVM::publish_module_bundle`).
    //
    // All modules in the bundle to be published must be loadable. This function performs all
    // verification steps to load these modules without actually loading them into the code cache.
    pub(crate) fn verify_module_bundle_for_publication(
        &self,
        modules: &[CompiledModule],
        data_store: &mut TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<()> {
        fail::fail_point!("verifier-failpoint-1", |_| { Ok(()) });

        let mut bundle_unverified: BTreeSet<_> = modules.iter().map(|m| m.self_id()).collect();
        let mut bundle_verified = BTreeMap::new();
        for module in modules {
            let module_id = module.self_id();
            bundle_unverified.remove(&module_id);

            self.verify_module_for_publication(
                module,
                &bundle_verified,
                &bundle_unverified,
                data_store,
                module_store,
            )?;
            bundle_verified.insert(module_id.clone(), module.clone());
        }
        Ok(())
    }

    // A module to be published must be loadable.
    //
    // This step performs all verification steps to load the module without loading it.
    // The module is not added to the code cache. It is simply published to the data cache.
    // See `verify_script()` for script verification steps.
    //
    // If a module `M` is published together with a bundle of modules (i.e., a vector of modules),
    // - the `bundle_verified` argument tracks the modules that have already been verified in the
    //   bundle. Basically, this represents the modules appears before `M` in the bundle vector.
    // - the `bundle_unverified` argument tracks the modules that have not been verified when `M`
    //   is being verified, i.e., the modules appears after `M` in the bundle vector.
    fn verify_module_for_publication(
        &self,
        module: &CompiledModule,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        bundle_unverified: &BTreeSet<ModuleId>,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<()> {
        // Performs all verification steps to load the module without loading it, i.e., the new
        // module will NOT show up in `module_cache`. In the module republishing case, it means
        // that the old module is still in the `module_cache`, unless a new Loader is created,
        // which means that a new MoveVM instance needs to be created.
        move_bytecode_verifier::verify_module_with_config(&self.vm_config.verifier, module)?;
        self.check_natives(module)?;

        let mut visited = BTreeSet::new();
        let mut friends_discovered = BTreeSet::new();
        visited.insert(module.self_id());
        friends_discovered.extend(module.immediate_friends());

        // downward exploration of the module's dependency graph. Since we know nothing about this
        // target module, we don't know what the module may specify as its dependencies and hence,
        // we allow the loading of dependencies and the subsequent linking to fail.
        self.load_and_verify_dependencies(
            module,
            bundle_verified,
            data_store,
            module_store,
            &mut visited,
            &mut friends_discovered,
            /* allow_dependency_loading_failure */ true,
            /* dependencies_depth */ 0,
        )?;

        // upward exploration of the modules's dependency graph. Similar to dependency loading, as
        // we know nothing about this target module, we don't know what the module may specify as
        // its friends and hence, we allow the loading of friends to fail.
        self.load_and_verify_friends(
            friends_discovered,
            bundle_verified,
            bundle_unverified,
            data_store,
            module_store,
            /* allow_friend_loading_failure */ true,
            /* dependencies_depth */ 0,
        )?;

        // make sure there is no cyclic dependency
        self.verify_module_cyclic_relations(
            module,
            bundle_verified,
            bundle_unverified,
            module_store,
        )
    }

    fn verify_module_cyclic_relations(
        &self,
        module: &CompiledModule,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        bundle_unverified: &BTreeSet<ModuleId>,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<()> {
        // let module_cache = self.module_cache.read();
        cyclic_dependencies::verify_module(
            module,
            |module_id| {
                bundle_verified
                    .get(module_id)
                    .map(|module| module.immediate_dependencies())
                    .or_else(|| {
                        module_store
                            .module_at(module_id)
                            .map(|m| m.module.immediate_dependencies())
                    })
                    .ok_or_else(|| PartialVMError::new(StatusCode::MISSING_DEPENDENCY))
            },
            |module_id| {
                if bundle_unverified.contains(module_id) {
                    // If the module under verification declares a friend which is also in the
                    // bundle (and positioned after this module in the bundle), we defer the cyclic
                    // relation checking when we verify that module.
                    Ok(vec![])
                } else {
                    // Otherwise, we get all the information we need to verify whether this module
                    // creates a cyclic relation.
                    bundle_verified
                        .get(module_id)
                        .map(|module| module.immediate_friends())
                        .or_else(|| {
                            module_store
                                .module_at(module_id)
                                .map(|m| m.module.immediate_friends())
                        })
                        .ok_or_else(|| PartialVMError::new(StatusCode::MISSING_DEPENDENCY))
                }
            },
        )
    }

    fn check_natives(&self, module: &CompiledModule) -> VMResult<()> {
        fn check_natives_impl(_loader: &Loader, module: &CompiledModule) -> PartialVMResult<()> {
            // TODO: fix check and error code if we leave something around for native structs.
            // For now this generates the only error test cases care about...
            for (idx, struct_def) in module.struct_defs().iter().enumerate() {
                if struct_def.field_information == StructFieldInformation::Native {
                    return Err(verification_error(
                        StatusCode::MISSING_DEPENDENCY,
                        IndexKind::FunctionHandle,
                        idx as TableIndex,
                    ));
                }
            }
            Ok(())
        }
        check_natives_impl(self, module).map_err(|e| e.finish(Location::Module(module.self_id())))
    }

    //
    // Helpers for loading and verification
    //

    pub(crate) fn load_type(
        &self,
        type_tag: &TypeTag,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<Type> {
        Ok(match type_tag {
            TypeTag::Bool => Type::Bool,
            TypeTag::U8 => Type::U8,
            TypeTag::U16 => Type::U16,
            TypeTag::U32 => Type::U32,
            TypeTag::U64 => Type::U64,
            TypeTag::U128 => Type::U128,
            TypeTag::U256 => Type::U256,
            TypeTag::Address => Type::Address,
            TypeTag::Signer => Type::Signer,
            TypeTag::Vector(tt) => Type::Vector(triomphe::Arc::new(self.load_type(
                tt,
                data_store,
                module_store,
            )?)),
            TypeTag::Struct(struct_tag) => {
                let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());
                self.load_module(&module_id, data_store, module_store)?;
                let struct_type = module_store
                    // GOOD module was loaded above
                    .get_struct_type_by_identifier(&struct_tag.name, &module_id)
                    .map_err(|e| e.finish(Location::Undefined))?;
                if struct_type.type_parameters.is_empty() && struct_tag.type_params.is_empty() {
                    Type::Struct {
                        idx: struct_type.idx,
                        ability: AbilityInfo::struct_(struct_type.abilities),
                    }
                } else {
                    let mut type_params = vec![];
                    for ty_param in &struct_tag.type_params {
                        type_params.push(self.load_type(ty_param, data_store, module_store)?);
                    }
                    self.verify_ty_args(struct_type.type_param_constraints(), &type_params)
                        .map_err(|e| e.finish(Location::Undefined))?;
                    Type::StructInstantiation {
                        idx: struct_type.idx,
                        ty_args: triomphe::Arc::new(type_params),
                        ability: AbilityInfo::generic_struct(
                            struct_type.abilities,
                            struct_type.phantom_ty_args_mask.clone(),
                        ),
                    }
                }
            },
        })
    }

    // The interface for module loading. Aligned with `load_type` and `load_function`, this function
    // verifies that the module is OK instead of expect it.
    pub(crate) fn load_module(
        &self,
        id: &ModuleId,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<Arc<Module>> {
        self.load_module_internal(id, data_store, module_store)
    }

    // Load the transitive closure of the target module first, and then verify that the modules in
    // the closure do not have cyclic dependencies.
    fn load_module_internal(
        &self,
        id: &ModuleId,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
    ) -> VMResult<Arc<Module>> {
        // if the module is already in the code cache, load the cached version
        if let Some(cached) = module_store.module_at(id) {
            self.module_cache_hits.write().insert(id.clone());
            return Ok(cached);
        }

        // otherwise, load the transitive closure of the target module
        let module_ref = self.load_and_verify_module_and_dependencies_and_friends(
            id,
            &BTreeMap::new(),
            &BTreeSet::new(),
            data_store,
            module_store,
            /* allow_module_loading_failure */ true,
            /* dependencies_depth */ 0,
        )?;

        // verify that the transitive closure does not have cycles
        self.verify_module_cyclic_relations(
            module_ref.module(),
            &BTreeMap::new(),
            &BTreeSet::new(),
            module_store,
        )
        .map_err(expect_no_verification_errors)?;
        Ok(module_ref)
    }

    // Load, deserialize, and check the module with the bytecode verifier, without linking
    fn load_and_verify_module(
        &self,
        id: &ModuleId,
        data_store: &TransactionDataCache,
        allow_loading_failure: bool,
    ) -> VMResult<CompiledModule> {
        // bytes fetching, allow loading to fail if the flag is set
        let bytes = match data_store
            .load_module(id)
            .map_err(|e| e.finish(Location::Undefined))
        {
            Ok(bytes) => bytes,
            Err(err) if allow_loading_failure => return Err(err),
            Err(err) => {
                return Err(expect_no_verification_errors(err));
            },
        };

        // for bytes obtained from the data store, they should always deserialize and verify.
        // It is an invariant violation if they don't.
        let module =
            CompiledModule::deserialize_with_config(&bytes, &self.vm_config.deserializer_config)
                .map_err(|err| {
                    let msg = format!("Deserialization error: {:?}", err);
                    PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR)
                        .with_message(msg)
                        .finish(Location::Module(id.clone()))
                })
                .map_err(expect_no_verification_errors)?;

        fail::fail_point!("verifier-failpoint-2", |_| { Ok(module.clone()) });

        if self.vm_config.paranoid_type_checks && &module.self_id() != id {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Module self id mismatch with storage".to_string())
                    .finish(Location::Module(id.clone())),
            );
        }

        // bytecode verifier checks that can be performed with the module itself
        move_bytecode_verifier::verify_module_with_config(&self.vm_config.verifier, &module)
            .map_err(expect_no_verification_errors)?;
        self.check_natives(&module)
            .map_err(expect_no_verification_errors)?;
        Ok(module)
    }

    // Everything in `load_and_verify_module` and also recursively load and verify all the
    // dependencies of the target module.
    fn load_and_verify_module_and_dependencies(
        &self,
        id: &ModuleId,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
        visited: &mut BTreeSet<ModuleId>,
        friends_discovered: &mut BTreeSet<ModuleId>,
        allow_module_loading_failure: bool,
        dependencies_depth: usize,
    ) -> VMResult<Arc<Module>> {
        // dependency loading does not permit cycles
        if visited.contains(id) {
            return Err(PartialVMError::new(StatusCode::CYCLIC_MODULE_DEPENDENCY)
                .finish(Location::Undefined));
        }

        // module self-check
        let module = self.load_and_verify_module(id, data_store, allow_module_loading_failure)?;
        visited.insert(id.clone());
        friends_discovered.extend(module.immediate_friends());

        // downward exploration of the module's dependency graph. For a module that is loaded from
        // the data_store, we should never allow its dependencies to fail to load.
        self.load_and_verify_dependencies(
            &module,
            bundle_verified,
            data_store,
            module_store,
            visited,
            friends_discovered,
            /* allow_dependency_loading_failure */ false,
            dependencies_depth,
        )?;

        // if linking goes well, insert the module to the code cache
        let module_ref =
            module_store.insert(&self.natives, id.clone(), module, &self.name_cache)?;

        Ok(module_ref)
    }

    // downward exploration of the module's dependency graph
    fn load_and_verify_dependencies(
        &self,
        module: &CompiledModule,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
        visited: &mut BTreeSet<ModuleId>,
        friends_discovered: &mut BTreeSet<ModuleId>,
        allow_dependency_loading_failure: bool,
        dependencies_depth: usize,
    ) -> VMResult<()> {
        if let Some(max_dependency_depth) = self.vm_config.verifier.max_dependency_depth {
            if dependencies_depth > max_dependency_depth {
                return Err(
                    PartialVMError::new(StatusCode::MAX_DEPENDENCY_DEPTH_REACHED)
                        .finish(Location::Undefined),
                );
            }
        }
        // all immediate dependencies of the module being verified should be in one of the locations
        // - the verified portion of the bundle (e.g., verified before this module)
        // - the code cache (i.e., loaded already)
        // - the data store (i.e., not loaded to code cache yet)
        let mut bundle_deps = vec![];
        let mut cached_deps = vec![];
        for module_id in module.immediate_dependencies() {
            if let Some(cached) = bundle_verified.get(&module_id) {
                bundle_deps.push(cached);
            } else {
                let loaded = match module_store.module_at(&module_id) {
                    None => self.load_and_verify_module_and_dependencies(
                        &module_id,
                        bundle_verified,
                        data_store,
                        module_store,
                        visited,
                        friends_discovered,
                        allow_dependency_loading_failure,
                        dependencies_depth + 1,
                    )?,
                    Some(cached) => cached,
                };
                cached_deps.push(loaded);
            }
        }

        // once all dependencies are loaded, do the linking check
        let all_imm_deps = bundle_deps
            .into_iter()
            .chain(cached_deps.iter().map(|m| m.module()));

        fail::fail_point!("verifier-failpoint-4", |_| { Ok(()) });

        let result = dependencies::verify_module(module, all_imm_deps);

        // if dependencies loading is not allowed to fail, the linking should not fail as well
        if allow_dependency_loading_failure {
            result
        } else {
            result.map_err(expect_no_verification_errors)
        }
    }

    // Everything in `load_and_verify_module_and_dependencies` and also recursively load and verify
    // all the friends modules of the newly loaded modules, until the friends frontier covers the
    // whole closure.
    fn load_and_verify_module_and_dependencies_and_friends(
        &self,
        id: &ModuleId,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        bundle_unverified: &BTreeSet<ModuleId>,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
        allow_module_loading_failure: bool,
        dependencies_depth: usize,
    ) -> VMResult<Arc<Module>> {
        // load the closure of the module in terms of dependency relation
        let mut visited = BTreeSet::new();
        let mut friends_discovered = BTreeSet::new();
        let module_ref = self.load_and_verify_module_and_dependencies(
            id,
            bundle_verified,
            data_store,
            module_store,
            &mut visited,
            &mut friends_discovered,
            allow_module_loading_failure,
            0,
        )?;

        // upward exploration of the module's friendship graph and expand the friendship frontier.
        // For a module that is loaded from the data_store, we should never allow that its friends
        // fail to load.
        self.load_and_verify_friends(
            friends_discovered,
            bundle_verified,
            bundle_unverified,
            data_store,
            module_store,
            /* allow_friend_loading_failure */ false,
            dependencies_depth,
        )?;
        Ok(module_ref)
    }

    // upward exploration of the module's dependency graph
    fn load_and_verify_friends(
        &self,
        friends_discovered: BTreeSet<ModuleId>,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        bundle_unverified: &BTreeSet<ModuleId>,
        data_store: &TransactionDataCache,
        module_store: &ModuleStorageAdapter,
        allow_friend_loading_failure: bool,
        dependencies_depth: usize,
    ) -> VMResult<()> {
        if let Some(max_dependency_depth) = self.vm_config.verifier.max_dependency_depth {
            if dependencies_depth > max_dependency_depth {
                return Err(
                    PartialVMError::new(StatusCode::MAX_DEPENDENCY_DEPTH_REACHED)
                        .finish(Location::Undefined),
                );
            }
        }
        // for each new module discovered in the frontier, load them fully and expand the frontier.
        // apply three filters to the new friend modules discovered
        // - `!locked_cache.has_module(mid)`
        //   If we friend a module that is already in the code cache, then we know that the
        //   transitive closure of that module is loaded into the cache already, skip the loading
        // - `!bundle_verified.contains_key(mid)`
        //   In the case of publishing a bundle, we don't actually put the published module into
        //   code cache. This `bundle_verified` cache is a temporary extension of the code cache
        //   in the bundle publication scenario. If a module is already verified, we don't need to
        //   re-load it again.
        // - `!bundle_unverified.contains(mid)
        //   If the module under verification declares a friend which is also in the bundle (and
        //   positioned after this module in the bundle), we defer the loading of that module when
        //   it is the module's turn in the bundle.

        // FIXME: Is there concurrency issue?
        let new_imm_friends: Vec<_> = friends_discovered
            .into_iter()
            .filter(|mid| {
                !module_store.has_module(mid)
                    && !bundle_verified.contains_key(mid)
                    && !bundle_unverified.contains(mid)
            })
            .collect();

        for module_id in new_imm_friends {
            self.load_and_verify_module_and_dependencies_and_friends(
                &module_id,
                bundle_verified,
                bundle_unverified,
                data_store,
                module_store,
                allow_friend_loading_failure,
                dependencies_depth + 1,
            )?;
        }
        Ok(())
    }

    // Return an instantiated type given a generic and an instantiation.
    // Stopgap to avoid a recursion that is either taking too long or using too
    // much memory
    fn subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<Type> {
        // Before instantiating the type, count the # of nodes of all type arguments plus
        // existing type instantiation.
        // If that number is larger than MAX_TYPE_INSTANTIATION_NODES, refuse to construct this type.
        // This prevents constructing larger and lager types via struct instantiation.
        match ty {
            Type::MutableReference(_) | Type::Reference(_) | Type::Vector(_) => {
                if self.vm_config.type_size_limit
                    && self.count_type_nodes(ty) > MAX_TYPE_INSTANTIATION_NODES
                {
                    return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
                }
            },
            Type::StructInstantiation {
                ty_args: struct_inst,
                ..
            } => {
                let mut sum_nodes = 1u64;
                for ty in ty_args.iter().chain(struct_inst.iter()) {
                    sum_nodes = sum_nodes.saturating_add(self.count_type_nodes(ty));
                    if sum_nodes > MAX_TYPE_INSTANTIATION_NODES {
                        return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
                    }
                }
            },
            Type::Address
            | Type::Bool
            | Type::Signer
            | Type::Struct { .. }
            | Type::TyParam(_)
            | Type::U8
            | Type::U16
            | Type::U32
            | Type::U64
            | Type::U128
            | Type::U256 => (),
        };
        ty.subst(ty_args)
    }

    // Verify the kind (constraints) of an instantiation.
    // Both function and script invocation use this function to verify correctness
    // of type arguments provided
    fn verify_ty_args<'a, I>(&self, constraints: I, ty_args: &[Type]) -> PartialVMResult<()>
    where
        I: IntoIterator<Item = &'a AbilitySet>,
        I::IntoIter: ExactSizeIterator,
    {
        let constraints = constraints.into_iter();
        if constraints.len() != ty_args.len() {
            return Err(PartialVMError::new(
                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
            ));
        }
        for (ty, expected_k) in ty_args.iter().zip(constraints) {
            if !expected_k.is_subset(ty.abilities()?) {
                return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED));
            }
        }
        Ok(())
    }

    //
    // Internal helpers
    //

    fn get_script(&self, hash: &ScriptHash) -> Arc<Script> {
        Arc::clone(
            self.scripts
                .read()
                .scripts
                .get(hash)
                .expect("Script hash on Function must exist"),
        )
    }
}

//
// Resolver
//

// A simple wrapper for a `Module` or a `Script` in the `Resolver`
enum BinaryType {
    Module(Arc<Module>),
    Script(Arc<Script>),
}

// A Resolver is a simple and small structure allocated on the stack and used by the
// interpreter. It's the only API known to the interpreter and it's tailored to the interpreter
// needs.
pub(crate) struct Resolver<'a> {
    loader: &'a Loader,
    module_store: &'a ModuleStorageAdapter,
    binary: BinaryType,
}

impl<'a> Resolver<'a> {
    fn for_module(
        loader: &'a Loader,
        module_store: &'a ModuleStorageAdapter,
        module: Arc<Module>,
    ) -> Self {
        let binary = BinaryType::Module(module);
        Self {
            loader,
            binary,
            module_store,
        }
    }

    fn for_script(
        loader: &'a Loader,
        module_store: &'a ModuleStorageAdapter,
        script: Arc<Script>,
    ) -> Self {
        let binary = BinaryType::Script(script);
        Self {
            loader,
            binary,
            module_store,
        }
    }

    //
    // Constant resolution
    //

    pub(crate) fn constant_at(&self, idx: ConstantPoolIndex) -> &Constant {
        match &self.binary {
            BinaryType::Module(module) => module.module.constant_at(idx),
            BinaryType::Script(script) => script.script.constant_at(idx),
        }
    }

    //
    // Function resolution
    //

    pub(crate) fn function_from_handle(
        &self,
        idx: FunctionHandleIndex,
    ) -> PartialVMResult<Arc<Function>> {
        let idx = match &self.binary {
            BinaryType::Module(module) => module.function_at(idx.0),
            BinaryType::Script(script) => script.function_at(idx.0),
        };
        self.module_store.function_at(idx)
    }

    pub(crate) fn function_from_instantiation(
        &self,
        idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<Arc<Function>> {
        let func_inst = match &self.binary {
            BinaryType::Module(module) => module.function_instantiation_at(idx.0),
            BinaryType::Script(script) => script.function_instantiation_at(idx.0),
        };
        self.module_store.function_at(&func_inst.handle)
    }

    pub(crate) fn instantiate_generic_function(
        &self,
        idx: FunctionInstantiationIndex,
        type_params: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let func_inst = match &self.binary {
            BinaryType::Module(module) => module.function_instantiation_at(idx.0),
            BinaryType::Script(script) => script.function_instantiation_at(idx.0),
        };
        let mut instantiation = vec![];
        for ty in &func_inst.instantiation {
            instantiation.push(self.subst(ty, type_params)?);
        }
        // Check if the function instantiation over all generics is larger
        // than MAX_TYPE_INSTANTIATION_NODES.
        let mut sum_nodes = 1u64;
        for ty in type_params.iter().chain(instantiation.iter()) {
            sum_nodes = sum_nodes.saturating_add(self.loader.count_type_nodes(ty));
            if sum_nodes > MAX_TYPE_INSTANTIATION_NODES {
                return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
            }
        }
        Ok(instantiation)
    }

    #[allow(unused)]
    pub(crate) fn type_params_count(&self, idx: FunctionInstantiationIndex) -> usize {
        let func_inst = match &self.binary {
            BinaryType::Module(module) => module.function_instantiation_at(idx.0),
            BinaryType::Script(script) => script.function_instantiation_at(idx.0),
        };
        func_inst.instantiation.len()
    }

    //
    // Type resolution
    //

    pub(crate) fn get_struct_type(&self, idx: StructDefinitionIndex) -> PartialVMResult<Type> {
        let struct_def = match &self.binary {
            BinaryType::Module(module) => module.struct_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        Ok(Type::Struct {
            idx: struct_def.idx,
            ability: AbilityInfo::struct_(struct_def.abilities),
        })
    }

    pub(crate) fn get_struct_type_generic(
        &self,
        idx: StructDefInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        let struct_inst = match &self.binary {
            BinaryType::Module(module) => module.struct_instantiation_at(idx.0),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };

        // Before instantiating the type, count the # of nodes of all type arguments plus
        // existing type instantiation.
        // If that number is larger than MAX_TYPE_INSTANTIATION_NODES, refuse to construct this type.
        // This prevents constructing larger and larger types via struct instantiation.
        let mut sum_nodes = 1u64;
        for ty in ty_args.iter().chain(struct_inst.instantiation.iter()) {
            sum_nodes = sum_nodes.saturating_add(self.loader.count_type_nodes(ty));
            if sum_nodes > MAX_TYPE_INSTANTIATION_NODES {
                return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
            }
        }

        let struct_ = &struct_inst.definition_struct_type;
        Ok(Type::StructInstantiation {
            idx: struct_.idx,
            ty_args: triomphe::Arc::new(
                struct_inst
                    .instantiation
                    .iter()
                    .map(|ty| self.subst(ty, ty_args))
                    .collect::<PartialVMResult<_>>()?,
            ),
            ability: AbilityInfo::generic_struct(
                struct_.abilities,
                struct_.phantom_ty_args_mask.clone(),
            ),
        })
    }

    pub(crate) fn get_field_type(&self, idx: FieldHandleIndex) -> PartialVMResult<Type> {
        match &self.binary {
            BinaryType::Module(module) => {
                let handle = &module.field_handles[idx.0 as usize];

                Ok(handle.definition_struct_type.fields[handle.offset].clone())
            },
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_field_type_generic(
        &self,
        idx: FieldInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        let field_instantiation = match &self.binary {
            BinaryType::Module(module) => &module.field_instantiations[idx.0 as usize],
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };

        let instantiation_types = field_instantiation
            .instantiation
            .iter()
            .map(|inst_ty| inst_ty.subst(ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;
        field_instantiation.definition_struct_type.fields[field_instantiation.offset]
            .subst(&instantiation_types)
    }

    pub(crate) fn get_struct_fields(
        &self,
        idx: StructDefinitionIndex,
    ) -> PartialVMResult<Arc<StructType>> {
        match &self.binary {
            BinaryType::Module(module) => Ok(module.struct_at(idx)),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn instantiate_generic_struct_fields(
        &self,
        idx: StructDefInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let struct_inst = match &self.binary {
            BinaryType::Module(module) => module.struct_instantiation_at(idx.0),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_type = &struct_inst.definition_struct_type;

        let instantiation_types = struct_inst
            .instantiation
            .iter()
            .map(|inst_ty| inst_ty.subst(ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;
        struct_type
            .fields
            .iter()
            .map(|ty| ty.subst(&instantiation_types))
            .collect::<PartialVMResult<Vec<_>>>()
    }

    fn single_type_at(&self, idx: SignatureIndex) -> &Type {
        match &self.binary {
            BinaryType::Module(module) => module.single_type_at(idx),
            BinaryType::Script(script) => script.single_type_at(idx),
        }
    }

    pub(crate) fn instantiate_single_type(
        &self,
        idx: SignatureIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        let ty = self.single_type_at(idx);
        if !ty_args.is_empty() {
            self.subst(ty, ty_args)
        } else {
            Ok(ty.clone())
        }
    }

    pub(crate) fn subst(&self, ty: &Type, ty_args: &[Type]) -> PartialVMResult<Type> {
        self.loader.subst(ty, ty_args)
    }

    //
    // Fields resolution
    //

    pub(crate) fn field_offset(&self, idx: FieldHandleIndex) -> usize {
        match &self.binary {
            BinaryType::Module(module) => module.field_offset(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn field_instantiation_offset(&self, idx: FieldInstantiationIndex) -> usize {
        match &self.binary {
            BinaryType::Module(module) => module.field_instantiation_offset(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn field_count(&self, idx: StructDefinitionIndex) -> u16 {
        match &self.binary {
            BinaryType::Module(module) => module.field_count(idx.0),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn field_instantiation_count(&self, idx: StructDefInstantiationIndex) -> u16 {
        match &self.binary {
            BinaryType::Module(module) => module.field_instantiation_count(idx.0),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn field_handle_to_struct(&self, idx: FieldHandleIndex) -> PartialVMResult<Type> {
        match &self.binary {
            BinaryType::Module(module) => {
                let struct_ = &module.field_handles[idx.0 as usize].definition_struct_type;
                Ok(Type::Struct {
                    idx: struct_.idx,
                    ability: AbilityInfo::struct_(struct_.abilities),
                })
            },
            BinaryType::Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn field_instantiation_to_struct(
        &self,
        idx: FieldInstantiationIndex,
        args: &[Type],
    ) -> PartialVMResult<Type> {
        match &self.binary {
            BinaryType::Module(module) => {
                let struct_ = &module.field_instantiations[idx.0 as usize].definition_struct_type;
                Ok(Type::StructInstantiation {
                    idx: struct_.idx,
                    ty_args: triomphe::Arc::new(
                        module.field_instantiations[idx.0 as usize]
                            .instantiation
                            .iter()
                            .map(|ty| ty.subst(args))
                            .collect::<PartialVMResult<Vec<_>>>()?,
                    ),
                    ability: AbilityInfo::generic_struct(
                        struct_.abilities,
                        struct_.phantom_ty_args_mask.clone(),
                    ),
                })
            },
            BinaryType::Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        self.loader.type_to_type_layout(ty, self.module_store)
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.loader
            .type_to_type_layout_with_identifier_mappings(ty, self.module_store)
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
    ) -> PartialVMResult<MoveTypeLayout> {
        self.loader
            .type_to_fully_annotated_layout(ty, self.module_store)
    }

    // get the loader
    pub(crate) fn loader(&self) -> &Loader {
        self.loader
    }

    // get the loader
    pub(crate) fn module_store(&self) -> &ModuleStorageAdapter {
        self.module_store
    }
}

#[derive(Clone)]
struct StructLayoutInfoCacheItem {
    struct_layout: MoveStructLayout,
    node_count: u64,
    has_identifier_mappings: bool,
}

//
// Cache for data associated to a Struct, used for de/serialization and more
//
#[derive(Clone)]
struct StructInfoCache {
    struct_tag: Option<(StructTag, u64)>,
    struct_layout_info: Option<StructLayoutInfoCacheItem>,
    annotated_struct_layout: Option<MoveStructLayout>,
    annotated_node_count: Option<u64>,
}

impl StructInfoCache {
    fn new() -> Self {
        Self {
            struct_tag: None,
            struct_layout_info: None,
            annotated_struct_layout: None,
            annotated_node_count: None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct TypeCache {
    structs: HashMap<StructIdentifier, HashMap<Vec<Type>, StructInfoCache>>,
    depth_formula: HashMap<StructIdentifier, DepthFormula>,
}

impl TypeCache {
    fn new() -> Self {
        Self {
            structs: HashMap::new(),
            depth_formula: HashMap::new(),
        }
    }
}

/// Maximal depth of a value in terms of type depth.
pub const VALUE_DEPTH_MAX: u64 = 128;

/// Maximal nodes which are allowed when converting to layout. This includes the the types of
/// fields for struct types.
const MAX_TYPE_TO_LAYOUT_NODES: u64 = 256;

/// Maximal nodes which are all allowed when instantiating a generic type. This does not include
/// field types of structs.
const MAX_TYPE_INSTANTIATION_NODES: u64 = 128;

struct PseudoGasContext {
    max_cost: u64,
    cost: u64,
    cost_base: u64,
    cost_per_byte: u64,
}

impl PseudoGasContext {
    fn charge(&mut self, amount: u64) -> PartialVMResult<()> {
        self.cost += amount;
        if self.cost > self.max_cost {
            Err(PartialVMError::new(StatusCode::TYPE_TAG_LIMIT_EXCEEDED)
                .with_message(format!("Max type limit {} exceeded", self.max_cost)))
        } else {
            Ok(())
        }
    }
}

impl Loader {
    fn struct_name_to_type_tag(
        &self,
        struct_idx: StructNameIndex,
        ty_args: &[Type],
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<StructTag> {
        let name = &*self.name_cache.idx_to_identifier(struct_idx);
        if let Some(struct_map) = self.type_cache.read().structs.get(name) {
            if let Some(struct_info) = struct_map.get(ty_args) {
                if let Some((struct_tag, gas)) = &struct_info.struct_tag {
                    gas_context.charge(*gas)?;
                    return Ok(struct_tag.clone());
                }
            }
        }

        let cur_cost = gas_context.cost;

        let ty_arg_tags = ty_args
            .iter()
            .map(|ty| self.type_to_type_tag_impl(ty, gas_context))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let struct_tag = StructTag {
            address: *name.module.address(),
            module: name.module.name().to_owned(),
            name: name.name.clone(),
            type_params: ty_arg_tags,
        };

        let size =
            (struct_tag.address.len() + struct_tag.module.len() + struct_tag.name.len()) as u64;
        gas_context.charge(size * gas_context.cost_per_byte)?;
        self.type_cache
            .write()
            .structs
            .entry(name.clone())
            .or_default()
            .entry(ty_args.to_vec())
            .or_insert_with(StructInfoCache::new)
            .struct_tag = Some((struct_tag.clone(), gas_context.cost - cur_cost));

        Ok(struct_tag)
    }

    fn type_to_type_tag_impl(
        &self,
        ty: &Type,
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<TypeTag> {
        gas_context.charge(gas_context.cost_base)?;
        Ok(match ty {
            Type::Bool => TypeTag::Bool,
            Type::U8 => TypeTag::U8,
            Type::U16 => TypeTag::U16,
            Type::U32 => TypeTag::U32,
            Type::U64 => TypeTag::U64,
            Type::U128 => TypeTag::U128,
            Type::U256 => TypeTag::U256,
            Type::Address => TypeTag::Address,
            Type::Signer => TypeTag::Signer,
            Type::Vector(ty) => TypeTag::Vector(Box::new(self.type_to_type_tag(ty)?)),
            Type::Struct { idx, .. } => TypeTag::Struct(Box::new(self.struct_name_to_type_tag(
                *idx,
                &[],
                gas_context,
            )?)),
            Type::StructInstantiation { idx, ty_args, .. } => TypeTag::Struct(Box::new(
                self.struct_name_to_type_tag(*idx, ty_args, gas_context)?,
            )),
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("no type tag for {:?}", ty)),
                );
            },
        })
    }

    fn count_type_nodes(&self, ty: &Type) -> u64 {
        let mut todo = vec![ty];
        let mut result = 0;
        while let Some(ty) = todo.pop() {
            match ty {
                Type::Vector(ty) => {
                    result += 1;
                    todo.push(ty);
                },
                Type::Reference(ty) | Type::MutableReference(ty) => {
                    result += 1;
                    todo.push(ty);
                },
                Type::StructInstantiation { ty_args, .. } => {
                    result += 1;
                    todo.extend(ty_args.iter())
                },
                _ => {
                    result += 1;
                },
            }
        }
        result
    }

    fn struct_name_to_type_layout(
        &self,
        module_store: &ModuleStorageAdapter,
        struct_idx: StructNameIndex,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveStructLayout, bool)> {
        let name = &*self.name_cache.idx_to_identifier(struct_idx);
        if let Some(struct_map) = self.type_cache.read().structs.get(name) {
            if let Some(struct_info) = struct_map.get(ty_args) {
                if let Some(struct_layout_info) = &struct_info.struct_layout_info {
                    *count += struct_layout_info.node_count;
                    return Ok((
                        struct_layout_info.struct_layout.clone(),
                        struct_layout_info.has_identifier_mappings,
                    ));
                }
            }
        }

        let count_before = *count;
        let struct_type = module_store.get_struct_type_by_identifier(&name.name, &name.module)?;

        // Some types can have fields which are lifted at serialization or deserialization
        // times. Right now these are Aggregator and AggregatorSnapshot.
        let maybe_mapping = self.get_identifier_mapping_kind(name);

        let field_tys = struct_type
            .fields
            .iter()
            .map(|ty| self.subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let (mut field_layouts, field_has_identifier_mappings): (Vec<MoveTypeLayout>, Vec<bool>) =
            field_tys
                .iter()
                .map(|ty| self.type_to_type_layout_impl(ty, module_store, count, depth + 1))
                .collect::<PartialVMResult<Vec<_>>>()?
                .into_iter()
                .unzip();

        // For aggregators / snapshots, the first field should be lifted.
        if let Some(kind) = &maybe_mapping {
            if let Some(l) = field_layouts.first_mut() {
                *l = MoveTypeLayout::Tagged(
                    LayoutTag::IdentifierMapping(kind.clone()),
                    Box::new(l.clone()),
                );
            }
        }

        let has_identifier_mappings =
            maybe_mapping.is_some() || field_has_identifier_mappings.into_iter().any(|b| b);

        let field_node_count = *count - count_before;
        let struct_layout = MoveStructLayout::new(field_layouts);

        let mut cache = self.type_cache.write();
        let info = cache
            .structs
            .entry(name.clone())
            .or_default()
            .entry(ty_args.to_vec())
            .or_insert_with(StructInfoCache::new);
        info.struct_layout_info = Some(StructLayoutInfoCacheItem {
            struct_layout: struct_layout.clone(),
            node_count: field_node_count,
            has_identifier_mappings,
        });

        Ok((struct_layout, has_identifier_mappings))
    }

    // TODO[agg_v2](cleanup):
    // Currently aggregator checks are hardcoded and leaking to loader.
    // It seems that this is only because there is no support for native
    // types.
    // Let's think how we can do this nicer.
    fn get_identifier_mapping_kind(
        &self,
        struct_name: &StructIdentifier,
    ) -> Option<IdentifierMappingKind> {
        if !self.vm_config.aggregator_v2_type_tagging {
            return None;
        }

        let ident_str_to_kind = |ident_str: &IdentStr| -> Option<IdentifierMappingKind> {
            if ident_str.eq(ident_str!("Aggregator")) {
                Some(IdentifierMappingKind::Aggregator)
            } else if ident_str.eq(ident_str!("AggregatorSnapshot")) {
                Some(IdentifierMappingKind::Snapshot)
            } else {
                None
            }
        };

        (struct_name.module.address().eq(&AccountAddress::ONE)
            && struct_name.module.name().eq(ident_str!("aggregator_v2")))
        .then_some(ident_str_to_kind(struct_name.name.as_ident_str()))
        .flatten()
    }

    fn type_to_type_layout_impl(
        &self,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        if *count > MAX_TYPE_TO_LAYOUT_NODES {
            return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
        }
        if depth > VALUE_DEPTH_MAX {
            return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
        }
        Ok(match ty {
            Type::Bool => {
                *count += 1;
                (MoveTypeLayout::Bool, false)
            },
            Type::U8 => {
                *count += 1;
                (MoveTypeLayout::U8, false)
            },
            Type::U16 => {
                *count += 1;
                (MoveTypeLayout::U16, false)
            },
            Type::U32 => {
                *count += 1;
                (MoveTypeLayout::U32, false)
            },
            Type::U64 => {
                *count += 1;
                (MoveTypeLayout::U64, false)
            },
            Type::U128 => {
                *count += 1;
                (MoveTypeLayout::U128, false)
            },
            Type::U256 => {
                *count += 1;
                (MoveTypeLayout::U256, false)
            },
            Type::Address => {
                *count += 1;
                (MoveTypeLayout::Address, false)
            },
            Type::Signer => {
                *count += 1;
                (MoveTypeLayout::Signer, false)
            },
            Type::Vector(ty) => {
                *count += 1;
                let (layout, has_identifier_mappings) =
                    self.type_to_type_layout_impl(ty, module_store, count, depth + 1)?;
                (
                    MoveTypeLayout::Vector(Box::new(layout)),
                    has_identifier_mappings,
                )
            },
            Type::Struct { idx, .. } => {
                *count += 1;
                // Note depth is incread inside struct_name_to_type_layout instead.
                let (layout, has_identifier_mappings) =
                    self.struct_name_to_type_layout(module_store, *idx, &[], count, depth)?;
                (MoveTypeLayout::Struct(layout), has_identifier_mappings)
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                *count += 1;
                // Note depth is incread inside struct_name_to_type_layout instead.
                let (layout, has_identifier_mappings) =
                    self.struct_name_to_type_layout(module_store, *idx, ty_args, count, depth)?;
                (MoveTypeLayout::Struct(layout), has_identifier_mappings)
            },
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("no type layout for {:?}", ty)),
                );
            },
        })
    }

    fn struct_name_to_fully_annotated_layout(
        &self,
        struct_idx: StructNameIndex,
        module_store: &ModuleStorageAdapter,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveStructLayout> {
        let name = &*self.name_cache.idx_to_identifier(struct_idx);
        if let Some(struct_map) = self.type_cache.read().structs.get(name) {
            if let Some(struct_info) = struct_map.get(ty_args) {
                if let Some(annotated_node_count) = &struct_info.annotated_node_count {
                    *count += *annotated_node_count
                }
                if let Some(layout) = &struct_info.annotated_struct_layout {
                    return Ok(layout.clone());
                }
            }
        }

        let struct_type = module_store.get_struct_type_by_identifier(&name.name, &name.module)?;
        if struct_type.fields.len() != struct_type.field_names.len() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    "Field types did not match the length of field names in loaded struct"
                        .to_owned(),
                ),
            );
        }

        let count_before = *count;
        let mut gas_context = PseudoGasContext {
            cost: 0,
            max_cost: self.vm_config.type_max_cost,
            cost_base: self.vm_config.type_base_cost,
            cost_per_byte: self.vm_config.type_byte_cost,
        };
        let struct_tag = self.struct_name_to_type_tag(struct_idx, ty_args, &mut gas_context)?;
        let field_layouts = struct_type
            .field_names
            .iter()
            .zip(&struct_type.fields)
            .map(|(n, ty)| {
                let ty = self.subst(ty, ty_args)?;
                let l =
                    self.type_to_fully_annotated_layout_impl(&ty, module_store, count, depth + 1)?;
                Ok(MoveFieldLayout::new(n.clone(), l))
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        let struct_layout = MoveStructLayout::with_types(struct_tag, field_layouts);
        let field_node_count = *count - count_before;

        let mut cache = self.type_cache.write();
        let info = cache
            .structs
            .entry(name.clone())
            .or_default()
            .entry(ty_args.to_vec())
            .or_insert_with(StructInfoCache::new);
        info.annotated_struct_layout = Some(struct_layout.clone());
        info.annotated_node_count = Some(field_node_count);

        Ok(struct_layout)
    }

    fn type_to_fully_annotated_layout_impl(
        &self,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveTypeLayout> {
        if *count > MAX_TYPE_TO_LAYOUT_NODES {
            return Err(PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES));
        }
        if depth > VALUE_DEPTH_MAX {
            return Err(PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
        }
        Ok(match ty {
            Type::Bool => MoveTypeLayout::Bool,
            Type::U8 => MoveTypeLayout::U8,
            Type::U16 => MoveTypeLayout::U16,
            Type::U32 => MoveTypeLayout::U32,
            Type::U64 => MoveTypeLayout::U64,
            Type::U128 => MoveTypeLayout::U128,
            Type::U256 => MoveTypeLayout::U256,
            Type::Address => MoveTypeLayout::Address,
            Type::Signer => MoveTypeLayout::Signer,
            Type::Vector(ty) => MoveTypeLayout::Vector(Box::new(
                self.type_to_fully_annotated_layout_impl(ty, module_store, count, depth + 1)?,
            )),
            Type::Struct { idx, .. } => MoveTypeLayout::Struct(
                self.struct_name_to_fully_annotated_layout(*idx, module_store, &[], count, depth)?,
            ),
            Type::StructInstantiation {
                idx: name, ty_args, ..
            } => MoveTypeLayout::Struct(self.struct_name_to_fully_annotated_layout(
                *name,
                module_store,
                ty_args,
                count,
                depth,
            )?),
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("no type layout for {:?}", ty)),
                );
            },
        })
    }

    pub(crate) fn calculate_depth_of_struct(
        &self,
        struct_idx: StructNameIndex,
        module_store: &ModuleStorageAdapter,
    ) -> PartialVMResult<DepthFormula> {
        let name = &*self.name_cache.idx_to_identifier(struct_idx);
        if let Some(depth_formula) = self.type_cache.read().depth_formula.get(name) {
            return Ok(depth_formula.clone());
        }

        let struct_type = module_store.get_struct_type_by_identifier(&name.name, &name.module)?;

        let formulas = struct_type
            .fields
            .iter()
            .map(|field_type| self.calculate_depth_of_type(field_type, module_store))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let formula = DepthFormula::normalize(formulas);
        let prev = self
            .type_cache
            .write()
            .depth_formula
            .insert(name.clone(), formula.clone());
        if prev.is_some() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Recursive type?".to_owned()),
            );
        }
        Ok(formula)
    }

    fn calculate_depth_of_type(
        &self,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
    ) -> PartialVMResult<DepthFormula> {
        Ok(match ty {
            Type::Bool
            | Type::U8
            | Type::U64
            | Type::U128
            | Type::Address
            | Type::Signer
            | Type::U16
            | Type::U32
            | Type::U256 => DepthFormula::constant(1),
            Type::Vector(ty) => {
                let mut inner = self.calculate_depth_of_type(ty, module_store)?;
                inner.scale(1);
                inner
            },
            Type::Reference(ty) | Type::MutableReference(ty) => {
                let mut inner = self.calculate_depth_of_type(ty, module_store)?;
                inner.scale(1);
                inner
            },
            Type::TyParam(ty_idx) => DepthFormula::type_parameter(*ty_idx),
            Type::Struct { idx, .. } => {
                let mut struct_formula = self.calculate_depth_of_struct(*idx, module_store)?;
                debug_assert!(struct_formula.terms.is_empty());
                struct_formula.scale(1);
                struct_formula
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                let ty_arg_map = ty_args
                    .iter()
                    .enumerate()
                    .map(|(idx, ty)| {
                        let var = idx as TypeParameterIndex;
                        Ok((var, self.calculate_depth_of_type(ty, module_store)?))
                    })
                    .collect::<PartialVMResult<BTreeMap<_, _>>>()?;
                let struct_formula = self.calculate_depth_of_struct(*idx, module_store)?;
                let mut subst_struct_formula = struct_formula.subst(ty_arg_map)?;
                subst_struct_formula.scale(1);
                subst_struct_formula
            },
        })
    }

    pub(crate) fn type_to_type_tag(&self, ty: &Type) -> PartialVMResult<TypeTag> {
        let mut gas_context = PseudoGasContext {
            cost: 0,
            max_cost: self.vm_config.type_max_cost,
            cost_base: self.vm_config.type_base_cost,
            cost_per_byte: self.vm_config.type_byte_cost,
        };
        self.type_to_type_tag_impl(ty, &mut gas_context)
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let mut count = 0;
        self.type_to_type_layout_impl(ty, module_store, &mut count, 1)
    }

    pub(crate) fn type_to_type_layout(
        &self,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
    ) -> PartialVMResult<MoveTypeLayout> {
        let mut count = 0;
        let (layout, _has_identifier_mappings) =
            self.type_to_type_layout_impl(ty, module_store, &mut count, 1)?;
        Ok(layout)
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
        module_store: &ModuleStorageAdapter,
    ) -> PartialVMResult<MoveTypeLayout> {
        let mut count = 0;
        self.type_to_fully_annotated_layout_impl(ty, module_store, &mut count, 1)
    }
}

// Public APIs for external uses.
impl Loader {
    pub(crate) fn get_type_layout(
        &self,
        type_tag: &TypeTag,
        move_storage: &TransactionDataCache,
        module_storage: &ModuleStorageAdapter,
    ) -> VMResult<MoveTypeLayout> {
        let ty = self.load_type(type_tag, move_storage, module_storage)?;
        self.type_to_type_layout(&ty, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub(crate) fn get_fully_annotated_type_layout(
        &self,
        type_tag: &TypeTag,
        move_storage: &TransactionDataCache,
        module_storage: &ModuleStorageAdapter,
    ) -> VMResult<MoveTypeLayout> {
        let ty = self.load_type(type_tag, move_storage, module_storage)?;
        self.type_to_fully_annotated_layout(&ty, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }
}
