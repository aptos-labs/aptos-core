// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    data_cache::TransactionDataCache,
    loader::modules::{StructVariantInfo, VariantFieldInfo},
    logging::expect_no_verification_errors,
    module_traversal::TraversalContext,
    native_functions::NativeFunctions,
    storage::{
        loader::LoaderV2,
        module_storage::{FunctionValueExtensionAdapter, ModuleStorage},
        ty_cache::StructInfoCache,
        ty_tag_cache::TypeTagBuilder,
    },
    CodeStorage,
};
use hashbrown::Equivalent;
use lazy_static::lazy_static;
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{verification_error, Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        CompiledModule, CompiledScript, Constant, ConstantPoolIndex, FieldHandleIndex,
        FieldInstantiationIndex, FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructFieldInformation,
        StructVariantHandleIndex, StructVariantInstantiationIndex, TableIndex, TypeParameterIndex,
        VariantFieldHandleIndex, VariantFieldInstantiationIndex, VariantIndex,
    },
    IndexKind,
};
use move_bytecode_verifier::{self, cyclic_dependencies, dependencies};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::{NumBytes, NumTypeNodes},
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{IdentifierMappingKind, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::{
        runtime_types::{
            AbilityInfo, DepthFormula, StructIdentifier, StructLayout, StructType, Type,
            TypeBuilder,
        },
        struct_name_indexing::{StructNameIndex, StructNameIndexMap},
    },
    sha3_256,
    value_serde::FunctionValueExtension,
};
use parking_lot::{Mutex, RwLock};
use std::{
    collections::{btree_map, BTreeMap, BTreeSet},
    hash::Hash,
    sync::Arc,
};
use type_loader::intern_type;
use typed_arena::Arena;

mod access_specifier_loader;
mod function;
mod modules;
mod script;
mod type_loader;

pub use function::{Function, LoadedFunction};
pub(crate) use function::{FunctionHandle, FunctionInstantiation, LoadedFunctionOwner};
pub use modules::Module;
pub(crate) use modules::{LegacyModuleCache, LegacyModuleStorage, LegacyModuleStorageAdapter};
pub use script::Script;
pub(crate) use script::ScriptCache;

type ScriptHash = [u8; 32];

// A simple cache that offers both a HashMap and a Vector lookup.
// Access to this cache is always under a `RwLock`.
#[derive(Clone)]
pub(crate) struct BinaryCache<K, V> {
    // Notice that we are using the HashMap implementation from the hashbrown crate, not the
    // one from std, as it allows alternative key representations to be used for lookup,
    // making certain optimizations possible.
    id_map: hashbrown::HashMap<K, usize>,
    binaries: Vec<V>,
}

impl<K, V> BinaryCache<K, V>
where
    K: Eq + Hash,
{
    fn new() -> Self {
        Self {
            id_map: hashbrown::HashMap::new(),
            binaries: vec![],
        }
    }

    fn insert(&mut self, key: K, binary: V) -> &V {
        let idx = self.binaries.len();
        self.binaries.push(binary);
        self.id_map.insert(key, idx);
        self.binaries
            .last()
            .expect("BinaryCache: last() after push() impossible failure")
    }

    fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: Hash + Eq + Equivalent<K>,
    {
        let index = self.id_map.get(key)?;
        self.binaries.get(*index)
    }
}

// Max number of modules that can skip re-verification.
const VERIFIED_CACHE_SIZE: usize = 100_000;

// Cache for already verified modules
lazy_static! {
    static ref VERIFIED_MODULES: Mutex<lru::LruCache<[u8; 32], ()>> =
        Mutex::new(lru::LruCache::new(VERIFIED_CACHE_SIZE));
}

//
// Loader
//

#[derive(Clone)]
pub(crate) enum Loader {
    V1(LoaderV1),
    V2(LoaderV2),
}

macro_rules! versioned_loader_getter {
    ($getter:ident, $return_ty:ty) => {
        pub(crate) fn $getter(&self) -> &$return_ty {
            match self {
                Self::V1(loader) => loader.$getter(),
                Self::V2(loader) => loader.$getter(),
            }
        }
    };
}

impl Loader {
    versioned_loader_getter!(vm_config, VMConfig);

    versioned_loader_getter!(ty_builder, TypeBuilder);

