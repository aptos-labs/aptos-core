// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    data_cache::TransactionDataCache,
    interpreter::Interpreter,
    loader::{
        LegacyModuleCache, LegacyModuleStorage, LegacyModuleStorageAdapter, LoadedFunction, Loader,
    },
    module_traversal::TraversalContext,
    native_extensions::NativeContextExtensions,
    session::SerializedReturnValues,
    storage::{
        code_storage::CodeStorage, module_storage::ModuleStorage,
        ty_layout_converter::LoaderLayoutConverter,
    },
    AsFunctionValueExtension, LayoutConverter, RuntimeEnvironment,
};
use move_binary_format::{
    access::ModuleAccess,
    compatibility::Compatibility,
    errors::{verification_error, Location, PartialVMError, PartialVMResult, VMResult},
    file_format::LocalIndex,
    normalized, CompiledModule, IndexKind,
};
use move_core_types::{
    account_address::AccountAddress, language_storage::TypeTag, value::MoveTypeLayout,
    vm_status::StatusCode,
};
use move_vm_metrics::{Timer, VM_TIMER};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::Type,
    value_serde::ValueSerDeContext,
    values::{Locals, Reference, VMValueCast, Value},
};
use std::{borrow::Borrow, collections::BTreeSet, sync::Arc};

/// An instantiation of the MoveVM.
pub(crate) struct VMRuntime {
    loader: Loader,
    pub(crate) module_cache: Arc<LegacyModuleCache>,
}

impl Clone for VMRuntime {
    fn clone(&self) -> Self {
        Self {
            loader: self.loader.clone(),
            module_cache: Arc::new(LegacyModuleCache::clone(&self.module_cache)),
        }
    }
}

impl VMRuntime {
    /// Creates a new runtime instance with provided environment.
    pub(crate) fn new(runtime_environment: &RuntimeEnvironment) -> Self {
        let vm_config = runtime_environment.vm_config().clone();
        let loader = if vm_config.use_loader_v2 {
            Loader::v2(vm_config)
        } else {
            let natives = runtime_environment.natives().clone();
            Loader::v1(natives, vm_config)
        };

        VMRuntime {
            loader,
            // TODO(loader_v2):
            //   We still create this cache, but if V2 loader is used, it is not used. We will
            //   remove it in the future together with other V1 components.
            module_cache: Arc::new(LegacyModuleCache::new()),
        }
    }

    #[deprecated]
    pub(crate) fn publish_module_bundle(
        &self,
        modules: Vec<Vec<u8>>,
        sender: AccountAddress,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        _gas_meter: &mut impl GasMeter,
        compat: Compatibility,
    ) -> VMResult<()> {
        // We must be using V1 loader flow here.
        let loader = match &self.loader {
            Loader::V1(loader) => loader,
            Loader::V2(_) => {
                let msg =
                    "Loader V2 cannot be used in V1 runtime::publish_module_bundle".to_string();
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(msg)
                        .finish(Location::Undefined),
                );
            },
        };

        // Deserialize the modules. Perform bounds check. After this indices can be
        // used with the `[]` operator
        let compiled_modules = match modules
            .iter()
            .map(|blob| {
                CompiledModule::deserialize_with_config(
                    blob,
                    &loader.vm_config().deserializer_config,
                )
            })
            .collect::<PartialVMResult<Vec<_>>>()
        {
            Ok(modules) => modules,
            Err(err) => {
                return Err(err
                    .append_message_with_separator(
                        '\n',
                        "[VM] module deserialization failed".to_string(),
                    )
                    .finish(Location::Undefined));
            },
        };

        // Make sure all modules' self addresses matches the transaction sender. The self address is
        // where the module will actually be published. If we did not check this, the sender could
        // publish a module under anyone's account.
        for module in &compiled_modules {
            if module.address() != &sender {
                return Err(verification_error(
                    StatusCode::MODULE_ADDRESS_DOES_NOT_MATCH_SENDER,
                    IndexKind::AddressIdentifier,
                    module.self_handle_idx().0,
                )
                .finish(Location::Undefined));
            }
        }

        // Collect ids for modules that are published together
        let mut bundle_unverified = BTreeSet::new();

