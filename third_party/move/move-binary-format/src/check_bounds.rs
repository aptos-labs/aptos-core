// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    binary_views::BinaryIndexedView,
    errors::{
        bounds_error, offset_out_of_bounds as offset_out_of_bounds_error, verification_error,
        PartialVMError, PartialVMResult,
    },
    file_format::{
        Bytecode, CodeOffset, CodeUnit, CompiledModule, CompiledScript, Constant, FieldDefinition,
        FieldHandle, FieldInstantiation, FunctionDefinition, FunctionDefinitionIndex,
        FunctionHandle, FunctionInstantiation, LocalIndex, ModuleHandle, Signature, SignatureIndex,
        SignatureToken, StructDefInstantiation, StructDefinition, StructFieldInformation,
        StructHandle, StructVariantHandle, StructVariantInstantiation, TableIndex,
        TypeParameterIndex, VariantFieldHandle, VariantFieldInstantiation, VariantIndex,
    },
    internals::ModuleIndex,
    IndexKind,
};
use move_core_types::{ability::AbilitySet, vm_status::StatusCode};
use std::{
    cell::RefCell,
    collections::{btree_map, BTreeMap},
};

enum BoundsCheckingContext {
    Module,
    ModuleFunction(FunctionDefinitionIndex),
    Script,
}
pub struct BoundsChecker<'a> {
    view: BinaryIndexedView<'a>,

    context: BoundsCheckingContext,

    // Using the interior mutability pattern to make the borrow checker happy.
    max_ty_param_ids: RefCell<BTreeMap<SignatureIndex, Option<TypeParameterIndex>>>,
}

impl<'a> BoundsChecker<'a> {
    pub fn verify_script(script: &'a CompiledScript) -> PartialVMResult<()> {
        let mut bounds_check = Self {
            view: BinaryIndexedView::Script(script),
            context: BoundsCheckingContext::Script,

            max_ty_param_ids: RefCell::new(BTreeMap::new()),
        };
        bounds_check.verify_impl()?;

        let type_param_count = script.type_parameters.len();

        check_bounds_impl(bounds_check.view.signatures(), script.parameters)?;
        bounds_check.check_type_parameters_in_signature(script.parameters, type_param_count)?;

        // The bounds checker has already checked each function definition's code, but a
        // script's code exists outside of any function definition. It gets checked here.
        bounds_check.check_code(
            &script.code,
            &script.type_parameters,
            bounds_check
                .view
                .signatures()
                .get(script.parameters.into_index())
                .unwrap(),
            CompiledScript::MAIN_INDEX.into_index(),
        )
    }

    pub fn verify_module(module: &'a CompiledModule) -> PartialVMResult<()> {
        let mut bounds_check = Self {
            view: BinaryIndexedView::Module(module),
            context: BoundsCheckingContext::Module,

            max_ty_param_ids: RefCell::new(BTreeMap::new()),
        };
        if bounds_check.view.module_handles().is_empty() {
            let status =
                verification_error(StatusCode::NO_MODULE_HANDLES, IndexKind::ModuleHandle, 0);
            return Err(status);
        }
        bounds_check.verify_impl()
    }

    fn verify_impl(&mut self) -> PartialVMResult<()> {
        self.check_signatures()?;
        self.check_constants()?;
        self.check_module_handles()?;
        self.check_self_module_handle()?;
        self.check_struct_handles()?;
        self.check_function_handles()?;
        self.check_field_handles()?;
        self.check_friend_decls()?;
        self.check_struct_instantiations()?;
        self.check_function_instantiations()?;
        self.check_field_instantiations()?;
        self.check_struct_defs()?;
        self.check_table(
            self.view.variant_field_handles(),
            Self::check_variant_field_handle,
        )?;
        self.check_table(
            self.view.struct_variant_handles(),
            Self::check_struct_variant_handle,
        )?;
        self.check_table(
            self.view.variant_field_instantiations(),
            Self::check_variant_field_instantiation,
        )?;
        self.check_table(
            self.view.struct_variant_instantiations(),
            Self::check_struct_variant_instantiation,
        )?;
        self.check_function_defs()?;
        Ok(())
    }