    fn ty_cache<'a>(&'a self, module_storage: &'a dyn ModuleStorage) -> &StructInfoCache {
        match self {
            Self::V1(loader) => &loader.type_cache,
            Self::V2(_) => module_storage.runtime_environment().ty_cache(),
        }
    }

    pub(crate) fn struct_name_index_map<'a>(
        &'a self,
        module_storage: &'a dyn ModuleStorage,
    ) -> &StructNameIndexMap {
        match self {
            Self::V1(loader) => &loader.name_cache,
            Self::V2(_) => module_storage.runtime_environment().struct_name_index_map(),
        }
    }

    pub(crate) fn v1(natives: NativeFunctions, vm_config: VMConfig) -> Self {
        Self::V1(LoaderV1 {
            scripts: RwLock::new(ScriptCache::new()),
            type_cache: StructInfoCache::empty(),
            name_cache: StructNameIndexMap::empty(),
            natives,
            invalidated: RwLock::new(false),
            module_cache_hits: RwLock::new(BTreeSet::new()),
            vm_config,
        })
    }

    pub(crate) fn v2(vm_config: VMConfig) -> Self {
        Self::V2(LoaderV2::new(vm_config))
    }

    /// Flush this cache if it is marked as invalidated.
    #[deprecated]
    pub(crate) fn flush_v1_if_invalidated(&self) {
        match self {
            Self::V1(loader) => {
                let mut invalidated = loader.invalidated.write();
                if *invalidated {
                    *loader.scripts.write() = ScriptCache::new();
                    loader.type_cache.flush();
                    loader.name_cache.flush();
                    *invalidated = false;
                }
            },
            Self::V2(_) => unreachable!("Loader V2 cannot be flushed"),
        }
    }

    /// Mark this cache as invalidated.
    #[deprecated]
    pub(crate) fn mark_v1_as_invalid(&self) {
        match self {
            Self::V1(loader) => {
                *loader.invalidated.write() = true;
            },
            Self::V2(_) => unreachable!("Loader V2 cannot be marked as invalid"),
        }
    }

    /// Check whether this cache is invalidated.
    #[deprecated]
    pub(crate) fn is_v1_invalidated(&self) -> bool {
        match self {
            Self::V1(loader) => *loader.invalidated.read(),
            Self::V2(_) => unreachable!("Loader V2 is never invalidated"),
        }
    }

    pub(crate) fn check_script_dependencies_and_check_gas(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        script_blob: &[u8],
        code_storage: &impl CodeStorage,
    ) -> VMResult<()> {
        match self {
            Self::V1(loader) => loader.check_script_dependencies_and_check_gas(
                module_store,
                data_store,
                gas_meter,
                traversal_context,
                script_blob,
            ),
            Self::V2(loader) => loader.check_script_dependencies_and_check_gas(
                code_storage,
                gas_meter,
                traversal_context,
                script_blob,
            ),
        }
    }

    pub(crate) fn check_dependencies_and_charge_gas<'a, I>(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        visited: &mut BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,
        referenced_modules: &'a Arena<Arc<CompiledModule>>,
        ids: I,
        module_storage: &dyn ModuleStorage,
    ) -> VMResult<()>
    where
        I: IntoIterator<Item = (&'a AccountAddress, &'a IdentStr)>,
        I::IntoIter: DoubleEndedIterator,
    {
        let _timer = VM_TIMER.timer_with_label("Loader::check_dependencies_and_charge_gas");
        match self {
            Self::V1(loader) => loader.check_dependencies_and_charge_gas(
                module_store,
                data_store,
                gas_meter,
                visited,
                referenced_modules,
                ids,
            ),
            Self::V2(loader) => loader.check_dependencies_and_charge_gas(
                module_storage,
                gas_meter,
                visited,
                referenced_modules,
                ids,
            ),
        }
    }

    pub(crate) fn load_script(
        &self,
        script_blob: &[u8],
        ty_args: &[TypeTag],
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        code_storage: &impl CodeStorage,
    ) -> VMResult<LoadedFunction> {
        match self {
            Self::V1(loader) => loader.load_script(script_blob, ty_args, data_store, module_store),
            Self::V2(loader) => loader.load_script(code_storage, script_blob, ty_args),
        }
    }

    fn load_function_without_type_args(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        match self {
            Loader::V1(loader) => {
                // Need to load the module first, before resolving it and the function.
                loader.load_module(module_id, data_store, module_store)?;
                module_store
                    .resolve_module_and_function_by_name(module_id, function_name)
                    .map_err(|err| err.finish(Location::Undefined))
            },
            Loader::V2(_) => module_storage.fetch_function_definition(
                module_id.address(),
                module_id.name(),
                function_name,
            ),
        }
    }

    //
    // Helpers for loading and verification
    //

    pub(crate) fn load_type(
        &self,
        ty_tag: &TypeTag,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<Type> {
        match self {
            Self::V1(loader) => loader.load_type(ty_tag, data_store, module_store),
            Self::V2(_) => module_storage
                .fetch_ty(ty_tag)
                .map_err(|e| e.finish(Location::Undefined)),
        }
    }

    //
    // Internal helpers
    //

    pub fn fetch_struct_ty_by_idx(
        &self,
        idx: StructNameIndex,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<Arc<StructType>> {
        // Ensure we do not return a guard here (still holding the lock) because loading the struct
        // type below can fetch struct type by index recursively.
        let struct_name = self
            .struct_name_index_map(module_storage)
            .idx_to_struct_name_ref(idx)?;

        match self {
            Loader::V1(_) => {
                module_store.get_struct_type_by_identifier(&struct_name.name, &struct_name.module)
            },
            Loader::V2(_) => module_storage.fetch_struct_ty(
                struct_name.module.address(),
                struct_name.module.name(),
                struct_name.name.as_ident_str(),
            ),
        }
    }
}

// A Loader is responsible to load scripts and modules and holds the cache of all loaded
// entities. Each cache is protected by a `RwLock`. Operation in the Loader must be thread safe
// (operating on values on the stack) and when cache needs updating the mutex must be taken.
// The `pub(crate)` API is what a Loader offers to the runtime.
pub(crate) struct LoaderV1 {
    scripts: RwLock<ScriptCache>,
    type_cache: StructInfoCache,
    natives: NativeFunctions,
    pub(crate) name_cache: StructNameIndexMap,

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

impl Clone for LoaderV1 {
    fn clone(&self) -> Self {
        Self {
            scripts: RwLock::new(self.scripts.read().clone()),
            type_cache: self.type_cache.clone(),
            natives: self.natives.clone(),
            name_cache: self.name_cache.clone(),
            invalidated: RwLock::new(*self.invalidated.read()),
            module_cache_hits: RwLock::new(self.module_cache_hits.read().clone()),
            vm_config: self.vm_config.clone(),
        }
    }
}

impl LoaderV1 {
    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config.ty_builder
    }

    //
    // Script verification and loading
    //

    pub(crate) fn check_script_dependencies_and_check_gas(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        script_blob: &[u8],
    ) -> VMResult<()> {
        let script =
            data_store.load_compiled_script_to_cache(script_blob, sha3_256(script_blob))?;
        let script = traversal_context.referenced_scripts.alloc(script);

        // TODO(Gas): Should we charge dependency gas for the script itself?
        self.check_dependencies_and_charge_gas(
            module_store,
            data_store,
            gas_meter,
            &mut traversal_context.visited,
            traversal_context.referenced_modules,
            script.immediate_dependencies_iter(),
        )?;

        Ok(())
    }

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
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
    ) -> VMResult<LoadedFunction> {
        // Retrieve or load the script.
        let hash_value = sha3_256(script_blob);
        let mut scripts = self.scripts.write();
        let script = match scripts.get(&hash_value) {
            Some(cached) => cached,
            None => {
                let ver_script = self.deserialize_and_verify_script(
                    script_blob,
                    hash_value,
                    data_store,
                    module_store,
                )?;

                let script = Script::new(ver_script, &self.name_cache)
                    .map_err(|e| e.finish(Location::Script))?;
                scripts.insert(hash_value, script)
            },
        };

        let ty_args = ty_args
            .iter()
            .map(|ty| self.load_type(ty, data_store, module_store))
            .collect::<VMResult<Vec<_>>>()?;

        let main = script.entry_point();
        Type::verify_ty_arg_abilities(main.ty_param_abilities(), &ty_args).map_err(|e| {
            e.with_message(format!(
                "Failed to verify type arguments for script {}",
                main.name()
            ))
            .finish(Location::Script)
        })?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Script(script),
            ty_args,
            function: main,
        })
    }

    // The process of deserialization and verification is not and it must not be under lock.
    // So when publishing modules through the dependency DAG it may happen that a different
    // thread had loaded the module after this process fetched it from storage.
    // Caching will take care of that by asking for each dependency module again under lock.
    fn deserialize_and_verify_script(
        &self,
        script: &[u8],
        hash_value: ScriptHash,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
    ) -> VMResult<Arc<CompiledScript>> {
        let _timer = VM_TIMER.timer_with_label("LoaderV1::deserialize_and_verify_script");

        let script = data_store.load_compiled_script_to_cache(script, hash_value)?;

        // Verification:
        //   - Local, using a bytecode verifier.
        //   - Global, loading & verifying module dependencies.
        move_bytecode_verifier::verify_script_with_config(
            &self.vm_config().verifier_config,
            script.as_ref(),
        )?;
        let loaded_deps = script
            .immediate_dependencies()
            .into_iter()
            .map(|module_id| self.load_module(&module_id, data_store, module_store))
            .collect::<VMResult<Vec<_>>>()?;
        dependencies::verify_script(&script, loaded_deps.iter().map(|m| m.as_ref().as_ref()))?;
        Ok(script)
    }

    //
    // Module verification and loading
    //
}

