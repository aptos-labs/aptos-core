// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::VMConfig,
    frame_type_cache::FrameTypeCache,
    loader::{FunctionHandle, LoadedFunctionOwner, StructVariantInfo, VariantFieldInfo},
    module_traversal::TraversalContext,
    reentrancy_checker::CallType,
    runtime_type_checks::RuntimeTypeCheck,
    storage::loader::traits::FunctionDefinitionLoader,
    LoadedFunction,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Constant, ConstantPoolIndex, FieldHandleIndex, FieldInstantiationIndex,
        FunctionHandleIndex, FunctionInstantiationIndex, LocalIndex, SignatureIndex,
        StructDefInstantiationIndex, StructDefinitionIndex, StructVariantHandleIndex,
        StructVariantInstantiationIndex, VariantFieldHandleIndex, VariantFieldInstantiationIndex,
        VariantIndex,
    },
};
use move_core_types::{
    ability::Ability, account_address::AccountAddress, gas_algebra::NumTypeNodes,
    identifier::IdentStr, language_storage::ModuleId, vm_status::StatusCode,
};
use move_vm_types::{
    gas::GasMeter,
    loaded_data::{
        runtime_access_specifier::{AccessSpecifierEnv, AddressSpecifierFunction},
        runtime_types::{AbilityInfo, StructType, Type, TypeBuilder},
    },
    values::Locals,
};
use std::{cell::RefCell, rc::Rc, sync::Arc};

/// Represents the execution context for a function. When calls are made, frames are
/// pushed and then popped to/from the call stack.
pub(crate) struct Frame {
    pub(crate) pc: u16,
    ty_builder: TypeBuilder,
    // Currently being executed function.
    pub(crate) function: Rc<LoadedFunction>,
    // How this frame was established.
    call_type: CallType,
    // Locals for this execution context and their instantiated types.
    pub(crate) locals: Locals,
    pub(crate) local_tys: Vec<Type>,
    // Cache of types accessed in this frame, to improve performance when accessing
    // and constructing types.
    pub(crate) frame_cache: Rc<RefCell<FrameTypeCache>>,
}

impl AccessSpecifierEnv for Frame {
    fn eval_address_specifier_function(
        &self,
        fun: AddressSpecifierFunction,
        local: LocalIndex,
    ) -> PartialVMResult<AccountAddress> {
        fun.eval(self.locals.copy_loc(local as usize)?)
    }
}

