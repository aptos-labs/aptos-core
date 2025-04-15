// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    loader::modules::{StructVariantInfo, VariantFieldInfo},
    storage::module_storage::{FunctionValueExtensionAdapter, ModuleStorage},
    LayoutConverter, StorageLayoutConverter,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Constant, ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex,
        FunctionHandleIndex, FunctionInstantiationIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructVariantHandleIndex,
        StructVariantInstantiationIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
        VariantIndex,
    },
};
use move_core_types::{
    gas_algebra::NumTypeNodes, identifier::IdentStr, language_storage::ModuleId,
    value::MoveTypeLayout, vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::runtime_types::{AbilityInfo, StructType, Type, TypeBuilder},
    value_serde::FunctionValueExtension,
    values::{AbstractFunction, SerializedFunctionData},
};
use std::sync::Arc;
use type_loader::intern_type;

mod access_specifier_loader;
mod function;
pub use function::{Function, LoadedFunction, LoadedFunctionOwner};
pub(crate) use function::{
    FunctionHandle, FunctionInstantiation, LazyLoadedFunction, LazyLoadedFunctionState,
};
mod modules;
pub use modules::Module;
use move_vm_types::resolver::ResourceResolver;

mod script;
pub use script::Script;
mod type_loader;

// A simple wrapper for a `Module` or a `Script` in the `Resolver`
enum BinaryType {
    Module(Arc<Module>),
    Script(Arc<Script>),
}

// A Resolver is a simple and small structure allocated on the stack and used by the
// interpreter. It's the only API known to the interpreter and it's tailored to the interpreter
// needs.
pub(crate) struct Resolver<'a> {
    binary: BinaryType,
    module_storage: &'a dyn ModuleStorage,
    resource_resolver: &'a dyn ResourceResolver,
}

impl<'a> Resolver<'a> {
    fn for_module(
        module: Arc<Module>,
        module_storage: &'a dyn ModuleStorage,
        resource_resolver: &'a dyn ResourceResolver,
    ) -> Self {
        let binary = BinaryType::Module(module);
        Self {
            binary,
            module_storage,
            resource_resolver,
        }
    }

    fn for_script(
        script: Arc<Script>,
        module_storage: &'a dyn ModuleStorage,
        resource_resolver: &'a dyn ResourceResolver,
    ) -> Self {
        let binary = BinaryType::Script(script);
        Self {
            binary,
            module_storage,
            resource_resolver,
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
            .module_storage
            .fetch_function_definition(module_id.address(), module_id.name(), function_name)
            .map_err(|_| {
                // Note: legacy loader implementation used this error, so we need to remap.
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

        let ty_builder = self.ty_builder();
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
        let ty_builder = self.ty_builder();
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
        let ty_builder = self.ty_builder();
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
        let ty_builder = self.ty_builder();
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
            self.ty_builder().create_ty_with_subst(ty, ty_args)
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
                self.ty_builder()
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
        self.ty_builder()
            .create_struct_ty(struct_ty.idx, AbilityInfo::struct_(struct_ty.abilities))
    }

    pub(crate) fn create_struct_instantiation_ty(
        &self,
        struct_ty: &Arc<StructType>,
        ty_params: &[Type],
        ty_args: &[Type],
    ) -> PartialVMResult<Type> {
        self.ty_builder()
            .create_struct_instantiation_ty(struct_ty, ty_params, ty_args)
    }

    pub(crate) fn type_to_type_layout(&self, ty: &Type) -> PartialVMResult<MoveTypeLayout> {
        self.layout_converter().type_to_type_layout(ty)
    }

    pub(crate) fn type_to_type_layout_with_identifier_mappings(
        &self,
        ty: &Type,
    ) -> PartialVMResult<(MoveTypeLayout, bool)> {
        self.layout_converter()
            .type_to_type_layout_with_identifier_mappings(ty)
    }

    pub(crate) fn type_to_fully_annotated_layout(
        &self,
        ty: &Type,
    ) -> PartialVMResult<MoveTypeLayout> {
        self.layout_converter().type_to_fully_annotated_layout(ty)
    }

    pub(crate) fn layout_converter(&self) -> StorageLayoutConverter {
        StorageLayoutConverter::new(self.module_storage)
    }

    //
    // Getters
    //

    pub(crate) fn module_storage(&self) -> &dyn ModuleStorage {
        self.module_storage
    }

    pub(crate) fn resource_resolver(&self) -> &dyn ResourceResolver {
        self.resource_resolver
    }

    pub(crate) fn vm_config(&self) -> &VMConfig {
        self.module_storage.runtime_environment().vm_config()
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.vm_config().ty_builder
    }
}

impl<'a> FunctionValueExtension for Resolver<'a> {
    fn create_from_serialization_data(
        &self,
        data: SerializedFunctionData,
    ) -> PartialVMResult<Box<dyn AbstractFunction>> {
        let function_value_extension = FunctionValueExtensionAdapter {
            module_storage: self.module_storage,
        };
        function_value_extension.create_from_serialization_data(data)
    }

    fn get_serialization_data(
        &self,
        fun: &dyn AbstractFunction,
    ) -> PartialVMResult<SerializedFunctionData> {
        let function_value_extension = FunctionValueExtensionAdapter {
            module_storage: self.module_storage,
        };
        function_value_extension.get_serialization_data(fun)
    }
}