impl Loader {
    // Loading verifies the module if it was never loaded.
    // Type parameters are inferred from the expected return type. Returns an error if it's not
    // possible to infer the type parameters or return type cannot be matched.
    // The type parameters are verified with capabilities.
    pub(crate) fn load_function_with_type_arg_inference(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        expected_return_type: &Type,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<LoadedFunction> {
        let (module, function) = self.load_function_without_type_args(
            module_id,
            function_name,
            data_store,
            module_store,
            module_storage,
        )?;

        if function.return_tys().len() != 1 {
            // For functions that are marked constructor this should not happen.
            return Err(PartialVMError::new(StatusCode::ABORTED).finish(Location::Undefined));
        }

        let mut map = BTreeMap::new();
        if !match_return_type(&function.return_tys()[0], expected_return_type, &mut map) {
            // For functions that are marked constructor this should not happen.
            return Err(
                PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                    .finish(Location::Undefined),
            );
        }

        // Construct the type arguments from the match
        let mut ty_args = vec![];
        let num_ty_args = function.ty_param_abilities().len();
        for i in 0..num_ty_args {
            if let Some(t) = map.get(&(i as u16)) {
                ty_args.push((*t).clone());
            } else {
                // Unknown type argument we are not able to infer the type arguments.
                // For functions that are marked constructor this should not happen.
                return Err(
                    PartialVMError::new(StatusCode::INVALID_MAIN_FUNCTION_SIGNATURE)
                        .finish(Location::Undefined),
                );
            }
        }

        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }

    // Loading verifies the module if it was never loaded.
    // Type parameters are checked as well after every type is loaded.
    pub(crate) fn load_function(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_args: &[TypeTag],
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<LoadedFunction> {
        let _timer = VM_TIMER.timer_with_label("Loader::load_function");

        let (module, function) = self.load_function_without_type_args(
            module_id,
            function_name,
            data_store,
            module_store,
            module_storage,
        )?;

        let ty_args = ty_args
            .iter()
            .map(|ty_arg| self.load_type(ty_arg, data_store, module_store, module_storage))
            .collect::<VMResult<Vec<_>>>()
            .map_err(|mut err| {
                // User provided type argument failed to load. Set extra sub status to distinguish from internal type loading error.
                if StatusCode::TYPE_RESOLUTION_FAILURE == err.major_status() {
                    err.set_sub_status(move_core_types::vm_status::sub_status::type_resolution_failure::EUSER_TYPE_LOADING_FAILURE);
                }
                err
            })?;

        Type::verify_ty_arg_abilities(function.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Module(module_id.clone())))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args,
            function,
        })
    }
}

impl LoaderV1 {
    // Entry point for module publishing (`MoveVM::publish_module_bundle`).
    //
    // All modules in the bundle to be published must be loadable. This function performs all
    // verification steps to load these modules without actually loading them into the code cache.
    pub(crate) fn verify_module_bundle_for_publication(
        &self,
        modules: &[CompiledModule],
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
    ) -> VMResult<()> {
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
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
    ) -> VMResult<()> {
        // Performs all verification steps to load the module without loading it, i.e., the new
        // module will NOT show up in `module_cache`. In the module republishing case, it means
        // that the old module is still in the `module_cache`, unless a new Loader is created,
        // which means that a new MoveVM instance needs to be created.
        move_bytecode_verifier::verify_module_with_config(
            &self.vm_config().verifier_config,
            module,
        )?;
        check_natives(module)?;

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
        module_store: &LegacyModuleStorageAdapter,
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

    //
    // Helpers for loading and verification
    //

    pub(crate) fn load_type(
        &self,
        ty_tag: &TypeTag,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
    ) -> VMResult<Type> {
        let resolver = |struct_tag: &StructTag| -> VMResult<Arc<StructType>> {
            let module_id = ModuleId::new(struct_tag.address, struct_tag.module.clone());
            self.load_module(&module_id, data_store, module_store)?;
            module_store
                .get_struct_type_by_identifier(&struct_tag.name, &module_id)
                .map_err(|e| e.finish(Location::Undefined))
        };
        self.ty_builder().create_ty(ty_tag, resolver)
    }

    /// Traverses the whole transitive closure of dependencies, starting from the specified
    /// modules and performs gas metering.
    ///
    /// The traversal follows a depth-first order, with the module itself being visited first,
    /// followed by its dependencies, and finally its friends.
    /// DO NOT CHANGE THE ORDER unless you have a good reason, or otherwise this could introduce
    /// a breaking change to the gas semantics.
    ///
    /// This will result in the shallow-loading of the modules -- they will be read from the
    /// storage as bytes and then deserialized, but NOT converted into the runtime representation.
    ///
    /// It should also be noted that this is implemented in a way that avoids the cloning of
    /// `ModuleId`, a.k.a. heap allocations, as much as possible, which is critical for
    /// performance.
    ///
    /// TODO: Revisit the order of traversal. Consider switching to alphabetical order.
    pub(crate) fn check_dependencies_and_charge_gas<'a, I>(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        data_store: &mut TransactionDataCache,
        gas_meter: &mut impl GasMeter,
        visited: &mut BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,
        referenced_modules: &'a Arena<Arc<CompiledModule>>,
        ids: I,
    ) -> VMResult<()>
    where
        I: IntoIterator<Item = (&'a AccountAddress, &'a IdentStr)>,
        I::IntoIter: DoubleEndedIterator,
    {
        // Initialize the work list (stack) and the map of visited modules.
        //
        // TODO: Determine the reserved capacity based on the max number of dependencies allowed.
        let mut stack = Vec::with_capacity(512);

        for (addr, name) in ids.into_iter().rev() {
            // TODO: Allow the check of special addresses to be customized.
            if !addr.is_special() && visited.insert((addr, name), ()).is_none() {
                stack.push((addr, name, true));
            }
        }

        while let Some((addr, name, allow_loading_failure)) = stack.pop() {
            // Load and deserialize the module only if it has not been cached by the loader.
            // Otherwise, this will cause a significant regression in performance.
            let (module, size) = match module_store.module_at_by_ref(addr, name) {
                Some(module) => (module.module.clone(), module.size),
                None => {
                    let (module, size, _) = data_store.load_compiled_module_to_cache(
                        ModuleId::new(*addr, name.to_owned()),
                        allow_loading_failure,
                    )?;
                    (module, size)
                },
            };

            // Extend the lifetime of the module to the remainder of the function body
            // by storing it in an arena.
            //
            // This is needed because we need to store references derived from it in the
            // work list.
            let module = referenced_modules.alloc(module);

            gas_meter
                .charge_dependency(false, addr, name, NumBytes::new(size as u64))
                .map_err(|err| {
                    err.finish(Location::Module(ModuleId::new(*addr, name.to_owned())))
                })?;

            // Explore all dependencies and friends that have been visited yet.
            for (addr, name) in module
                .immediate_dependencies_iter()
                .chain(module.immediate_friends_iter())
                .rev()
            {
                // TODO: Allow the check of special addresses to be customized.
                if !addr.is_special() && visited.insert((addr, name), ()).is_none() {
                    stack.push((addr, name, false));
                }
            }
        }

        Ok(())
    }