macro_rules! build_loaded_function {
    ($function_name:ident, $idx_ty:ty, $get_function_handle:ident) => {
        pub(crate) fn $function_name(
            &self,
            loader: &impl FunctionDefinitionLoader,
            gas_meter: &mut impl GasMeter,
            traversal_context: &mut TraversalContext,
            idx: $idx_ty,
            verified_ty_args: Vec<Type>,
        ) -> PartialVMResult<LoadedFunction> {
            match self.function.owner() {
                LoadedFunctionOwner::Module(module) => {
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
                                loader,
                                gas_meter,
                                traversal_context,
                                module,
                                name,
                                verified_ty_args,
                            ),
                    }
                },
                LoadedFunctionOwner::Script(script) => {
                    let handle = script.$get_function_handle(idx.0);
                    match handle {
                        FunctionHandle::Local(_) => Err(PartialVMError::new(
                            StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR,
                        )
                        .with_message("Scripts never have local functions".to_string())),
                        FunctionHandle::Remote { module, name } => self
                            .build_loaded_function_from_name_and_ty_args(
                                loader,
                                gas_meter,
                                traversal_context,
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

impl Frame {
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

    #[inline(always)]
    pub(crate) fn make_new_frame<RTTCheck: RuntimeTypeCheck>(
        gas_meter: &mut impl GasMeter,
        call_type: CallType,
        vm_config: &VMConfig,
        function: Rc<LoadedFunction>,
        locals: Locals,
        frame_cache: Rc<RefCell<FrameTypeCache>>,
    ) -> PartialVMResult<Frame> {
        let ty_args = function.ty_args();
        for ty in function.local_tys() {
            gas_meter
                .charge_create_ty(NumTypeNodes::new(ty.num_nodes_in_subst(ty_args)? as u64))?;
        }

        let ty_builder = vm_config.ty_builder.clone();
        let local_tys = if RTTCheck::should_perform_checks(&function.function) {
            if ty_args.is_empty() {
                function.local_tys().to_vec()
            } else {
                function
                    .local_tys()
                    .iter()
                    .map(|ty| ty_builder.create_ty_with_subst(ty, ty_args))
                    .collect::<PartialVMResult<Vec<_>>>()?
            }
        } else {
            vec![]
        };

        Ok(Frame {
            pc: 0,
            ty_builder,
            locals,
            function,
            call_type,
            local_tys,
            frame_cache,
        })
    }

    pub(crate) fn ty_builder(&self) -> &TypeBuilder {
        &self.ty_builder
    }

    pub(crate) fn call_type(&self) -> CallType {
        self.call_type
    }

    #[inline(always)]
    pub(crate) fn local_ty_at(&self, idx: usize) -> &Type {
        &self.local_tys[idx]
    }

    #[inline(always)]
    pub(crate) fn check_local_tys_have_drop_ability(&self) -> PartialVMResult<()> {
        for (idx, ty) in self.local_tys.iter().enumerate() {
            if !self.locals.is_invalid(idx)? {
                ty.paranoid_check_has_ability(Ability::Drop)?;
            }
        }
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn untrusted_code(&self) -> bool {
        !self.function.function.is_trusted
    }

    #[inline(always)]
    pub(crate) fn constant_at(&self, idx: ConstantPoolIndex) -> &Constant {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.module.constant_at(idx),
            Script(script) => script.script.constant_at(idx),
        }
    }

    #[inline(always)]
    pub(crate) fn get_struct_ty(&self, idx: StructDefinitionIndex) -> Type {
        use LoadedFunctionOwner::*;
        let struct_ty = match self.function.owner() {
            Module(module) => module.struct_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        self.create_struct_ty(struct_ty)
    }

    pub(crate) fn get_struct_variant_at(
        &self,
        idx: StructVariantHandleIndex,
    ) -> &StructVariantInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.struct_variant_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_struct_variant_instantiation_at(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> &StructVariantInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.struct_variant_instantiation_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn get_generic_struct_ty(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_instantiation_at(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };

        let struct_ty = &struct_inst.definition_struct_type;
        self.ty_builder.create_struct_instantiation_ty(
            struct_ty,
            &struct_inst.instantiation,
            self.function.ty_args(),
        )
    }

    #[inline(always)]
    pub(crate) fn get_field_ty(&self, idx: FieldHandleIndex) -> PartialVMResult<&Type> {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => {
                let handle = &module.field_handles[idx.0 as usize];
                Ok(&handle.field_ty)
            },
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn get_generic_field_ty(
        &self,
        idx: FieldInstantiationIndex,
    ) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        let field_instantiation = match self.function.owner() {
            Module(module) => &module.field_instantiations[idx.0 as usize],
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let field_ty = &field_instantiation.uninstantiated_field_ty;
        self.instantiate_ty(field_ty, &field_instantiation.instantiation)
    }

    pub(crate) fn instantiate_ty(
        &self,
        ty: &Type,
        instantiation_tys: &[Type],
    ) -> PartialVMResult<Type> {
        let instantiation_tys = instantiation_tys
            .iter()
            .map(|inst_ty| {
                self.ty_builder
                    .create_ty_with_subst(inst_ty, self.function.ty_args())
            })
            .collect::<PartialVMResult<Vec<_>>>()?;
        self.ty_builder.create_ty_with_subst(ty, &instantiation_tys)
    }

    pub(crate) fn variant_field_info_at(&self, idx: VariantFieldHandleIndex) -> &VariantFieldInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.variant_field_info_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn variant_field_instantiation_info_at(
        &self,
        idx: VariantFieldInstantiationIndex,
    ) -> &VariantFieldInfo {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.variant_field_instantiation_info_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn get_struct(&self, idx: StructDefinitionIndex) -> &Arc<StructType> {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.struct_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    pub(crate) fn instantiate_generic_struct_fields(
        &self,
        idx: StructDefInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_instantiation_at(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(struct_ty, None, &struct_inst.instantiation)
    }

    pub(crate) fn instantiate_generic_struct_variant_fields(
        &self,
        idx: StructVariantInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let struct_inst = match self.function.owner() {
            Module(module) => module.struct_variant_instantiation_at(idx),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        };
        let struct_ty = &struct_inst.definition_struct_type;
        self.instantiate_generic_fields(
            struct_ty,
            Some(struct_inst.variant),
            &struct_inst.instantiation,
        )
    }

    pub(crate) fn instantiate_generic_fields(
        &self,
        struct_ty: &Arc<StructType>,
        variant: Option<VariantIndex>,
        instantiation: &[Type],
    ) -> PartialVMResult<Vec<Type>> {
        let instantiation_tys = instantiation
            .iter()
            .map(|inst_ty| {
                self.ty_builder
                    .create_ty_with_subst(inst_ty, self.function.ty_args())
            })
            .collect::<PartialVMResult<Vec<_>>>()?;

        struct_ty
            .fields(variant)?
            .iter()
            .map(|(_, inst_ty)| {
                self.ty_builder
                    .create_ty_with_subst(inst_ty, &instantiation_tys)
            })
            .collect::<PartialVMResult<Vec<_>>>()
    }

    fn single_type_at(&self, idx: SignatureIndex) -> &Type {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.single_type_at(idx),
            Script(script) => script.single_type_at(idx),
        }
    }

    pub(crate) fn instantiate_single_type(&self, idx: SignatureIndex) -> PartialVMResult<Type> {
        let ty = self.single_type_at(idx);
        let ty_args = self.function.ty_args();
        if !ty_args.is_empty() {
            self.ty_builder.create_ty_with_subst(ty, ty_args)
        } else {
            Ok(ty.clone())
        }
    }

    pub(crate) fn create_struct_ty(&self, struct_ty: &Arc<StructType>) -> Type {
        self.ty_builder
            .create_struct_ty(struct_ty.idx, AbilityInfo::struct_(struct_ty.abilities))
    }

    pub(crate) fn create_struct_instantiation_ty(
        &self,
        struct_ty: &Arc<StructType>,
        ty_params: &[Type],
    ) -> PartialVMResult<Type> {
        self.ty_builder.create_struct_instantiation_ty(
            struct_ty,
            ty_params,
            self.function.ty_args(),
        )
    }

    #[inline(always)]
    pub(crate) fn field_offset(&self, idx: FieldHandleIndex) -> usize {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.field_offset(idx),
            Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn field_instantiation_offset(&self, idx: FieldInstantiationIndex) -> usize {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.field_instantiation_offset(idx),
            Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn field_count(&self, idx: StructDefinitionIndex) -> u16 {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.field_count(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn field_instantiation_count(&self, idx: StructDefInstantiationIndex) -> u16 {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => module.field_instantiation_count(idx.0),
            Script(_) => unreachable!("Scripts cannot have type instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn field_handle_to_struct(&self, idx: FieldHandleIndex) -> Type {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => {
                let struct_ty = &module.field_handles[idx.0 as usize].definition_struct_type;
                self.create_struct_ty(struct_ty)
            },
            Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    #[inline(always)]
    pub(crate) fn field_instantiation_to_struct(
        &self,
        idx: FieldInstantiationIndex,
    ) -> PartialVMResult<Type> {
        use LoadedFunctionOwner::*;
        match self.function.owner() {
            Module(module) => {
                let field_inst = &module.field_instantiations[idx.0 as usize];
                self.create_struct_instantiation_ty(
                    &field_inst.definition_struct_type,
                    &field_inst.instantiation,
                )
            },
            Script(_) => unreachable!("Scripts cannot have field instructions"),
        }
    }

    pub(crate) fn instantiate_generic_function(
        &self,
        gas_meter: Option<&mut impl GasMeter>,
        idx: FunctionInstantiationIndex,
    ) -> PartialVMResult<Vec<Type>> {
        use LoadedFunctionOwner::*;
        let instantiation = match self.function.owner() {
            Module(module) => module.function_instantiation_at(idx.0),
            Script(script) => script.function_instantiation_at(idx.0),
        };

        let ty_args = self.function.ty_args();
        if let Some(gas_meter) = gas_meter {
            for ty in instantiation {
                gas_meter
                    .charge_create_ty(NumTypeNodes::new(ty.num_nodes_in_subst(ty_args)? as u64))?;
            }
        }

        let instantiation = instantiation
            .iter()
            .map(|ty| self.ty_builder.create_ty_with_subst(ty, ty_args))
            .collect::<PartialVMResult<Vec<_>>>()?;
        Ok(instantiation)
    }

    pub(crate) fn build_loaded_function_from_name_and_ty_args(
        &self,
        loader: &impl FunctionDefinitionLoader,
        gas_meter: &mut impl GasMeter,
        traversal_context: &mut TraversalContext,
        module_id: &ModuleId,
        function_name: &IdentStr,
        verified_ty_args: Vec<Type>,
    ) -> PartialVMResult<LoadedFunction> {
        let (module, function) = loader
            .load_function_definition(gas_meter, traversal_context, module_id, function_name)
            .map_err(|err| err.to_partial())?;
        Ok(LoadedFunction {
            owner: LoadedFunctionOwner::Module(module),
            ty_args: verified_ty_args,
            function,
        })
    }
}