    fn check_table<T>(
        &self,
        table: Option<&[T]>,
        checker: impl Fn(&Self, &T) -> PartialVMResult<()>,
    ) -> PartialVMResult<()> {
        if let Some(table) = table {
            for elem in table {
                checker(self, elem)?
            }
        }
        Ok(())
    }

    fn check_signatures(&self) -> PartialVMResult<()> {
        for signature in self.view.signatures() {
            self.check_signature(signature)?
        }
        Ok(())
    }

    fn check_constants(&self) -> PartialVMResult<()> {
        for constant in self.view.constant_pool() {
            self.check_constant(constant)?
        }
        Ok(())
    }

    fn check_module_handles(&self) -> PartialVMResult<()> {
        for script_handle in self.view.module_handles() {
            self.check_module_handle(script_handle)?
        }
        Ok(())
    }

    fn check_struct_handles(&self) -> PartialVMResult<()> {
        for struct_handle in self.view.struct_handles() {
            self.check_struct_handle(struct_handle)?
        }
        Ok(())
    }

    fn check_function_handles(&self) -> PartialVMResult<()> {
        for function_handle in self.view.function_handles() {
            self.check_function_handle(function_handle)?
        }
        Ok(())
    }

    fn check_field_handles(&self) -> PartialVMResult<()> {
        for field_handle in self.view.field_handles().into_iter().flatten() {
            self.check_field_handle(field_handle)?
        }
        Ok(())
    }

    fn check_friend_decls(&self) -> PartialVMResult<()> {
        for friend_decl in self.view.friend_decls().into_iter().flatten() {
            self.check_module_handle(friend_decl)?
        }
        Ok(())
    }

    fn check_struct_instantiations(&self) -> PartialVMResult<()> {
        for struct_instantiation in self.view.struct_instantiations().into_iter().flatten() {
            self.check_struct_instantiation(struct_instantiation)?
        }
        Ok(())
    }

    fn check_function_instantiations(&self) -> PartialVMResult<()> {
        for function_instantiation in self.view.function_instantiations() {
            self.check_function_instantiation(function_instantiation)?
        }
        Ok(())
    }

    fn check_field_instantiations(&self) -> PartialVMResult<()> {
        for field_instantiation in self.view.field_instantiations().into_iter().flatten() {
            self.check_field_instantiation(field_instantiation)?
        }
        Ok(())
    }

    fn check_struct_defs(&self) -> PartialVMResult<()> {
        for (struct_def_idx, struct_def) in
            self.view.struct_defs().into_iter().flatten().enumerate()
        {
            self.check_struct_def(struct_def, struct_def_idx)?
        }
        Ok(())
    }

    fn check_function_defs(&mut self) -> PartialVMResult<()> {
        let view = self.view;
        for (function_def_idx, function_def) in
            view.function_defs().into_iter().flatten().enumerate()
        {
            self.check_function_def(function_def_idx, function_def)?
        }
        Ok(())
    }

    fn check_module_handle(&self, module_handle: &ModuleHandle) -> PartialVMResult<()> {
        check_bounds_impl(self.view.address_identifiers(), module_handle.address)?;
        check_bounds_impl(self.view.identifiers(), module_handle.name)
    }

    fn check_self_module_handle(&self) -> PartialVMResult<()> {
        match self.view.self_handle_idx() {
            Some(idx) => check_bounds_impl(self.view.module_handles(), idx),
            None => Ok(()),
        }
    }

    fn check_struct_handle(&self, struct_handle: &StructHandle) -> PartialVMResult<()> {
        check_bounds_impl(self.view.module_handles(), struct_handle.module)?;
        check_bounds_impl(self.view.identifiers(), struct_handle.name)
    }

    fn check_function_handle(&self, function_handle: &FunctionHandle) -> PartialVMResult<()> {
        check_bounds_impl(self.view.module_handles(), function_handle.module)?;
        check_bounds_impl(self.view.identifiers(), function_handle.name)?;
        check_bounds_impl(self.view.signatures(), function_handle.parameters)?;
        check_bounds_impl(self.view.signatures(), function_handle.return_)?;
        // function signature type paramters must be in bounds to the function type parameters
        let type_param_count = function_handle.type_parameters.len();
        self.check_type_parameters_in_signature(function_handle.parameters, type_param_count)?;
        self.check_type_parameters_in_signature(function_handle.return_, type_param_count)?;
        Ok(())
    }