        // For now, we assume that all modules can be republished, as long as the new module is
        // backward compatible with the old module.
        //
        // TODO: in the future, we may want to add restrictions on module republishing, possibly by
        // changing the bytecode format to include an `is_upgradable` flag in the CompiledModule.
        for module in &compiled_modules {
            let module_id = module.self_id();

            #[allow(deprecated)]
            if data_store.exists_module(&module_id)? && compat.need_check_compat() {
                let old_module_ref = loader.load_module(&module_id, data_store, module_store)?;
                let old_module = old_module_ref.as_ref().as_ref();
                if loader.vm_config().use_compatibility_checker_v2 {
                    compat
                        .check(old_module_ref.as_ref().as_ref(), module)
                        .map_err(|e| e.finish(Location::Undefined))?
                } else {
                    #[allow(deprecated)]
                    let old_m = normalized::Module::new(old_module)
                        .map_err(|e| e.finish(Location::Undefined))?;
                    #[allow(deprecated)]
                    let new_m = normalized::Module::new(module)
                        .map_err(|e| e.finish(Location::Undefined))?;
                    compat
                        .legacy_check(&old_m, &new_m)
                        .map_err(|e| e.finish(Location::Undefined))?
                }
            }
            if !bundle_unverified.insert(module_id) {
                return Err(PartialVMError::new(StatusCode::DUPLICATE_MODULE_NAME)
                    .finish(Location::Undefined));
            }
        }

        // Perform bytecode and loading verification. Modules must be sorted in topological order.
        loader.verify_module_bundle_for_publication(&compiled_modules, data_store, module_store)?;

        // NOTE: we want to (informally) argue that all modules pass the linking check before being
        // published to the data store.
        //
        // The linking check consists of two checks actually
        // - dependencies::verify_module(module, all_imm_deps)
        // - cyclic_dependencies::verify_module(module, fn_imm_deps, fn_imm_friends)
        //
        // [Claim 1]
        // We show that the `dependencies::verify_module` check is always satisfied whenever a
        // module M is published or updated and the `all_imm_deps` contains the actual modules
        // required by M.
        //
        // Suppose M depends on D, and we now consider the following scenarios:
        // 1) D does not appear in the bundle together with M
        // -- In this case, D must be either in the code cache or in the data store which can be
        //    loaded into the code cache (and pass all checks on D).
        //    - If D is missing, the linking will fail and return an error.
        //    - If D exists, D will be added to the `all_imm_deps` arg when checking M.
        //
        // 2) D appears in the bundle *before* M
        // -- In this case, regardless of whether D is in code cache or not, D will be put into the
        //    `bundle_verified` argument and modules in `bundle_verified` will be prioritized before
        //    returning a module in code cache.
        //
        // 3) D appears in the bundle *after* M
        // -- This technically should be discouraged but this is user input so we cannot have this
        //    assumption here. But nevertheless, we can still make the claim 1 even in this case.
        //    When M is verified, flow 1) is effectively activated, which means:
        //    - If the code cache or the data store does not contain a D' which has the same name
        //      with D, then the linking will fail and return an error.
        //    - If D' exists, and M links against D', then when verifying D in a later time point,
        //      a compatibility check will be invoked to ensure that D is compatible with D',
        //      meaning, whichever module that links against D' will have to link against D as well.
        //
        // [Claim 2]
        // We show that the `cyclic_dependencies::verify_module` check is always satisfied whenever
        // a module M is published or updated and the dep/friend modules returned by the transitive
        // dependency closure functions are valid.
        //
        // Currently, the code is written in a way that, from the view point of the
        // `cyclic_dependencies::verify_module` check, modules checked prior to module M in the same
        // bundle looks as if they have already been published and loaded to the code cache.
        //
        // Therefore, if M forms a cyclic dependency with module A in the same bundle that is
        // checked prior to M, such an error will be detected. However, if M forms a cyclic
        // dependency with a module X that appears in the same bundle *after* M. The cyclic
        // dependency can only be caught when X is verified.
        //
        // In summary: the code is written in a way that, certain checks are skipped while checking
        // each individual module in the bundle in order. But if every module in the bundle pass
        // all the checks, then the whole bundle can be published/upgraded together. Otherwise,
        // none of the module can be published/updated.

