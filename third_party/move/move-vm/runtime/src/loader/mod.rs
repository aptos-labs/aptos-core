// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig, module_traversal::TraversalContext, storage::module_storage::ModuleStorage,
    CodeStorage,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{verification_error, Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        CompiledModule, Constant, ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex,
        FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructFieldInformation, TableIndex,
        TypeParameterIndex,
    },
    IndexKind,
};
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::NumTypeNodes,
    ident_str,
    identifier::IdentStr,
    language_storage::{ModuleId, StructTag, TypeTag},
    value::{IdentifierMappingKind, MoveFieldLayout, MoveStructLayout, MoveTypeLayout},
    vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{
        AbilityInfo, DepthFormula, StructIdentifier, StructNameIndex, StructType, Type,
    },
    module_linker_error,
};
use std::{
    collections::{btree_map, BTreeMap},
    sync::Arc,
};
use typed_arena::Arena;

mod access_specifier_loader;
mod function;
mod modules;
mod script;
mod type_loader;

use crate::{
    loader::modules::{StructVariantInfo, VariantFieldInfo},
    logging::expect_no_verification_errors,
};
pub use function::LoadedFunction;
pub(crate) use function::{Function, FunctionHandle, FunctionInstantiation, LoadedFunctionOwner};
pub use modules::Module;
use move_binary_format::file_format::{
    StructVariantHandleIndex, StructVariantInstantiationIndex, VariantFieldHandleIndex,
    VariantFieldInstantiationIndex, VariantIndex,
};
use move_core_types::gas_algebra::NumBytes;
use move_vm_types::loaded_data::runtime_types::{StructLayout, TypeBuilder};
pub use script::Script;
use type_loader::intern_type;

/// Loader is responsible for fetching functions, scripts, types and modules from the storage.
pub(crate) struct Loader {
    /// VM configuration that this loader is using.
    vm_config: VMConfig,
}

impl Loader {
    /// Returns a new loader with the specified VM configuration.
    pub(crate) fn new(vm_config: VMConfig) -> Self {
        Self { vm_config }
    }

    /// Returns the [VMConfig] used by this loader.
    pub(crate) fn vm_config(&self) -> &VMConfig {
        &self.vm_config
    }