    fn check_field_handle(&self, field_handle: &FieldHandle) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.struct_defs(), field_handle.owner)?;
        // field offset must be in bounds, struct def just checked above must exist
        if let Some(struct_def) = &self
            .view
            .struct_defs()
            .and_then(|d| d.get(field_handle.owner.into_index()))
        {
            let fields_count = struct_def.field_information.field_count(None);
            if field_handle.field as usize >= fields_count {
                return Err(bounds_error(
                    StatusCode::INDEX_OUT_OF_BOUNDS,
                    IndexKind::MemberCount,
                    field_handle.field,
                    fields_count,
                ));
            }
        }
        Ok(())
    }

    fn check_variant_field_handle(&self, field_handle: &VariantFieldHandle) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.struct_defs(), field_handle.struct_index)?;
        if field_handle.variants.is_empty() {
            return Err(verification_error(
                StatusCode::ZERO_VARIANTS_ERROR,
                IndexKind::MemberCount,
                field_handle.field,
            ));
        }
        let struct_def = self.view.struct_def_at(field_handle.struct_index)?;
        for variant in &field_handle.variants {
            Self::check_variant_index(struct_def, *variant)?;
            let field_count = struct_def.field_information.field_count(Some(*variant));
            if field_handle.field as usize >= field_count {
                return Err(bounds_error(
                    StatusCode::INDEX_OUT_OF_BOUNDS,
                    IndexKind::MemberCount,
                    field_handle.field,
                    field_count,
                ));
            }
        }
        Ok(())
    }

    fn check_struct_variant_handle(&self, handle: &StructVariantHandle) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.struct_defs(), handle.struct_index)?;
        let struct_def = self.view.struct_def_at(handle.struct_index)?;
        Self::check_variant_index(struct_def, handle.variant)
    }

    fn check_variant_index(
        struct_def: &StructDefinition,
        variant_index: VariantIndex,
    ) -> PartialVMResult<()> {
        let count = struct_def.field_information.variant_count();
        if (variant_index as usize) >= count {
            Err(bounds_error(
                StatusCode::INDEX_OUT_OF_BOUNDS,
                IndexKind::MemberCount,
                variant_index,
                count,
            ))
        } else {
            Ok(())
        }
    }

    fn check_struct_instantiation(
        &self,
        struct_instantiation: &StructDefInstantiation,
    ) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.struct_defs(), struct_instantiation.def)?;
        check_bounds_impl(self.view.signatures(), struct_instantiation.type_parameters)
    }

    fn check_function_instantiation(
        &self,
        function_instantiation: &FunctionInstantiation,
    ) -> PartialVMResult<()> {
        check_bounds_impl(self.view.function_handles(), function_instantiation.handle)?;
        check_bounds_impl(
            self.view.signatures(),
            function_instantiation.type_parameters,
        )
    }

    fn check_field_instantiation(
        &self,
        field_instantiation: &FieldInstantiation,
    ) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.field_handles(), field_instantiation.handle)?;
        check_bounds_impl(self.view.signatures(), field_instantiation.type_parameters)
    }

    fn check_variant_field_instantiation(
        &self,
        inst: &VariantFieldInstantiation,
    ) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.variant_field_handles(), inst.handle)?;
        check_bounds_impl(self.view.signatures(), inst.type_parameters)
    }

    fn check_struct_variant_instantiation(
        &self,
        inst: &StructVariantInstantiation,
    ) -> PartialVMResult<()> {
        check_bounds_impl_opt(&self.view.struct_variant_handles(), inst.handle)?;
        check_bounds_impl(self.view.signatures(), inst.type_parameters)
    }

    fn check_signature(&self, signature: &Signature) -> PartialVMResult<()> {
        for ty in &signature.0 {
            self.check_type(ty)?
        }
        Ok(())
    }

    fn check_constant(&self, constant: &Constant) -> PartialVMResult<()> {
        self.check_type(&constant.type_)
    }

    fn check_struct_def(
        &self,
        struct_def: &StructDefinition,
        struct_def_idx: usize,
    ) -> PartialVMResult<()> {
        check_bounds_impl(self.view.struct_handles(), struct_def.struct_handle)?;
        // check signature (type) and type parameter for the field types
        let type_param_count = self
            .view
            .struct_handles()
            .get(struct_def.struct_handle.into_index())
            .map_or(0, |sh| sh.type_parameters.len());
        match &struct_def.field_information {
            StructFieldInformation::Declared(fields) => {
                // field signatures are inlined
                for field in fields {
                    self.check_field_def(type_param_count, field)?;
                }
            },
            StructFieldInformation::DeclaredVariants(variants) => {
                for variant in variants {
                    check_bounds_impl(self.view.identifiers(), variant.name)?;
                    for field in &variant.fields {
                        self.check_field_def(type_param_count, field)?;
                    }
                }
                if variants.is_empty() {
                    // Empty variants are not allowed
                    return Err(verification_error(
                        StatusCode::ZERO_VARIANTS_ERROR,
                        IndexKind::StructDefinition,
                        struct_def_idx as TableIndex,
                    ));
                }
            },
            StructFieldInformation::Native => {},
        }
        Ok(())
    }

    fn check_field_def(
        &self,
        type_param_count: usize,
        field: &FieldDefinition,
    ) -> Result<(), PartialVMError> {
        check_bounds_impl(self.view.identifiers(), field.name)?;
        self.check_type(&field.signature.0)?;
        self.check_type_parameters_in_ty(&field.signature.0, type_param_count)?;
        Ok(())
    }

    fn check_function_def(
        &mut self,
        function_def_idx: usize,
        function_def: &FunctionDefinition,
    ) -> PartialVMResult<()> {
        self.context = BoundsCheckingContext::ModuleFunction(FunctionDefinitionIndex(
            function_def_idx as TableIndex,
        ));
        check_bounds_impl(self.view.function_handles(), function_def.function)?;
        for ty in &function_def.acquires_global_resources {
            check_bounds_impl_opt(&self.view.struct_defs(), *ty)?;
        }

        let code_unit = match &function_def.code {
            Some(code) => code,
            None => return Ok(()),
        };

        if function_def.function.into_index() >= self.view.function_handles().len() {
            return Err(verification_error(
                StatusCode::INDEX_OUT_OF_BOUNDS,
                IndexKind::FunctionDefinition,
                function_def_idx as TableIndex,
            ));
        }
        let function_handle = &self.view.function_handles()[function_def.function.into_index()];
        if function_handle.parameters.into_index() >= self.view.signatures().len() {
            return Err(verification_error(
                StatusCode::INDEX_OUT_OF_BOUNDS,
                IndexKind::FunctionDefinition,
                function_def_idx as TableIndex,
            ));
        }
        let parameters = &self.view.signatures()[function_handle.parameters.into_index()];

        self.check_code(
            code_unit,
            &function_handle.type_parameters,
            parameters,
            function_def_idx,
        )
    }

    fn check_code(
        &self,
        code_unit: &CodeUnit,
        type_parameters: &[AbilitySet],
        parameters: &Signature,
        index: usize,
    ) -> PartialVMResult<()> {
        check_bounds_impl(self.view.signatures(), code_unit.locals)?;

        let locals = self.get_locals(code_unit)?;
        // Use saturating add for stability
        let locals_count = locals.len().saturating_add(parameters.len());

        if locals_count > LocalIndex::MAX as usize {
            return Err(verification_error(
                StatusCode::TOO_MANY_LOCALS,
                IndexKind::FunctionDefinition,
                index as TableIndex,
            ));
        }

        // if there are locals check that the type parameters in local signature are in bounds.
        let type_param_count = type_parameters.len();
        self.check_type_parameters_in_signature(code_unit.locals, type_param_count)?;

        // check bytecodes
        let code_len = code_unit.code.len();
        for (bytecode_offset, bytecode) in code_unit.code.iter().enumerate() {
            use self::Bytecode::*;

            match bytecode {
                LdConst(idx) => self.check_code_unit_bounds_impl(
                    self.view.constant_pool(),
                    *idx,
                    bytecode_offset,
                )?,
                MutBorrowField(idx) | ImmBorrowField(idx) => self.check_code_unit_bounds_impl_opt(
                    &self.view.field_handles(),
                    *idx,
                    bytecode_offset,
                )?,
                MutBorrowVariantField(idx) | ImmBorrowVariantField(idx) => self
                    .check_code_unit_bounds_impl_opt(
                        &self.view.variant_field_handles(),
                        *idx,
                        bytecode_offset,
                    )?,
                MutBorrowFieldGeneric(idx) | ImmBorrowFieldGeneric(idx) => {
                    self.check_code_unit_bounds_impl_opt(
                        &self.view.field_instantiations(),
                        *idx,
                        bytecode_offset,
                    )?;
                    if let Some(field_inst) = self
                        .view
                        .field_instantiations()
                        .and_then(|f| f.get(idx.into_index()))
                    {
                        self.check_type_parameters_in_signature(
                            field_inst.type_parameters,
                            type_param_count,
                        )?;
                    }
                },
                MutBorrowVariantFieldGeneric(idx) | ImmBorrowVariantFieldGeneric(idx) => {
                    self.check_code_unit_bounds_impl_opt(
                        &self.view.variant_field_instantiations(),
                        *idx,
                        bytecode_offset,
                    )?;
                    if let Some(field_inst) = self
                        .view
                        .variant_field_instantiations()
                        .and_then(|f| f.get(idx.into_index()))
                    {
                        self.check_type_parameters_in_signature(
                            field_inst.type_parameters,
                            type_param_count,
                        )?;
                    }
                },
                Call(idx) | PackClosure(idx, _) => self.check_code_unit_bounds_impl(
                    self.view.function_handles(),
                    *idx,
                    bytecode_offset,
                )?,
                CallGeneric(idx) | PackClosureGeneric(idx, _) => {
                    self.check_code_unit_bounds_impl(
                        self.view.function_instantiations(),
                        *idx,
                        bytecode_offset,
                    )?;
                    if let Some(func_inst) =
                        self.view.function_instantiations().get(idx.into_index())
                    {
                        self.check_type_parameters_in_signature(
                            func_inst.type_parameters,
                            type_param_count,
                        )?;
                    }
                },
                Pack(idx) | Unpack(idx) | Exists(idx) | ImmBorrowGlobal(idx)
                | MutBorrowGlobal(idx) | MoveFrom(idx) | MoveTo(idx) => self
                    .check_code_unit_bounds_impl_opt(
                        &self.view.struct_defs(),
                        *idx,
                        bytecode_offset,
                    )?,
                PackVariant(idx) | UnpackVariant(idx) | TestVariant(idx) => self
                    .check_code_unit_bounds_impl_opt(
                        &self.view.struct_variant_handles(),
                        *idx,
                        bytecode_offset,
                    )?,
                PackGeneric(idx)
                | UnpackGeneric(idx)
                | ExistsGeneric(idx)
                | ImmBorrowGlobalGeneric(idx)
                | MutBorrowGlobalGeneric(idx)
                | MoveFromGeneric(idx)
                | MoveToGeneric(idx) => {
                    self.check_code_unit_bounds_impl_opt(
                        &self.view.struct_instantiations(),
                        *idx,
                        bytecode_offset,
                    )?;
                    // check type parameters in type operations are bound to the function type parameters
                    if let Some(struct_inst) = self
                        .view
                        .struct_instantiations()
                        .and_then(|s| s.get(idx.into_index()))
                    {
                        self.check_type_parameters_in_signature(
                            struct_inst.type_parameters,
                            type_param_count,
                        )?;
                    }
                },
                PackVariantGeneric(idx) | UnpackVariantGeneric(idx) | TestVariantGeneric(idx) => {
                    self.check_code_unit_bounds_impl_opt(
                        &self.view.struct_variant_instantiations(),
                        *idx,
                        bytecode_offset,
                    )?;
                    // check type parameters
                    if let Some(struct_variant_inst) = self
                        .view
                        .struct_variant_instantiations()
                        .and_then(|s| s.get(idx.into_index()))
                    {
                        self.check_type_parameters_in_signature(
                            struct_variant_inst.type_parameters,
                            type_param_count,
                        )?;
                    }
                },
                // Instructions that refer to this code block.
                BrTrue(offset) | BrFalse(offset) | Branch(offset) => {
                    let offset = *offset as usize;
                    if offset >= code_len {
                        return Err(self.offset_out_of_bounds(
                            StatusCode::INDEX_OUT_OF_BOUNDS,
                            IndexKind::CodeDefinition,
                            offset,
                            code_len,
                            bytecode_offset as CodeOffset,
                        ));
                    }
                },
                // Instructions that refer to the locals.
                CopyLoc(idx) | MoveLoc(idx) | StLoc(idx) | MutBorrowLoc(idx)
                | ImmBorrowLoc(idx) => {
                    let idx = *idx as usize;
                    if idx >= locals_count {
                        return Err(self.offset_out_of_bounds(
                            StatusCode::INDEX_OUT_OF_BOUNDS,
                            IndexKind::LocalPool,
                            idx,
                            locals_count,
                            bytecode_offset as CodeOffset,
                        ));
                    }
                },

                // Instructions that refer to a signature
                CallClosure(idx)
                | VecPack(idx, _)
                | VecLen(idx)
                | VecImmBorrow(idx)
                | VecMutBorrow(idx)
                | VecPushBack(idx)
                | VecPopBack(idx)
                | VecUnpack(idx, _)
                | VecSwap(idx) => {
                    self.check_code_unit_bounds_impl(
                        self.view.signatures(),
                        *idx,
                        bytecode_offset,
                    )?;
                    self.check_type_parameters_in_signature(*idx, type_param_count)?;
                },

                // List out the other options explicitly so there's a compile error if a new
                // bytecode gets added.
                FreezeRef | Pop | Ret | LdU8(_) | LdU16(_) | LdU32(_) | LdU64(_) | LdU256(_)
                | LdU128(_) | CastU8 | CastU16 | CastU32 | CastU64 | CastU128 | CastU256
                | LdTrue | LdFalse | ReadRef | WriteRef | Add | Sub | Mul | Mod | Div | BitOr
                | BitAnd | Xor | Shl | Shr | Or | And | Not | Eq | Neq | Lt | Gt | Le | Ge
                | Abort | Nop => (),
            }
        }
        Ok(())
    }

    fn check_type(&self, ty: &SignatureToken) -> PartialVMResult<()> {
        use self::SignatureToken::*;

        for ty in ty.preorder_traversal() {
            match ty {
                Bool | U8 | U16 | U32 | U64 | U128 | U256 | Address | Signer | TypeParameter(_)
                | Reference(_) | MutableReference(_) | Vector(_) | Function(..) => (),
                Struct(idx) => {
                    check_bounds_impl(self.view.struct_handles(), *idx)?;
                    if let Some(sh) = self.view.struct_handles().get(idx.into_index()) {
                        if !sh.type_parameters.is_empty() {
                            return Err(PartialVMError::new(
                                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
                            )
                            .with_message(format!(
                                "expected {} type parameters got 0 (Struct)",
                                sh.type_parameters.len(),
                            )));
                        }
                    }
                },
                StructInstantiation(idx, type_params) => {
                    check_bounds_impl(self.view.struct_handles(), *idx)?;
                    if let Some(sh) = self.view.struct_handles().get(idx.into_index()) {
                        if sh.type_parameters.len() != type_params.len() {
                            return Err(PartialVMError::new(
                                StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH,
                            )
                            .with_message(format!(
                                "expected {} type parameters got {}",
                                sh.type_parameters.len(),
                                type_params.len(),
                            )));
                        }
                    }
                },
            }
        }
        Ok(())
    }

    fn check_type_parameters_in_ty(
        &self,
        ty: &SignatureToken,
        type_param_count: usize,
    ) -> PartialVMResult<()> {
        for ty in ty.preorder_traversal() {
            if let SignatureToken::TypeParameter(idx) = ty {
                if *idx as usize >= type_param_count {
                    return Err(bounds_error(
                        StatusCode::INDEX_OUT_OF_BOUNDS,
                        IndexKind::TypeParameter,
                        *idx,
                        type_param_count,
                    ));
                }
            }
        }

        Ok(())
    }

    fn check_type_parameters_in_signature(
        &self,
        sig_idx: SignatureIndex,
        type_param_count: usize,
    ) -> PartialVMResult<()> {
        let max_ty_param_idx = match self.max_ty_param_ids.borrow_mut().entry(sig_idx) {
            btree_map::Entry::Vacant(entry) => {
                let mut max_idx = None;

                // This can panic but it is fine, since we only allow this function to be called after the signature index
                // has been verified.
                let sig = self.view.signature_at(sig_idx);

                for ty in sig.0.iter().flat_map(|ty| ty.preorder_traversal()) {
                    if let SignatureToken::TypeParameter(idx) = ty {
                        match &mut max_idx {
                            Some(max_idx) => *max_idx = u16::max(*max_idx, *idx),
                            None => max_idx = Some(*idx),
                        }
                    }
                }

                *entry.insert(max_idx)
            },
            btree_map::Entry::Occupied(entry) => *entry.get(),
        };

        match max_ty_param_idx {
            Some(idx) => {
                if idx as usize >= type_param_count {
                    Err(bounds_error(
                        StatusCode::INDEX_OUT_OF_BOUNDS,
                        IndexKind::TypeParameter,
                        idx,
                        type_param_count,
                    ))
                } else {
                    Ok(())
                }
            },
            None => Ok(()),
        }
    }

    fn check_code_unit_bounds_impl_opt<T, I>(
        &self,
        pool: &Option<&[T]>,
        idx: I,
        bytecode_offset: usize,
    ) -> PartialVMResult<()>
    where
        I: ModuleIndex,
    {
        pool.map_or(Ok(()), |p| {
            self.check_code_unit_bounds_impl(p, idx, bytecode_offset)
        })
    }

    fn check_code_unit_bounds_impl<T, I>(
        &self,
        pool: &[T],
        idx: I,
        bytecode_offset: usize,
    ) -> PartialVMResult<()>
    where
        I: ModuleIndex,
    {
        let idx = idx.into_index();
        let len = pool.len();
        if idx >= len {
            Err(self.offset_out_of_bounds(
                StatusCode::INDEX_OUT_OF_BOUNDS,
                I::KIND,
                idx,
                len,
                bytecode_offset as CodeOffset,
            ))
        } else {
            Ok(())
        }
    }

    fn get_locals(&self, code_unit: &CodeUnit) -> PartialVMResult<&[SignatureToken]> {
        match self.view.signatures().get(code_unit.locals.into_index()) {
            Some(signature) => Ok(&signature.0),
            None => Err(bounds_error(
                StatusCode::INDEX_OUT_OF_BOUNDS,
                IndexKind::Signature,
                code_unit.locals.into_index() as u16,
                self.view.signatures().len(),
            )),
        }
    }

    fn offset_out_of_bounds(
        &self,
        status: StatusCode,
        kind: IndexKind,
        target_offset: usize,
        target_pool_len: usize,
        cur_bytecode_offset: CodeOffset,
    ) -> PartialVMError {
        match self.context {
            BoundsCheckingContext::Module => {
                let msg = format!("Indexing into bytecode {} during bounds checking but 'current_function' was not set", cur_bytecode_offset);
                PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR).with_message(msg)
            },
            BoundsCheckingContext::ModuleFunction(current_function_index) => {
                offset_out_of_bounds_error(
                    status,
                    kind,
                    target_offset,
                    target_pool_len,
                    current_function_index,
                    cur_bytecode_offset,
                )
            },
            BoundsCheckingContext::Script => {
                let msg = format!(
        "Index {} out of bounds for {} at bytecode offset {} in script while indexing {}",
        target_offset, target_pool_len, cur_bytecode_offset, kind);
                PartialVMError::new(status).with_message(msg)
            },
        }
    }
}

fn check_bounds_impl_opt<T, I>(pool: &Option<&[T]>, idx: I) -> PartialVMResult<()>
where
    I: ModuleIndex,
{
    pool.map_or(Ok(()), |p| check_bounds_impl(p, idx))
}

fn check_bounds_impl<T, I>(pool: &[T], idx: I) -> PartialVMResult<()>
where
    I: ModuleIndex,
{
    let idx = idx.into_index();
    let len = pool.len();
    if idx >= len {
        Err(bounds_error(
            StatusCode::INDEX_OUT_OF_BOUNDS,
            I::KIND,
            idx as TableIndex,
            len,
        ))
    } else {
        Ok(())
    }
}