        // All modules verified, publish them to data cache
        for (module, blob) in compiled_modules.into_iter().zip(modules.into_iter()) {
            #[allow(deprecated)]
            let is_republishing = data_store.exists_module(&module.self_id())?;
            if is_republishing {
                // This is an upgrade, so invalidate the loader cache, which still contains the
                // old module.
                #[allow(deprecated)]
                self.loader.mark_v1_as_invalid();
            }
            #[allow(deprecated)]
            data_store.publish_module(&module.self_id(), blob, is_republishing)?;
        }
        Ok(())
    }

    fn deserialize_arg(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        ty: &Type,
        arg: impl Borrow<[u8]>,
    ) -> PartialVMResult<Value> {
        let (layout, has_identifier_mappings) =
            match LoaderLayoutConverter::new(&self.loader, module_store, module_storage)
                .type_to_type_layout_with_identifier_mappings(ty)
            {
                Ok(layout) => layout,
                Err(_err) => {
                    return Err(PartialVMError::new(
                        StatusCode::INVALID_PARAM_TYPE_FOR_DESERIALIZATION,
                    )
                    .with_message("[VM] failed to get layout from type".to_string()));
                },
            };

        let deserialization_error = || -> PartialVMError {
            PartialVMError::new(StatusCode::FAILED_TO_DESERIALIZE_ARGUMENT)
                .with_message("[VM] failed to deserialize argument".to_string())
        };

        // Make sure we do not construct values which might have identifiers
        // inside. This should be guaranteed by transaction argument validation
        // but because it does not use layouts we double-check here.
        if has_identifier_mappings {
            return Err(deserialization_error());
        }

        let function_value_extension = module_storage.as_function_value_extension();
        match ValueSerDeContext::new()
            .with_func_args_deserialization(&function_value_extension)
            .deserialize(arg.borrow(), &layout)
        {
            Some(val) => Ok(val),
            None => Err(deserialization_error()),
        }
    }

    fn deserialize_args(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        param_tys: Vec<Type>,
        serialized_args: Vec<impl Borrow<[u8]>>,
    ) -> PartialVMResult<(Locals, Vec<Value>)> {
        if param_tys.len() != serialized_args.len() {
            return Err(
                PartialVMError::new(StatusCode::NUMBER_OF_ARGUMENTS_MISMATCH).with_message(
                    format!(
                        "argument length mismatch: expected {} got {}",
                        param_tys.len(),
                        serialized_args.len()
                    ),
                ),
            );
        }

        // Create a list of dummy locals. Each value stored will be used be borrowed and passed
        // by reference to the invoked function
        let mut dummy_locals = Locals::new(param_tys.len());
        // Arguments for the invoked function. These can be owned values or references
        let deserialized_args = param_tys
            .into_iter()
            .zip(serialized_args)
            .enumerate()
            .map(|(idx, (ty, arg_bytes))| match &ty {
                Type::MutableReference(inner_t) | Type::Reference(inner_t) => {
                    dummy_locals.store_loc(
                        idx,
                        self.deserialize_arg(module_store, module_storage, inner_t, arg_bytes)?,
                        self.loader.vm_config().check_invariant_in_swap_loc,
                    )?;
                    dummy_locals.borrow_loc(idx)
                },
                _ => self.deserialize_arg(module_store, module_storage, &ty, arg_bytes),
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok((dummy_locals, deserialized_args))
    }

    fn serialize_return_value(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        ty: &Type,
        value: Value,
    ) -> PartialVMResult<(Vec<u8>, MoveTypeLayout)> {
        let (ty, value) = match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => {
                let ref_value: Reference = value.cast()?;
                let inner_value = ref_value.read_ref()?;
                (&**inner, inner_value)
            },
            _ => (ty, value),
        };

        let (layout, has_identifier_mappings) =
            LoaderLayoutConverter::new(&self.loader, module_store, module_storage)
                .type_to_type_layout_with_identifier_mappings(ty)
                .map_err(|_err| {
                    // TODO: Should we use `err` instead of mapping?
                    PartialVMError::new(StatusCode::VERIFICATION_ERROR).with_message(
                        "entry point functions cannot have non-serializable return types"
                            .to_string(),
                    )
                })?;

        let serialization_error = || -> PartialVMError {
            PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                .with_message("failed to serialize return values".to_string())
        };

        // Disallow native values to escape through return values of a function.
        if has_identifier_mappings {
            return Err(serialization_error());
        }

        let function_value_extension = module_storage.as_function_value_extension();
        let bytes = ValueSerDeContext::new()
            .with_func_args_deserialization(&function_value_extension)
            .serialize(&value, &layout)?
            .ok_or_else(serialization_error)?;
        Ok((bytes, layout))
    }

    fn serialize_return_values(
        &self,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        return_types: &[Type],
        return_values: Vec<Value>,
    ) -> PartialVMResult<Vec<(Vec<u8>, MoveTypeLayout)>> {
        if return_types.len() != return_values.len() {
            return Err(
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(
                    format!(
                        "declared {} return types, but got {} return values",
                        return_types.len(),
                        return_values.len()
                    ),
                ),
            );
        }

        return_types
            .iter()
            .zip(return_values)
            .map(|(ty, value)| self.serialize_return_value(module_store, module_storage, ty, value))
            .collect()
    }

    fn execute_function_impl(
        &self,
        function: LoadedFunction,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        module_storage: &impl ModuleStorage,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
    ) -> VMResult<SerializedReturnValues> {
        let ty_builder = self.loader().ty_builder();
        let ty_args = function.ty_args();

        let param_tys = function
            .param_tys()
            .iter()
            .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Undefined))?;
        let mut_ref_args = param_tys
            .iter()
            .enumerate()
            .filter_map(|(idx, ty)| match ty {
                Type::MutableReference(inner) => Some((idx, inner.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (mut dummy_locals, deserialized_args) = self
            .deserialize_args(module_store, module_storage, param_tys, serialized_args)
            .map_err(|e| e.finish(Location::Undefined))?;
        let return_tys = function
            .return_tys()
            .iter()
            .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()
            .map_err(|err| err.finish(Location::Undefined))?;

        let timer = VM_TIMER.timer_with_label("Interpreter::entrypoint");
        let return_values = Interpreter::entrypoint(
            function,
            deserialized_args,
            data_store,
            module_store,
            module_storage,
            gas_meter,
            traversal_context,
            extensions,
            &self.loader,
        )?;
        drop(timer);

        let serialized_return_values = self
            .serialize_return_values(module_store, module_storage, &return_tys, return_values)
            .map_err(|e| e.finish(Location::Undefined))?;
        let serialized_mut_ref_outputs = mut_ref_args
            .into_iter()
            .map(|(idx, ty)| {
                // serialize return values first in the case that a value points into this local
                let local_val = dummy_locals
                    .move_loc(idx, self.loader.vm_config().check_invariant_in_swap_loc)?;
                let (bytes, layout) =
                    self.serialize_return_value(module_store, module_storage, &ty, local_val)?;
                Ok((idx as LocalIndex, bytes, layout))
            })
            .collect::<PartialVMResult<_>>()
            .map_err(|e| e.finish(Location::Undefined))?;

        // locals should not be dropped until all return values are serialized
        drop(dummy_locals);

        Ok(SerializedReturnValues {
            mutable_reference_outputs: serialized_mut_ref_outputs,
            return_values: serialized_return_values,
        })
    }

    pub(crate) fn execute_function_instantiation(
        &self,
        func: LoadedFunction,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<SerializedReturnValues> {
        self.execute_function_impl(
            func,
            serialized_args,
            data_store,
            module_store,
            module_storage,
            gas_meter,
            traversal_context,
            extensions,
        )
    }

    pub(crate) fn execute_script(
        &self,
        script: impl Borrow<[u8]>,
        ty_args: Vec<TypeTag>,
        serialized_args: Vec<impl Borrow<[u8]>>,
        data_store: &mut TransactionDataCache,
        module_store: &LegacyModuleStorageAdapter,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        extensions: &mut NativeContextExtensions,
        code_storage: &impl CodeStorage,
    ) -> VMResult<()> {
        // Load the script first, verify it, and then execute the entry-point main function.
        let main = self.loader.load_script(
            script.borrow(),
            &ty_args,
            data_store,
            module_store,
            code_storage,
        )?;

        self.execute_function_impl(
            main,
            serialized_args,
            data_store,
            module_store,
            code_storage,
            gas_meter,
            traversal_context,
            extensions,
        )?;
        Ok(())
    }

    pub(crate) fn loader(&self) -> &Loader {
        &self.loader
    }

    pub(crate) fn module_storage_v1(&self) -> Arc<dyn LegacyModuleStorage> {
        self.module_cache.clone() as Arc<dyn LegacyModuleStorage>
    }
}