    // The interface for module loading. Aligned with `load_type` and `load_function`, this function
    // verifies that the module is OK instead of expect it.
    pub(crate) fn load_module(
        &self,
        id: &ModuleId,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
    ) -> VMResult<Arc<Module>> {
        // if the module is already in the code cache, load the cached version
        if let Some(cached) = module_store.module_at(id) {
            self.module_cache_hits.write().insert(id.clone());
            return Ok(cached);
        }

        let _timer = VM_TIMER.timer_with_label("LoaderV1::load_module [cache miss]");

        // otherwise, load the transitive closure of the target module
        let module_ref = self.load_and_verify_module_and_dependencies_and_friends(
            id,
            &BTreeMap::new(),
            &BTreeSet::new(),
            data_store,
            module_store,
            /* allow_module_loading_failure */ true,
        )?;

        // verify that the transitive closure does not have cycles
        self.verify_module_cyclic_relations(
            &module_ref,
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
        data_store: &mut TransactionDataCache,
        allow_loading_failure: bool,
    ) -> VMResult<(Arc<CompiledModule>, usize)> {
        let (module, size, hash_value) =
            data_store.load_compiled_module_to_cache(id.clone(), allow_loading_failure)?;

        if self.vm_config.paranoid_type_checks && &module.self_id() != id {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                    .with_message("Module self id mismatch with storage".to_string())
                    .finish(Location::Module(id.clone())),
            );
        }

        // Verify the module if it hasn't been verified before.
        if VERIFIED_MODULES.lock().get(&hash_value).is_none() {
            let _timer = VM_TIMER
                .timer_with_label("LoaderV1::load_and_verify_module [verification cache miss]");

            move_bytecode_verifier::verify_module_with_config(
                &self.vm_config.verifier_config,
                &module,
            )
            .map_err(expect_no_verification_errors)?;

            VERIFIED_MODULES.lock().put(hash_value, ());
        }

        check_natives(&module).map_err(expect_no_verification_errors)?;
        Ok((module, size))
    }

    // Everything in `load_and_verify_module` and also recursively load and verify all the
    // dependencies of the target module.
    fn load_and_verify_module_and_dependencies(
        &self,
        id: &ModuleId,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        visited: &mut BTreeSet<ModuleId>,
        friends_discovered: &mut BTreeSet<ModuleId>,
        allow_module_loading_failure: bool,
    ) -> VMResult<Arc<Module>> {
        // dependency loading does not permit cycles
        if visited.contains(id) {
            return Err(PartialVMError::new(StatusCode::CYCLIC_MODULE_DEPENDENCY)
                .finish(Location::Undefined));
        }

        // module self-check
        let (module, size) =
            self.load_and_verify_module(id, data_store, allow_module_loading_failure)?;
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
        )?;

        // if linking goes well, insert the module to the code cache
        let module_ref =
            module_store.insert(&self.natives, id.clone(), size, module, &self.name_cache)?;

        Ok(module_ref)
    }

    // downward exploration of the module's dependency graph
    fn load_and_verify_dependencies(
        &self,
        module: &CompiledModule,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        visited: &mut BTreeSet<ModuleId>,
        friends_discovered: &mut BTreeSet<ModuleId>,
        allow_dependency_loading_failure: bool,
    ) -> VMResult<()> {
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
                    )?,
                    Some(cached) => cached,
                };
                cached_deps.push(loaded);
            }
        }

        // once all dependencies are loaded, do the linking check
        let all_imm_deps = bundle_deps
            .into_iter()
            .chain(cached_deps.iter().map(|m| m.as_ref().as_ref()));
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
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        allow_module_loading_failure: bool,
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
        )?;
        Ok(module_ref)
    }

    // upward exploration of the module's dependency graph
    fn load_and_verify_friends(
        &self,
        friends_discovered: BTreeSet<ModuleId>,
        bundle_verified: &BTreeMap<ModuleId, CompiledModule>,
        bundle_unverified: &BTreeSet<ModuleId>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        allow_friend_loading_failure: bool,
    ) -> VMResult<()> {
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
            )?;
        }
        Ok(())
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
    module_store: &'a LegacyModuleStorageAdapter,
    binary: BinaryType,

    module_storage: &'a dyn ModuleStorage,
}

impl<'a> Resolver<'a> {
    fn for_module(
        loader: &'a Loader,
        module_store: &'a LegacyModuleStorageAdapter,
        module_storage: &'a dyn ModuleStorage,
        module: Arc<Module>,
    ) -> Self {
        let binary = BinaryType::Module(module);
        Self {
            loader,
            binary,
            module_store,
            module_storage,
        }
    }

    fn for_script(
        loader: &'a Loader,
        module_store: &'a LegacyModuleStorageAdapter,
        module_storage: &'a dyn ModuleStorage,
        script: Arc<Script>,
    ) -> Self {
        let binary = BinaryType::Script(script);
        Self {
            loader,
            binary,
            module_store,
            module_storage,
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
}

macro_rules! build_loaded_function {
    ($function_name:ident, $idx_ty:ty, $get_function_handle:ident) => {
        pub(crate) fn $function_name(
            &self,
            idx: $idx_ty,
            verified_ty_args: Vec<Type>,
        ) -> PartialVMResult<LoadedFunction> {
            match &self.binary {
                BinaryType::Module(module) => {
                    let handle = module.$get_function_handle(idx.0);
                    match handle {
                        FunctionHandle::Local(function) => {
                            (Ok(LoadedFunction {
                                owner: LoadedFunctionOwner::Module(module.clone()),
                                ty_args: verified_ty_args,
                                function: function.clone(),
                            }))
                        },
                        FunctionHandle::Remote { module, name } => self
                            .build_loaded_function_from_name_and_ty_args(
                                module,
                                name,
                                verified_ty_args,
                            ),
                    }
                },
                BinaryType::Script(script) => {
                    let handle = script.$get_function_handle(idx.0);
                    match handle {
                        FunctionHandle::Local(_) => Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message("Scripts never have local functions".to_string())),
                        FunctionHandle::Remote { module, name } => self
                            .build_loaded_function_from_name_and_ty_args(
                                module,
                                name,
                                verified_ty_args,
                            ),
                    }
                },
            }
        }
    };
}

impl<'a> Resolver<'a> {
    build_loaded_function!(
        build_loaded_function_from_handle_and_ty_args,
        FunctionHandleIndex,
        function_at
    );