    /// Returns [TypeBuilder] used by this loader.
    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config.ty_builder
    }

    pub(crate) fn check_script_dependencies_and_check_gas(
        &self,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        serialized_script: &[u8],
        code_storage: &impl CodeStorage,
    ) -> VMResult<()> {
        let compiled_script = code_storage.deserialize_and_cache_script(serialized_script)?;
        let compiled_script = traversal_context.referenced_scripts.alloc(compiled_script);

        // TODO(Gas): Should we charge dependency gas for the script itself?
        self.check_dependencies_and_charge_gas(
            gas_meter,
            &mut traversal_context.visited,
            traversal_context.referenced_modules,
            compiled_script.immediate_dependencies_iter(),
            code_storage,
        )?;

        Ok(())
    }

    pub(crate) fn check_dependencies_and_charge_gas<'a, I>(
        &self,
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
        // Initialize the work list (stack) and the map of visited modules.
        //
        // TODO: Determine the reserved capacity based on the max number of dependencies allowed.
        let mut stack = Vec::with_capacity(512);
        push_next_ids_to_visit(&mut stack, visited, ids);

        while let Some((addr, name)) = stack.pop() {
            let size = module_storage
                .fetch_module_size_in_bytes(addr, name)?
                .ok_or_else(|| module_linker_error!(addr, name))?;
            gas_meter
                .charge_dependency(false, addr, name, NumBytes::new(size as u64))
                .map_err(|err| {
                    err.finish(Location::Module(ModuleId::new(*addr, name.to_owned())))
                })?;

            // Extend the lifetime of the module to the remainder of the function body
            // by storing it in an arena.
            //
            // This is needed because we need to store references derived from it in the
            // work list.
            let compiled_module = module_storage
                .fetch_deserialized_module(addr, name)?
                .ok_or_else(|| module_linker_error!(addr, name))?;
            let compiled_module = referenced_modules.alloc(compiled_module);

            // Explore all dependencies and friends that have been visited yet.
            let imm_deps_and_friends = compiled_module
                .immediate_dependencies_iter()
                .chain(compiled_module.immediate_friends_iter());
            push_next_ids_to_visit(&mut stack, visited, imm_deps_and_friends);
        }

        Ok(())
    }

    /// Returns a loaded & verified module corresponding to the specified name.
    pub(crate) fn load_module(
        &self,
        module_storage: &dyn ModuleStorage,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Arc<Module>> {
        module_storage
            .fetch_verified_module(address, module_name)
            .map_err(expect_no_verification_errors)?
            .ok_or_else(|| module_linker_error!(address, module_name))
    }

    /// Loads the script:
    ///   1. Fetches it from the cache (or deserializes and verifies it if it is not cached).
    ///   2. Verifies type arguments (modules that define the type arguments are also loaded).
    /// If both steps are successful, returns a [LoadedFunction] corresponding to the script's
    /// entrypoint.
    pub(crate) fn load_script(
        &self,
        serialized_script: &[u8],
        ty_tag_args: &[TypeTag],
        code_storage: &impl CodeStorage,
    ) -> VMResult<LoadedFunction> {
        // Step 1: Load script. During the loading process, if script has not been previously
        // cached, it will be verified.
        let script = code_storage.verify_and_cache_script(serialized_script)?;

        // Step 2: Load & verify types used as type arguments passed to this script. Note that
        // arguments for scripts are verified on the client side.
        let ty_args = ty_tag_args
            .iter()
            .map(|ty_tag| self.load_ty(ty_tag, code_storage))
            .collect::<VMResult<Vec<_>>>()
            // TODO(loader_v2):
            //   Loader V1 implementation returned undefined here, causing some tests to fail. We
            //   should probably map this to script.
            .map_err(|e| e.to_partial().finish(Location::Undefined))?;

        let main = script.entry_point();
        Type::verify_ty_arg_abilities(main.ty_param_abilities(), &ty_args)
            .map_err(|e| e.finish(Location::Script))?;

        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Script(script),
            ty_args,
            function: main,
        })
    }

    /// Returns the function definition corresponding to the specified name, as well as the module
    /// where this function is defined.
    pub(crate) fn load_function_without_ty_args(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        module_storage: &dyn ModuleStorage,
    ) -> VMResult<(Arc<Module>, Arc<Function>)> {
        let module = self.load_module(module_storage, module_id.address(), module_id.name())?;
        let function = module
            .function_map
            .get(function_name)
            .and_then(|idx| module.function_defs.get(*idx))
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE)
                    .with_message(format!(
                        "Function {}::{}::{} does not exist",
                        module_id.address(),
                        module_id.name(),
                        function_name
                    ))
                    .finish(Location::Undefined)
            })?
            .clone();
        Ok((module, function))
    }

    /// Returns a struct type corresponding to the specified name. The module containing the struct
    /// is loaded.
    pub(crate) fn load_struct_ty(
        &self,
        module_storage: &dyn ModuleStorage,
        address: &AccountAddress,
        module_name: &IdentStr,
        struct_name: &IdentStr,
    ) -> PartialVMResult<Arc<StructType>> {
        let module = self
            .load_module(module_storage, address, module_name)
            .map_err(|e| e.to_partial())?;
        Ok(module
            .struct_map
            .get(struct_name)
            .and_then(|idx| module.structs.get(*idx))
            .ok_or_else(|| {
                PartialVMError::new(StatusCode::TYPE_RESOLUTION_FAILURE).with_message(format!(
                    "Struct {}::{}::{} does not exist",
                    address, module_name, struct_name
                ))
            })?
            .definition_struct_type
            .clone())
    }

    /// Returns a runtime type corresponding to the specified type tag (file format type
    /// representation). In case struct types are transitively loaded, the module containing
    /// the struct definition is also loaded.
    pub(crate) fn load_ty(
        &self,
        ty_tag: &TypeTag,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<Type> {
        self.ty_builder().create_ty(ty_tag, |st| {
            self.load_struct_ty(
                module_storage,
                &st.address,
                st.module.as_ident_str(),
                st.name.as_ident_str(),
            )
            .map_err(|e| e.finish(Location::Undefined))
        })
    }

    /// Returns a struct type corresponding to the specified name associated with the passed index.
    /// The module containing the struct is loaded.
    pub fn load_struct_ty_by_idx(
        &self,
        idx: StructNameIndex,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<Arc<StructType>> {
        // Ensure we do not return a guard here (still holding the lock) because loading the struct
        // type below can fetch struct type by index recursively.
        let struct_name = module_storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_name_ref(idx)?;

        self.load_struct_ty(
            module_storage,
            struct_name.module.address(),
            struct_name.module.name(),
            struct_name.name.as_ident_str(),
        )
    }

    // Loading verifies the module if it was never loaded.
    // Type parameters are inferred from the expected return type. Returns an error if it's not
    // possible to infer the type parameters or return type cannot be matched.
    // The type parameters are verified with capabilities.
    pub(crate) fn load_function_with_ty_arg_inference(
        &self,
        module_id: &ModuleId,
        function_name: &IdentStr,
        expected_return_type: &Type,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<LoadedFunction> {
        let (module, function) =
            self.load_function_without_ty_args(module_id, function_name, module_storage)?;

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
        module_storage: &impl ModuleStorage,
    ) -> VMResult<LoadedFunction> {
        let (module, function) =
            self.load_function_without_ty_args(module_id, function_name, module_storage)?;

        let ty_args = ty_args
            .iter()
            .map(|ty_arg| self.load_ty(ty_arg, module_storage))
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
    binary: BinaryType,
    module_storage: &'a dyn ModuleStorage,
}

impl<'a> Resolver<'a> {
    fn for_module(
        loader: &'a Loader,
        module_storage: &'a dyn ModuleStorage,
        module: Arc<Module>,
    ) -> Self {
        let binary = BinaryType::Module(module);
        Self {
            loader,
            binary,
            module_storage,
        }
    }

    fn for_script(
        loader: &'a Loader,
        module_storage: &'a dyn ModuleStorage,
        script: Arc<Script>,
    ) -> Self {
        let binary = BinaryType::Script(script);
        Self {
            loader,
            binary,
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
        let (module, function) = self
            .loader()
            .load_function_without_ty_args(module_id, function_name, self.module_storage)
            .map_err(|_| {
                // TODO(loader_v2):
                //   Loader V1 implementation uses this error, but it should be a linker error.
                PartialVMError::new(StatusCode::FUNCTION_RESOLUTION_FAILURE).with_message(format!(
                    "Module or function do not exist for {}::{}::{}",
                    module_id.address(),
                    module_id.name(),
                    function_name
                ))
            })?;
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
        self.loader.type_to_type_layout(ty, self.module_storage)
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.loader
            .type_to_type_layout_with_identifier_mappings(ty, self.module_storage)
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
    ) -> PartialVMResult<MoveTypeLayout> {
        self.loader
            .type_to_fully_annotated_layout(ty, self.module_storage)
    }

    //
    // Getters
    //

    pub(crate) fn loader(&self) -> &Loader {
        self.loader
    }

    pub(crate) fn module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        self.loader().vm_config()
    }
}

/// Maximal depth of a value in terms of type depth.
pub const VALUE_DEPTH_MAX: u64 = 128;

/// Maximal nodes which are allowed when converting to layout. This includes the types of
/// fields for struct types.
const MAX_TYPE_TO_LAYOUT_NODES: u64 = 256;

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

impl Loader {
    fn struct_name_to_type_tag(
        &self,
        struct_name_idx: StructNameIndex,
        ty_args: &[Type],
        gas_context: &mut PseudoGasContext,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<StructTag> {
        let ty_cache = module_storage.runtime_environment().ty_cache();
        if let Some((struct_tag, gas)) = ty_cache.get_struct_tag(&struct_name_idx, ty_args) {
            gas_context.charge(gas)?;
            return Ok(struct_tag.clone());
        }

        let cur_cost = gas_context.cost;

        let type_args = ty_args
            .iter()
            .map(|ty| self.type_to_type_tag_impl(ty, gas_context, module_storage))
            .collect::<PartialVMResult<Vec<_>>>()?;

        let struct_tag = module_storage
            .runtime_environment()
            .struct_name_index_map()
            .idx_to_struct_tag(struct_name_idx, type_args)?;

        let size =
            (struct_tag.address.len() + struct_tag.module.len() + struct_tag.name.len()) as u64;
        gas_context.charge(size * gas_context.cost_per_byte)?;
        ty_cache.store_struct_tag(
            struct_name_idx,
            ty_args.to_vec(),
            struct_tag.clone(),
            gas_context.cost - cur_cost,
        );
        Ok(struct_tag)
    }

    fn type_to_type_tag_impl(
        &self,
        ty: &Type,
        gas_context: &mut PseudoGasContext,
        module_storage: &dyn ModuleStorage,
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
            Type::Vector(ty) => {
                let el_ty_tag = self.type_to_type_tag_impl(ty, gas_context, module_storage)?;
                TypeTag::Vector(Box::new(el_ty_tag))
            },
            Type::Struct { idx, .. } => TypeTag::Struct(Box::new(self.struct_name_to_type_tag(
                *idx,
                &[],
                gas_context,
                module_storage,
            )?)),
            Type::StructInstantiation { idx, ty_args, .. } => TypeTag::Struct(Box::new(
                self.struct_name_to_type_tag(*idx, ty_args, gas_context, module_storage)?,
            )),
            Type::Reference(_) | Type::MutableReference(_) | Type::TyParam(_) => {
                return Err(
                    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
                        .with_message(format!("No type tag for {:?}", ty)),
                );
            },
        })
    }

    fn struct_name_to_type_layout(
        &self,
        struct_name_idx: StructNameIndex,
        module_storage: &dyn ModuleStorage,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let ty_cache = module_storage.runtime_environment().ty_cache();
        if let Some((struct_layout, node_count, has_identifier_mappings)) =
            ty_cache.get_struct_layout_info(&struct_name_idx, ty_args)
        {
            *count += node_count;
            return Ok((struct_layout, has_identifier_mappings));
        }

        let count_before = *count;
        let struct_type = self.load_struct_ty_by_idx(struct_name_idx, module_storage)?;

        let mut has_identifier_mappings = false;

        let layout = match &struct_type.layout {
            StructLayout::Single(fields) => {
                // Some types can have fields which are lifted at serialization or deserialization
                // times. Right now these are Aggregator and AggregatorSnapshot.
                let struct_name = module_storage
                    .runtime_environment()
                    .struct_name_index_map()
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
                    .map(|ty| self.type_to_type_layout_impl(ty, module_storage, count, depth))
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
                let (layout, has_identifier_mappings) =
                    self.type_to_type_layout_impl(ty, module_storage, count, depth + 1)?;
                (
                    MoveTypeLayout::Vector(Box::new(layout)),
                    has_identifier_mappings,
                )
            },
            Type::Struct { idx, .. } => {
                *count += 1;
                let (layout, has_identifier_mappings) =
                    self.struct_name_to_type_layout(*idx, module_storage, &[], count, depth + 1)?;
                (layout, has_identifier_mappings)
            },
            Type::StructInstantiation { idx, ty_args, .. } => {
                *count += 1;
                let (layout, has_identifier_mappings) = self.struct_name_to_type_layout(
                    *idx,
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
        module_storage: &dyn ModuleStorage,
        ty_args: &[Type],
        count: &mut u64,
        depth: u64,
    ) -> PartialVMResult<MoveTypeLayout> {
        let ty_cache = module_storage.runtime_environment().ty_cache();
        if let Some((layout, annotated_node_count)) =
            ty_cache.get_annotated_struct_layout_info(&struct_name_idx, ty_args)
        {
            *count += annotated_node_count;
            return Ok(layout);
        }

        let struct_type = self.load_struct_ty_by_idx(struct_name_idx, module_storage)?;

        // TODO(#13806): have annotated layouts for variants. Currently, we just return the raw
        //   layout for them.
        if matches!(struct_type.layout, StructLayout::Variants(_)) {
            return self
                .struct_name_to_type_layout(struct_name_idx, module_storage, ty_args, count, depth)
                .map(|(l, _)| l);
        }

        let count_before = *count;
        let mut gas_context = PseudoGasContext {
            cost: 0,
            max_cost: self.vm_config().type_max_cost,
            cost_base: self.vm_config().type_base_cost,
            cost_per_byte: self.vm_config().type_byte_cost,
        };
        let struct_tag = self.struct_name_to_type_tag(
            struct_name_idx,
            ty_args,
            &mut gas_context,
            module_storage,
        )?;
        let fields = struct_type.fields(None)?;

        let field_layouts = fields
            .iter()
            .map(|(n, ty)| {
                let ty = self.ty_builder().create_ty_with_subst(ty, ty_args)?;
                let l =
                    self.type_to_fully_annotated_layout_impl(&ty, module_storage, count, depth)?;
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
            Type::Vector(ty) => MoveTypeLayout::Vector(Box::new(
                self.type_to_fully_annotated_layout_impl(ty, module_storage, count, depth + 1)?,
            )),
            Type::Struct { idx, .. } => self.struct_name_to_fully_annotated_layout(
                *idx,
                module_storage,
                &[],
                count,
                depth + 1,
            )?,
            Type::StructInstantiation { idx, ty_args, .. } => self
                .struct_name_to_fully_annotated_layout(
                    *idx,
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
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<DepthFormula> {
        let ty_cache = module_storage.runtime_environment().ty_cache();
        if let Some(depth_formula) = ty_cache.get_depth_formula(&struct_name_idx) {
            return Ok(depth_formula);
        }

        let struct_type = self.load_struct_ty_by_idx(struct_name_idx, module_storage)?;
        let formulas = match &struct_type.layout {
            StructLayout::Single(fields) => fields
                .iter()
                .map(|(_, field_ty)| self.calculate_depth_of_type(field_ty, module_storage))
                .collect::<PartialVMResult<Vec<_>>>()?,
            StructLayout::Variants(variants) => variants
                .iter()
                .flat_map(|variant| variant.1.iter().map(|(_, ty)| ty))
                .map(|field_ty| self.calculate_depth_of_type(field_ty, module_storage))
                .collect::<PartialVMResult<Vec<_>>>()?,
        };

        let formula = DepthFormula::normalize(formulas);

        let struct_name_index_map = module_storage.runtime_environment().struct_name_index_map();
        ty_cache.store_depth_formula(struct_name_idx, struct_name_index_map, &formula)?;
        Ok(formula)
    }

    fn calculate_depth_of_type(
        &self,
        ty: &Type,
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
                let mut inner = self.calculate_depth_of_type(ty, module_storage)?;
                inner.scale(1);
                inner
            },
            Type::Reference(ty) | Type::MutableReference(ty) => {
                let mut inner = self.calculate_depth_of_type(ty, module_storage)?;
                inner.scale(1);
                inner
            },
            Type::TyParam(ty_idx) => DepthFormula::type_parameter(*ty_idx),
            Type::Struct { idx, .. } => {
                let mut struct_formula = self.calculate_depth_of_struct(*idx, module_storage)?;
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
                        Ok((var, self.calculate_depth_of_type(ty, module_storage)?))
                    })
                    .collect::<PartialVMResult<BTreeMap<_, _>>>()?;
                let struct_formula = self.calculate_depth_of_struct(*idx, module_storage)?;
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
        let mut gas_context = PseudoGasContext {
            cost: 0,
            max_cost: self.vm_config().type_max_cost,
            cost_base: self.vm_config().type_base_cost,
            cost_per_byte: self.vm_config().type_byte_cost,
        };
        self.type_to_type_tag_impl(ty, &mut gas_context, module_storage)
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        let mut count = 0;
        self.type_to_type_layout_impl(ty, module_storage, &mut count, 1)
    }

    pub(crate) fn type_to_type_layout(
        &self,
        ty: &Type,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<MoveTypeLayout> {
        let mut count = 0;
        let (layout, _has_identifier_mappings) =
            self.type_to_type_layout_impl(ty, module_storage, &mut count, 1)?;
        Ok(layout)
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
        module_storage: &dyn ModuleStorage,
    ) -> PartialVMResult<MoveTypeLayout> {
        let mut count = 0;
        self.type_to_fully_annotated_layout_impl(ty, module_storage, &mut count, 1)
    }
}

// Public APIs for external uses.
impl Loader {
    pub(crate) fn get_type_layout(
        &self,
        type_tag: &TypeTag,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<MoveTypeLayout> {
        let ty = self.load_ty(type_tag, module_storage)?;
        self.type_to_type_layout(&ty, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }

    pub(crate) fn get_fully_annotated_type_layout(
        &self,
        type_tag: &TypeTag,
        module_storage: &impl ModuleStorage,
    ) -> VMResult<MoveTypeLayout> {
        let ty = self.load_ty(type_tag, module_storage)?;
        self.type_to_fully_annotated_layout(&ty, module_storage)
            .map_err(|e| e.finish(Location::Undefined))
    }
}

/// Given a list of addresses and module names, pushes them onto stack unless they have been
/// already visited or if the address is special.
#[inline]
pub(crate) fn push_next_ids_to_visit<'a, I>(
    stack: &mut Vec<(&'a AccountAddress, &'a IdentStr)>,
    visited: &mut BTreeMap<(&'a AccountAddress, &'a IdentStr), ()>,
    ids: I,
) where
    I: IntoIterator<Item = (&'a AccountAddress, &'a IdentStr)>,
    I::IntoIter: DoubleEndedIterator,
{
    for (addr, name) in ids.into_iter().rev() {
        // TODO: Allow the check of special addresses to be customized.
        if !addr.is_special() && visited.insert((addr, name), ()).is_none() {
            stack.push((addr, name));
        }
    }
}

/// Matches the actual returned type to the expected type, binding any type args to the
/// necessary type as stored in the map. The expected type must be a concrete type (no type
/// parameters). Returns true if a successful match is made.
fn match_return_type<'a>(
    returned: &Type,
    expected: &'a Type,
    map: &mut BTreeMap<u16, &'a Type>,
) -> bool {
    match (returned, expected) {
        // The important case, deduce the type params.
        (Type::TyParam(idx), _) => match map.entry(*idx) {
            btree_map::Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(expected);
                true
            },
            btree_map::Entry::Occupied(occupied_entry) => *occupied_entry.get() == expected,
        },
        // Recursive types we need to recurse the matching types.
        (Type::Reference(ret_inner), Type::Reference(expected_inner))
        | (Type::MutableReference(ret_inner), Type::MutableReference(expected_inner)) => {
            match_return_type(ret_inner, expected_inner, map)
        },
        (Type::Vector(ret_inner), Type::Vector(expected_inner)) => {
            match_return_type(ret_inner, expected_inner, map)
        },
        // Abilities should not contribute to the equality check as they just serve for caching
        // computations. For structs the both need to be the same struct.
        (
            Type::Struct { idx: ret_idx, .. },
            Type::Struct {
                idx: expected_idx, ..
            },
        ) => *ret_idx == *expected_idx,
        // For struct instantiations we need to additionally match all type arguments.
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
        // For primitive types we need to assure the types match.
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
        // Note we don't use the _ pattern but spell out all cases, so that the compiler will bark
        // when a case is missed upon future updates to the types.
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