    build_loaded_function!(
        build_loaded_function_from_instantiation_and_ty_args,
        FunctionInstantiationIndex,
        function_instantiation_handle_at
    );

    pub(crate) fn build_loaded_function_from_name_and_ty_args(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        verified_ty_args: Vec<Type>,
    ) -> PartialVMResult<LoadedFunction> {
        let (module, function) = match self.loader() {
            Loader::V1(_) => self
                .module_store
                .resolve_module_and_function_by_name(module_id, function_name)?,
            Loader::V2(_) => self
                .module_storage
                .fetch_function_definition(module_id.address(), module_id.name(), function_name)
                .map_err(|_| {
                    // Note: legacy loader implementation used this error, so we need to remap.
                    PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE).with_message(
                        format!(
                            "Module or function do not exist for {}::{}::{}",
                            module_id.address(),
                            module_id.name(),
                            function_name
                        ),
                    )
                })?,
        };
        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args: verified_ty_args,
            function,
        })
    }

    pub(crate) fn instantiate_generic_function(
        &self,
        gas_meter: Option<&mut impl GasMeter>,
        idx: FunctionInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let instantiation = match &self.binary {
            BinaryType::Module(module) => module.function_instantiation_at(idx.0),
            BinaryType::Script(script) => script.function_instantiation_at(idx.0),
        };

        if let Some(gas_meter) = gas_meter {
            for ty in instantiation {
                gas_meter
                    .charge_create_ty(NumTypeNodes::new(ty.num_nodes_in_subst(ty_args)? as u64))?;
            }
        }

        let ty_builder = self.loader().ty_builder();
        let instantiation = instantiation
            .iter()
            .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok(instantiation)
    }

    //
    // Type resolution
    //

    pub(crate) fn get_struct_ty(&self, idx: StructDefinitionIndex) -> Type {
        let struct_ty = match &self.binary {
            BinaryType::Module(module) => module.struct_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        self.create_struct_ty(&struct_ty)
    }

    pub(crate) fn get_struct_variant_at(
        &self,
        idx: StructVariantHandleIndex,
    ) -> &StructVariantInfo {
        match &self.binary {
            BinaryType::Module(module) => module.struct_variant_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_struct_variant_instantiation_at(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> &StructVariantInfo {
        match &self.binary {
            BinaryType::Module(module) => module.struct_variant_instantiation_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_generic_struct_ty(
        &self,
        idx: StructDefInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        let struct_inst = match &self.binary {
            BinaryType::Module(module) => module.struct_instantiation_at(idx.0),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };

        let struct_ty = &struct_inst.definition_struct_type;
        let ty_builder = self.loader().ty_builder();
        ty_builder.create_struct_instantiation_ty(struct_ty, &struct_inst.instantiation, ty_args)
    }

    pub(crate) fn get_field_ty(&self, idx: FieldHandleIndex) -> PartialVMResult<&Type> {
        match &self.binary {
            BinaryType::Module(module) => {
                let handle = &module.field_handles[idx.0 as usize];
                Ok(&handle.field_ty)
            },
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_generic_field_ty(
        &self,
        idx: FieldInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        let field_instantiation = match &self.binary {
            BinaryType::Module(module) => &module.field_instantiations[idx.0 as usize],
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let field_ty = &field_instantiation.uninstantiated_field_ty;
        self.instantiate_ty(field_ty, ty_args, &field_instantiation.instantiation)
    }

    pub(crate) fn instantiate_ty(
        &self,
        ty: &Type,
        ty_args: &[Type],
        instantiation_tys: &[Type],
    ) -> PartialVMResult<Type> {
        let ty_builder = self.loader().ty_builder();
        let instantiation_tys = instantiation_tys
            .iter()
            .map(|inst_ty| ty_builder.create_ty_with_subst(inst_ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;
        ty_builder.create_ty_with_subst(ty, &instantiation_tys)
    }

    pub(crate) fn variant_field_info_at(&self, idx: VariantFieldHandleIndex) -> &VariantFieldInfo {
        match &self.binary {
            BinaryType::Module(module) => module.variant_field_info_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn variant_field_instantiation_info_at(
        &self,
        idx: VariantFieldInstantiationIndex,
    ) -> &VariantFieldInfo {
        match &self.binary {
            BinaryType::Module(module) => module.variant_field_instantiation_info_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_struct(
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
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(struct_ty, None, &struct_inst.instantiation, ty_args)
    }

    pub(crate) fn instantiate_generic_struct_variant_fields(
        &self,
        idx: StructVariantInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let struct_inst = match &self.binary {
            BinaryType::Module(module) => module.struct_variant_instantiation_at(idx),
            BinaryType::Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(
            struct_ty,
            Some(struct_inst.variant),
            &struct_inst.instantiation,
            ty_args,
        )
    }

    pub(crate) fn instantiate_generic_fields(
        &self,
        struct_ty: &Arc<StructType>,
        variant: Option<VariantIndex>,
        instantiation: &[Type],
        ty_args: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let ty_builder = self.loader().ty_builder();
        let instantiation_tys = instantiation
            .iter()
            .map(|inst_ty| ty_builder.create_ty_with_subst(inst_ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;

        struct_ty
            .fields(variant)?
            .iter()
            .map(|(_, inst_ty)| ty_builder.create_ty_with_subst(inst_ty, &instantiation_tys))
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
            self.loader().ty_builder().create_ty_with_subst(ty, ty_args)
        } else {
            Ok(ty.clone())
        }
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

    pub(crate) fn field_handle_to_struct(&self, idx: FieldHandleIndex) -> Type {
        match &self.binary {
            BinaryType::Module(module) => {
                let struct_ty = &module.field_handles[idx.0 as usize].definition_struct_type;
                self.loader()
                    .ty_builder()
                    .create_struct_ty(struct_ty.idx, AbilityInfo::struct_(struct_ty.abilities))
            },
            BinaryType::Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn field_instantiation_to_struct(
        &self,
        idx: FieldInstantiationIndex,
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        match &self.binary {
            BinaryType::Module(module) => {
                let field_inst = &module.field_instantiations[idx.0 as usize];
                let struct_ty = &field_inst.definition_struct_type;
                let ty_params = &field_inst.instantiation;
                self.create_struct_instantiation_ty(struct_ty, ty_params, ty_args)
            },
            BinaryType::Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn create_struct_ty(&self, struct_ty: &Arc<StructType>) -> Type {
        self.loader()
            .ty_builder()
            .create_struct_ty(struct_ty.idx, AbilityInfo::struct_(struct_ty.abilities))
    }

    pub(crate) fn create_struct_instantiation_ty(
        &self,
        struct_ty: &Arc<StructType>,
        ty_params: &[Type],
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        self.loader()
            .ty_builder()
            .create_struct_instantiation_ty(struct_ty, ty_params, ty_args)
    }

    pub(crate) fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        self.loader
            .type_to_type_layout(ty, self.module_store, self.module_storage)
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.loader.type_to_type_layout_with_identifier_mappings(
            ty,
            self.module_store,
            self.module_storage,
        )
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
    ) -> PartialVMResult<MoveTypeLayout> {
        self.loader
            .type_to_fully_annotated_layout(ty, self.module_store, self.module_storage)
    }

    //
    // Getters
    //

    pub(crate) fn loader(&self) -> &Loader {
        self.loader
    }

    pub(crate) fn module_store(&self) -> &LegacyModuleStorageAdapter {
        self.module_store
    }

    pub(crate) fn module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        self.loader().vm_config()
    }
}

impl<'a> FunctionValueExtension for Resolver<'a> {
    fn get_function_arg_tys(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        ty_arg_tags: Vec<TypeTag>,
    ) -> PartialVMResult<Vec<Type>> {
        let function_value_extension = FunctionValueExtensionAdapter {
            module_storage: self.module_storage,
        };
        function_value_extension.get_function_arg_tys(module_id, function_name, ty_arg_tags)
    }
}

/// Maximal depth of a value in terms of type depth.
pub const VALUE_DEPTH_MAX: u64 = 128;

/// Maximal nodes which are allowed when converting to layout. This includes the types of
/// fields for struct types.
const MAX_TYPE_TO_LAYOUT_NODES: u64 = 256;

pub(crate) struct PseudoGasContext {
    // Parameters for metering type tag construction:
    //   - maximum allowed cost,
    //   - base cost for any type to tag conversion,
    //   - cost for size of a struct tag.
    max_cost: u64,
    pub(crate) cost: u64,
    cost_base: u64,
    cost_per_byte: u64,
}

impl PseudoGasContext {
    pub(crate) fn new(vm_config: &VMConfig) -> Self {
        Self {
            max_cost: vm_config.type_max_cost,
            cost: 0,
            cost_base: vm_config.type_base_cost,
            cost_per_byte: vm_config.type_byte_cost,
        }
    }

    pub(crate) fn current_cost(&mut self) -> u64 {
        self.cost
    }

    pub(crate) fn charge_base(&mut self) -> PartialVMResult<()> {
        self.charge(self.cost_base)
    }

    pub(crate) fn charge_struct_tag(&mut self, struct_tag: &StructTag) -> PartialVMResult<()> {
        let size =
            (struct_tag.address.len() + struct_tag.module.len() + struct_tag.name.len()) as u64;
        self.charge(size * self.cost_per_byte)
    }

    pub(crate) fn charge(&mut self, amount: u64) -> PartialVMResult<()> {
        self.cost += amount;
        if self.cost > self.max_cost {
            Err(
                PartialVMError::new(StatusCode::TYPE_TAG_LIMIT_EXCEEDED).with_message(format!(
                    "Exceeded maximum type tag limit of {}",
                    self.max_cost
                )),
            )
        } else {
            Ok(())
        }
    }
}

impl LoaderV1 {
    fn struct_name_to_type_tag(
        &self,
        struct_name_idx: StructNameIndex,
        ty_args: &[Type],
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<StructTag> {
        if let Some((struct_tag, gas)) = self.type_cache.get_struct_tag(&struct_name_idx, ty_args) {
            gas_context.charge(gas)?;
            return Ok(struct_tag.clone());
        }

        let cur_cost = gas_context.current_cost();

        let type_args = ty_args
            .iter()
            .map(|ty| self.type_to_type_tag_impl(ty, gas_context))
            .collect::<PartialVMResult<Vec<_>>>()?;
        let struct_tag = self
            .name_cache
            .idx_to_struct_tag(struct_name_idx, type_args)?;

        gas_context.charge_struct_tag(&struct_tag)?;
        self.type_cache.store_struct_tag(
            struct_name_idx,
            ty_args.to_vec(),
            struct_tag.clone(),
            gas_context.current_cost() - cur_cost,
        );
        Ok(struct_tag)
    }

    fn type_to_type_tag_impl(
        &self,
        ty: &Type,
        gas_context: &mut PseudoGasContext,
    ) -> PartialVMResult<TypeTag> {
        gas_context.charge_base()?;
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
            Type::Vector(ty) => {
                let el_ty_tag = self.type_to_type_tag_impl(ty, gas_context)?;
                TypeTag::Vector(Box::new(el_ty_tag))
            },
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
                        .with_message(format!("No type tag for {:?}", ty)),
                );
            },
        })
    }
}

impl Loader {
    fn struct_name_to_type_layout(
        &self,
        struct_name_idx: StructNameIndex,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let ty_cache = self.ty_cache(module_storage);
        if let Some((struct_layout, node_count, has_identifier_mappings)) =
            ty_cache.get_struct_layout_info(&struct_name_idx, ty_args)
        {
            *count += node_count;
            return Ok((struct_layout, has_identifier_mappings));
        }

        let count_before = *count;
        let struct_type =
            self.fetch_struct_ty_by_idx(struct_name_idx, module_store, module_storage)?;

        let mut has_identifier_mappings = false;

        let layout = match &struct_type.layout {
            StructLayout::Single(fields) => {
                // Some types can have fields which are lifted at serialization or deserialization
                // times. Right now these are Aggregator and AggregatorSnapshot.
                let struct_name = self
                    .struct_name_index_map(module_storage)
                    .idx_to_struct_name_ref(struct_name_idx)?;
                let maybe_mapping = self.get_identifier_mapping_kind(struct_name.as_ref());

                let field_tys = fields
                    .iter()
                    .map(|(_, ty)| self.ty_builder().create_ty_with_subst(ty, ty_args))
                    .collect::<PartialVMResult<Vec<_>>>()?;
                let (mut field_layouts, field_has_identifier_mappings): (
                    Vec<MoveTypeLayout>,
                    Vec<bool>,
                ) = field_tys
                    .iter()
                    .map(|ty| {
                        self.type_to_type_layout_impl(
                            ty,
                            module_store,
                            module_storage,
                            count,
                            depth,
                        )
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?
                    .into_iter()
                    .unzip();

                has_identifier_mappings =
                    maybe_mapping.is_some() || field_has_identifier_mappings.into_iter().any(|b| b);

                let layout = if Some(IdentifierMappingKind::DerivedString) == maybe_mapping {
                    // For DerivedString, the whole object should be lifted.
                    MoveTypeLayout::Native(
                        IdentifierMappingKind::DerivedString,
                        Box::new(MoveTypeLayout::Struct(MoveStructLayout::new(field_layouts))),
                    )
                } else {
                    // For aggregators / snapshots, the first field should be lifted.
                    if let Some(kind) = &maybe_mapping {
                        if let Some(l) = field_layouts.first_mut() {
                            *l = MoveTypeLayout::Native(kind.clone(), Box::new(l.clone()));
                        }
                    }
                    MoveTypeLayout::Struct(MoveStructLayout::new(field_layouts))
                };
                layout
            },
            StructLayout::Variants(variants) => {
                // We do not support variants to have direct identifier mappings,
                // but their inner types may.
                let variant_layouts = variants
                    .iter()
                    .map(|variant| {
                        variant
                            .1
                            .iter()
                            .map(|(_, ty)| {
                                let ty = self.ty_builder().create_ty_with_subst(ty, ty_args)?;
                                let (ty, has_id_mappings) = self.type_to_type_layout_impl(
                                    &ty,
                                    module_store,
                                    module_storage,
                                    count,
                                    depth,
                                )?;
                                has_identifier_mappings |= has_id_mappings;
                                Ok(ty)
                            })
                            .collect::<PartialVMResult<Vec<_>>>()
                    })
                    .collect::<PartialVMResult<Vec<_>>>()?;
                MoveTypeLayout::Struct(MoveStructLayout::RuntimeVariants(variant_layouts))
            },
        };

        let field_node_count = *count - count_before;
        ty_cache.store_struct_layout_info(
            struct_name_idx,
            ty_args.to_vec(),
            layout.clone(),
            field_node_count,
            has_identifier_mappings,
        );
        Ok((layout, has_identifier_mappings))
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
        if !self.vm_config().delayed_field_optimization_enabled {
            return None;
        }

        let ident_str_to_kind = |ident_str: &IdentStr| -> Option<IdentifierMappingKind> {
            if ident_str.eq(ident_str!("Aggregator")) {
                Some(IdentifierMappingKind::Aggregator)
            } else if ident_str.eq(ident_str!("AggregatorSnapshot")) {
                Some(IdentifierMappingKind::Snapshot)
            } else if ident_str.eq(ident_str!("DerivedStringSnapshot")) {
                Some(IdentifierMappingKind::DerivedString)
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
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        if *count > MAX_TYPE_TO_LAYOUT_NODES {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Number of type nodes when constructing type layout exceeded the maximum of {}",
                    MAX_TYPE_TO_LAYOUT_NODES
                )),
            );
        }
        if depth > VALUE_DEPTH_MAX {
            return Err(
                PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED).with_message(format!(
                    "Depth of a layout exceeded the maximum of {} during construction",
                    VALUE_DEPTH_MAX
                )),
            );
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
                let (layout, has_identifier_mappings) = self.type_to_type_layout_impl(
                    ty,
                    module_store,
                    module_storage,
                    count,
                    depth + 1,
                )?;
                (
                    MoveTypeLayout::Vector(Box::new(layout)),
                    has_identifier_mappings,
                )
            },
            Type::Struct { idx, .. } => {
                *count += 1;
                let (layout, has_identifier_mappings) = self.struct_name_to_type_layout(
                    *idx,
                    module_store,
                    module_storage,
                    &[],
                    count,
                    depth + 1,
                )?;
                (layout, has_identifier_mappings)
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                *count += 1;
                let (layout, has_identifier_mappings) = self.struct_name_to_type_layout(
                    *idx,
                    module_store,
                    module_storage,
                    ty_args,
                    count,
                    depth + 1,
                )?;
                (layout, has_identifier_mappings)
            },
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type layout for {:?}", ty)),
                );
            },
        })
    }

    fn struct_name_to_fully_annotated_layout(
        &self,
        struct_name_idx: StructNameIndex,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveTypeLayout> {
        let ty_cache = self.ty_cache(module_storage);
        if let Some((layout, annotated_node_count)) =
            ty_cache.get_annotated_struct_layout_info(&struct_name_idx, ty_args)
        {
            *count += annotated_node_count;
            return Ok(layout);
        }

        let struct_type =
            self.fetch_struct_ty_by_idx(struct_name_idx, module_store, module_storage)?;

        // TODO(#13806): have annotated layouts for variants. Currently, we just return the raw
        //   layout for them.
        if matches!(struct_type.layout, StructLayout::Variants(_)) {
            return self
                .struct_name_to_type_layout(
                    struct_name_idx,
                    module_store,
                    module_storage,
                    ty_args,
                    count,
                    depth,
                )
                .map(|(l, _)| l);
        }

        let count_before = *count;
        let struct_tag = match self {
            Loader::V1(loader) => {
                let mut gas_context = PseudoGasContext::new(loader.vm_config());
                loader.struct_name_to_type_tag(struct_name_idx, ty_args, &mut gas_context)?
            },
            Loader::V2(_) => {
                let ty_tag_builder = TypeTagBuilder::new(module_storage.runtime_environment());
                ty_tag_builder.struct_name_idx_to_struct_tag(&struct_name_idx, ty_args)?
            },
        };
        let fields = struct_type.fields(None)?;

        let field_layouts = fields
            .iter()
            .map(|(n, ty)| {
                let ty = self.ty_builder().create_ty_with_subst(ty, ty_args)?;
                let l = self.type_to_fully_annotated_layout_impl(
                    &ty,
                    module_store,
                    module_storage,
                    count,
                    depth,
                )?;
                Ok(MoveFieldLayout::new(n.clone(), l))
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        let struct_layout =
            MoveTypeLayout::Struct(MoveStructLayout::with_types(struct_tag, field_layouts));
        let field_node_count = *count - count_before;

        ty_cache.store_annotated_struct_layout_info(
            struct_name_idx,
            ty_args.to_vec(),
            struct_layout.clone(),
            field_node_count,
        );
        Ok(struct_layout)
    }

    fn type_to_fully_annotated_layout_impl(
        &self,
        ty: &Type,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveTypeLayout> {
        if *count > MAX_TYPE_TO_LAYOUT_NODES {
            return Err(
                PartialVMError::new(StatusCode::TOO_MANY_TYPE_NODES).with_message(format!(
                    "Number of type nodes when constructing type layout exceeded the maximum of {}",
                    MAX_TYPE_TO_LAYOUT_NODES
                )),
            );
        }
        if depth > VALUE_DEPTH_MAX {
            return Err(
                PartialVMError::new(StatusCode::VM_MAX_VALUE_DEPTH_REACHED).with_message(format!(
                    "Depth of a layout exceeded the maximum of {} during construction",
                    VALUE_DEPTH_MAX
                )),
            );
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
            Type::Vector(ty) => {
                MoveTypeLayout::Vector(Box::new(self.type_to_fully_annotated_layout_impl(
                    ty,
                    module_store,
                    module_storage,
                    count,
                    depth + 1,
                )?))
            },
            Type::Struct { idx, .. } => self.struct_name_to_fully_annotated_layout(
                *idx,
                module_store,
                module_storage,
                &[],
                count,
                depth + 1,
            )?,
            Type::StructInstantiation { idx, ty_args, .. } => self
                .struct_name_to_fully_annotated_layout(
                    *idx,
                    module_store,
                    module_storage,
                    ty_args,
                    count,
                    depth + 1,
                )?,
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type layout for {:?}", ty)),
                );
            },
        })
    }

    pub(crate) fn calculate_depth_of_struct(
        &self,
        struct_name_idx: StructNameIndex,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<DepthFormula> {
        let ty_cache = self.ty_cache(module_storage);
        if let Some(depth_formula) = ty_cache.get_depth_formula(&struct_name_idx) {
            return Ok(depth_formula);
        }

        let struct_type =
            self.fetch_struct_ty_by_idx(struct_name_idx, module_store, module_storage)?;
        let formulas = match &struct_type.layout {
            StructLayout::Single(fields) => fields
                .iter()
                .map(|(_, field_ty)| {
                    self.calculate_depth_of_type(field_ty, module_store, module_storage)
                })
                .collect::<PartialVMResult<Vec<_>>>()?,
            StructLayout::Variants(variants) => variants
                .iter()
                .flat_map(|variant| variant.1.iter().map(|(_, ty)| ty))
                .map(|field_ty| {
                    self.calculate_depth_of_type(field_ty, module_store, module_storage)
                })
                .collect::<PartialVMResult<Vec<_>>>()?,
        };

        let formula = DepthFormula::normalize(formulas);

        let struct_name_index_map = self.struct_name_index_map(module_storage);
        ty_cache.store_depth_formula(struct_name_idx, struct_name_index_map, &formula)?;
        Ok(formula)
    }

    fn calculate_depth_of_type(
        &self,
        ty: &Type,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
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
                let mut inner = self.calculate_depth_of_type(ty, module_store, module_storage)?;
                inner.scale(1);
                inner
            },
            Type::Reference(ty) | Type::MutableReference(ty) => {
                let mut inner = self.calculate_depth_of_type(ty, module_store, module_storage)?;
                inner.scale(1);
                inner
            },
            Type::TyParam(ty_idx) => DepthFormula::type_parameter(*ty_idx),
            Type::Struct { idx, .. } => {
                let mut struct_formula =
                    self.calculate_depth_of_struct(*idx, module_store, module_storage)?;
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
                        Ok((
                            var,
                            self.calculate_depth_of_type(ty, module_store, module_storage)?,
                        ))
                    })
                    .collect::<PartialVMResult<BTreeMap<_, _>>>()?;
                let struct_formula =
                    self.calculate_depth_of_struct(*idx, module_store, module_storage)?;
                let mut subst_struct_formula = struct_formula.subst(ty_arg_map)?;
                subst_struct_formula.scale(1);
                subst_struct_formula
            },
        })
    }

    pub(crate) fn type_to_type_tag(
        &self,
        ty: &Type,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<TypeTag> {
        match self {
            Loader::V1(loader) => {
                let mut gas_context = PseudoGasContext::new(self.vm_config());
                loader.type_to_type_tag_impl(ty, &mut gas_context)
            },
            Loader::V2(_) => {
                let ty_tag_builder = TypeTagBuilder::new(module_storage.runtime_environment());
                ty_tag_builder.ty_to_ty_tag(ty)
            },
        }
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let mut count = 0;
        self.type_to_type_layout_impl(ty, module_store, module_storage, &mut count, 1)
    }

    pub(crate) fn type_to_type_layout(
        &self,
        ty: &Type,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<MoveTypeLayout> {
        let mut count = 0;
        let (layout, _has_identifier_mappings) =
            self.type_to_type_layout_impl(ty, module_store, module_storage, &mut count, 1)?;
        Ok(layout)
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<MoveTypeLayout> {
        let mut count = 0;
        self.type_to_fully_annotated_layout_impl(ty, module_store, module_storage, &mut count, 1)
    }
}

// Public APIs for external uses.
impl Loader {
    pub(crate) fn get_type_layout(
        &self,
        type_tag: &TypeTag,
        move_storage: &mut TransactionDataCache,
        module_storage_adapter: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<MoveTypeLayout> {
        let ty = self.load_type(
            type_tag,
            move_storage,
            module_storage_adapter,
            module_storage,
        )?;
        self.type_to_type_layout(&ty, module_storage_adapter, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub(crate) fn get_fully_annotated_type_layout(
        &self,
        type_tag: &TypeTag,
        move_storage: &mut TransactionDataCache,
        module_storage_adapter: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<MoveTypeLayout> {
        let ty = self.load_type(
            type_tag,
            move_storage,
            module_storage_adapter,
            module_storage,
        )?;
        self.type_to_fully_annotated_layout(&ty, module_storage_adapter, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }
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
            match_return_type(ret_inner, expected_inner, map)
        },
        (Type::Vector(ret_inner), Type::Vector(expected_inner)) => {
            match_return_type(ret_inner, expected_inner, map)
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
                    .all(|types| match_return_type(types.0, types.1, map))
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

pub(crate) fn check_natives(module: &CompiledModule) -> VMResult<()> {
    // TODO: fix check and error code if we leave something around for native structs.
    // For now this generates the only error test cases care about...
    for (idx, struct_def) in module.struct_defs().iter().enumerate() {
        if struct_def.field_information == StructFieldInformation::Native {
            return Err(verification_error(
                StatusCode::MISSING_DEPENDENCY,
                IndexKind::FunctionHandle,
                idx as TableIndex,
            )
            .finish(Location::Module(module.self_id())));
        }
    }
    Ok(())
}
