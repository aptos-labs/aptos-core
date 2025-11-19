// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module implements the checker for verifying correctness of function bodies.
//! The overall verification is split between stack_usage_verifier.rs and
//! abstract_interpreter.rs. CodeUnitVerifier simply orchestrates calls into these two files.
use crate::{
    acquires_list_verifier::AcquiresVerifier,
    control_flow, locals_safety,
    meter::{BoundMeter, Meter, Scope},
    reference_safety,
    stack_usage_verifier::StackUsageVerifier,
    type_safety,
    verifier::VerifierConfig,
};
use move_binary_format::{
    access::ModuleAccess,
    binary_views::{BinaryIndexedView, FunctionView},
    control_flow_graph::ControlFlowGraph,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        Bytecode, CodeUnit, CompiledModule, CompiledScript, FunctionAttribute, FunctionDefinition,
        FunctionDefinitionIndex, IdentifierIndex, MemberCount, TableIndex,
    },
    IndexKind,
};
use move_core_types::vm_status::StatusCode;
use std::collections::HashMap;

pub struct CodeUnitVerifier<'a> {
    resolver: BinaryIndexedView<'a>,
    function_view: FunctionView<'a>,
    name_def_map: &'a HashMap<IdentifierIndex, FunctionDefinitionIndex>,
}

impl<'a> CodeUnitVerifier<'a> {
    pub fn verify_module(
        verifier_config: &VerifierConfig,
        module: &'a CompiledModule,
    ) -> VMResult<()> {
        Self::verify_module_impl(verifier_config, module)
            .map_err(|e| e.finish(Location::Module(module.self_id())))
    }

    /// Check the pattern of the pack API.
    /// Pattern:
    /// MoveLoc(...)
    /// Pack(...) | PackGeneric(...)
    /// Ret
    fn pattern_check_for_pack(code: &CodeUnit) -> PartialVMResult<()> {
        if code.code.len() < 2 {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        }

        match (
            &code.code[code.code.len() - 1],
            &code.code[code.code.len() - 2],
        ) {
            (&Bytecode::Ret, &Bytecode::Pack(_) | &Bytecode::PackGeneric(_)) => {},
            _ => {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            },
        }

        for i in 0..code.code.len() - 2 {
            if !matches!(code.code[i], Bytecode::MoveLoc(_)) {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
        }

        Ok(())
    }

    /// Check the pattern of the pack variant API.
    /// Pattern:
    /// MoveLoc(...)
    /// PackVariant(...) | PackVariantGeneric(...)
    /// Ret
    fn pattern_check_for_pack_variant(code: &CodeUnit) -> PartialVMResult<()> {
        if code.code.len() < 2 {
            return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
        }

        match (
            &code.code[code.code.len() - 1],
            &code.code[code.code.len() - 2],
        ) {
            (&Bytecode::Ret, &Bytecode::PackVariant(_) | &Bytecode::PackVariantGeneric(_)) => {},
            _ => {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            },
        }

        for i in 0..code.code.len() - 2 {
            if !matches!(code.code[i], Bytecode::MoveLoc(_)) {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            }
        }

        Ok(())
    }

    /// Check the pattern of the unpack API.
    /// Pattern:
    /// MoveLoc(...)
    /// Unpack(...) | UnpackGeneric(...)
    /// Ret
    fn pattern_check_for_unpack(code: &CodeUnit) -> PartialVMResult<()> {
        if code.code.len() == 3 {
            match (&code.code[0], &code.code[1], &code.code[2]) {
                (
                    Bytecode::MoveLoc(_),
                    Bytecode::Unpack(_) | Bytecode::UnpackGeneric(_),
                    Bytecode::Ret,
                ) => Ok(()),
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
            }
        } else {
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        }
    }

    /// Check the pattern of the unpack variant API.
    /// Pattern:
    /// MoveLoc(...)
    /// UnpackVariant(...) | UnpackVariantGeneric(...)
    /// Ret
    fn pattern_check_for_unpack_variant(code: &CodeUnit) -> PartialVMResult<()> {
        if code.code.len() == 3 {
            match (&code.code[0], &code.code[1], &code.code[2]) {
                (
                    Bytecode::MoveLoc(_),
                    Bytecode::UnpackVariant(_) | Bytecode::UnpackVariantGeneric(_),
                    Bytecode::Ret,
                ) => Ok(()),
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
            }
        } else {
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        }
    }

    /// Check the pattern of the test variant API.
    /// Pattern:
    /// MoveLoc(...)
    /// TestVariant(...) | TestVariantGeneric(...)
    /// Ret
    fn pattern_check_for_test_variant(code: &CodeUnit) -> PartialVMResult<()> {
        if code.code.len() == 3 {
            match (&code.code[0], &code.code[1], &code.code[2]) {
                (
                    Bytecode::MoveLoc(_),
                    Bytecode::TestVariant(_) | Bytecode::TestVariantGeneric(_),
                    Bytecode::Ret,
                ) => Ok(()),
                _ => Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE)),
            }
        } else {
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        }
    }

    /// Check the pattern of the borrow field API.
    /// Pattern:
    /// MoveLoc(...)
    /// if immut is true:
    ///     ImmBorrowField(...) | ImmBorrowFieldGeneric(...) | ImmBorrowVariantField(...) | ImmBorrowVariantFieldGeneric(...)
    /// else:
    ///     MutBorrowField(...) | MutBorrowFieldGeneric(...) | MutBorrowVariantField(...) | MutBorrowVariantFieldGeneric(...)
    /// Ret
    fn pattern_check_for_borrow_field(
        immut: bool,
        resolver: &BinaryIndexedView,
        offset: &MemberCount,
        code: &CodeUnit,
    ) -> PartialVMResult<()> {
        if code.code.len() == 3 {
            match (&code.code[0], &code.code[2]) {
                (Bytecode::MoveLoc(_), Bytecode::Ret) => {},
                _ => {
                    return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                },
            }
            match (immut, &code.code[1]) {
                (false, Bytecode::MutBorrowField(field_handle_index)) => {
                    let field_handle = resolver.field_handle_at(*field_handle_index)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (false, Bytecode::MutBorrowFieldGeneric(field_inst_index)) => {
                    let field_inst = resolver.field_instantiation_at(*field_inst_index)?;
                    let field_handle = resolver.field_handle_at(field_inst.handle)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (false, Bytecode::MutBorrowVariantField(field_handle_index)) => {
                    let field_handle = resolver.variant_field_handle_at(*field_handle_index)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (false, Bytecode::MutBorrowVariantFieldGeneric(field_inst_index)) => {
                    let field_inst = resolver.variant_field_instantiation_at(*field_inst_index)?;
                    let field_handle = resolver.variant_field_handle_at(field_inst.handle)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (true, Bytecode::ImmBorrowField(field_handle_index)) => {
                    let field_handle = resolver.field_handle_at(*field_handle_index)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (true, Bytecode::ImmBorrowFieldGeneric(field_inst_index)) => {
                    let field_inst = resolver.field_instantiation_at(*field_inst_index)?;
                    let field_handle = resolver.field_handle_at(field_inst.handle)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (true, Bytecode::ImmBorrowVariantField(field_handle_index)) => {
                    let field_handle = resolver.variant_field_handle_at(*field_handle_index)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (true, Bytecode::ImmBorrowVariantFieldGeneric(field_inst_index)) => {
                    let field_inst = resolver.variant_field_instantiation_at(*field_inst_index)?;
                    let field_handle = resolver.variant_field_handle_at(field_inst.handle)?;
                    if field_handle.field != *offset {
                        return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                    }
                },
                (_, _) => {
                    return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                },
            }
            Ok(())
        } else {
            Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE))
        }
    }

    fn try_get_struct_api_attr(
        attrs: &[FunctionAttribute],
    ) -> PartialVMResult<Option<FunctionAttribute>> {
        use FunctionAttribute::*;
        let mut count: usize = 0;
        let mut attr_attribute = None;
        for attr in attrs {
            let is_struct_api_attr = match attr {
                Pack
                | PackVariant(_)
                | Unpack
                | UnpackVariant(_)
                | TestVariant(_)
                | BorrowFieldImmutable(_)
                | BorrowFieldMutable(_) => {
                    attr_attribute = Some(attr.clone());
                    true
                },
                Persistent | ModuleLock => false,
            };
            if is_struct_api_attr {
                count += 1;
                if count > 1 {
                    return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
                }
            }
        }
        Ok(attr_attribute)
    }

    /// Check well-formedness of struct API attributes.
    /// 1) any API related APIs can only appear once for a function.
    /// 2) implementation of each API must be well-formed.
    /// Note that any operations on a struct can only happen in the module where the struct is defined.
    /// Therefore, we only need to check the well-formedness of the attributes in the module where the struct is defined.
    fn check_struct_api_impl(
        resolver: &BinaryIndexedView<'a>,
        module: &'a CompiledModule,
        function_definition: &FunctionDefinition,
    ) -> PartialVMResult<()> {
        let handle = module.function_handle_at(function_definition.function);
        let struct_api_attr = Self::try_get_struct_api_attr(&handle.attributes)?;
        if let Some(attr) = struct_api_attr {
            let Some(code) = function_definition.code.as_ref() else {
                return Err(PartialVMError::new(StatusCode::INVALID_STRUCT_API_CODE));
            };
            match attr {
                FunctionAttribute::Pack => {
                    return Self::pattern_check_for_pack(code);
                },
                FunctionAttribute::PackVariant(_) => {
                    return Self::pattern_check_for_pack_variant(code);
                },
                FunctionAttribute::Unpack => {
                    return Self::pattern_check_for_unpack(code);
                },
                FunctionAttribute::UnpackVariant(_) => {
                    return Self::pattern_check_for_unpack_variant(code);
                },
                FunctionAttribute::TestVariant(_) => {
                    return Self::pattern_check_for_test_variant(code);
                },
                FunctionAttribute::BorrowFieldImmutable(offset) => {
                    return Self::pattern_check_for_borrow_field(true, resolver, &offset, code);
                },
                FunctionAttribute::BorrowFieldMutable(offset) => {
                    return Self::pattern_check_for_borrow_field(false, resolver, &offset, code);
                },
                _ => {},
            }
        }
        Ok(())
    }

    fn verify_module_impl(
        verifier_config: &VerifierConfig,
        module: &'a CompiledModule,
    ) -> PartialVMResult<()> {
        let mut meter = BoundMeter::new(verifier_config);
        let mut name_def_map = HashMap::new();
        for (idx, func_def) in module.function_defs().iter().enumerate() {
            let fh = module.function_handle_at(func_def.function);
            name_def_map.insert(fh.name, FunctionDefinitionIndex(idx as u16));
        }
        let mut total_back_edges = 0;
        for (idx, function_definition) in module.function_defs().iter().enumerate() {
            let index = FunctionDefinitionIndex(idx as TableIndex);
            let num_back_edges = Self::verify_function(
                verifier_config,
                index,
                function_definition,
                module,
                &name_def_map,
                &mut meter,
            )
            .map_err(|err| err.at_index(IndexKind::FunctionDefinition, index.0))?;
            total_back_edges += num_back_edges;
            // check whether struct APIs related code are well-formed.
            let resolver = BinaryIndexedView::Module(module);
            Self::check_struct_api_impl(&resolver, module, function_definition)
                .map_err(|err| err.at_index(IndexKind::FunctionDefinition, index.0))?;
        }
        if let Some(limit) = verifier_config.max_back_edges_per_module {
            if total_back_edges > limit {
                return Err(PartialVMError::new(StatusCode::TOO_MANY_BACK_EDGES));
            }
        }
        Ok(())
    }

    pub fn verify_script(
        verifier_config: &VerifierConfig,
        module: &'a CompiledScript,
    ) -> VMResult<()> {
        Self::verify_script_impl(verifier_config, module).map_err(|e| e.finish(Location::Script))
    }

    fn verify_script_impl(
        verifier_config: &VerifierConfig,
        script: &'a CompiledScript,
    ) -> PartialVMResult<()> {
        let mut meter = BoundMeter::new(verifier_config);
        // create `FunctionView` and `BinaryIndexedView`
        let function_view = control_flow::verify_script(verifier_config, script)?;
        let resolver = BinaryIndexedView::Script(script);
        let name_def_map = HashMap::new();

        if let Some(limit) = verifier_config.max_basic_blocks_in_script {
            if function_view.cfg().blocks().len() > limit {
                return Err(PartialVMError::new(StatusCode::TOO_MANY_BASIC_BLOCKS));
            }
        }

        if let Some(limit) = verifier_config.max_back_edges_per_function {
            if function_view.cfg().num_back_edges() > limit {
                return Err(PartialVMError::new(StatusCode::TOO_MANY_BACK_EDGES));
            }
        }

        //verify
        meter.enter_scope("script", Scope::Function);
        let code_unit_verifier = CodeUnitVerifier {
            resolver,
            function_view,
            name_def_map: &name_def_map,
        };
        code_unit_verifier.verify_common(verifier_config, &mut meter)
    }

    fn verify_function(
        verifier_config: &VerifierConfig,
        index: FunctionDefinitionIndex,
        function_definition: &FunctionDefinition,
        module: &CompiledModule,
        name_def_map: &HashMap<IdentifierIndex, FunctionDefinitionIndex>,
        meter: &mut impl Meter,
    ) -> PartialVMResult<usize> {
        meter.enter_scope(
            module
                .identifier_at(module.function_handle_at(function_definition.function).name)
                .as_str(),
            Scope::Function,
        );
        // nothing to verify for native function
        let code = match &function_definition.code {
            Some(code) => code,
            None => return Ok(0),
        };

        // create `FunctionView` and `BinaryIndexedView`
        let function_view = control_flow::verify_function(
            verifier_config,
            module,
            index,
            function_definition,
            code,
            meter,
        )?;

        if let Some(limit) = verifier_config.max_basic_blocks {
            if function_view.cfg().blocks().len() > limit {
                return Err(
                    PartialVMError::new(StatusCode::TOO_MANY_BASIC_BLOCKS).at_code_offset(index, 0)
                );
            }
        }

        let num_back_edges = function_view.cfg().num_back_edges();
        if let Some(limit) = verifier_config.max_back_edges_per_function {
            if num_back_edges > limit {
                return Err(
                    PartialVMError::new(StatusCode::TOO_MANY_BACK_EDGES).at_code_offset(index, 0)
                );
            }
        }

        let resolver = BinaryIndexedView::Module(module);
        // verify
        let code_unit_verifier = CodeUnitVerifier {
            resolver,
            function_view,
            name_def_map,
        };
        code_unit_verifier.verify_common(verifier_config, meter)?;
        AcquiresVerifier::verify(module, index, function_definition, meter)?;

        meter.transfer(Scope::Function, Scope::Module, 1.0)?;

        Ok(num_back_edges)
    }

    fn verify_common(
        &self,
        verifier_config: &VerifierConfig,
        meter: &mut impl Meter,
    ) -> PartialVMResult<()> {
        StackUsageVerifier::verify(verifier_config, &self.resolver, &self.function_view, meter)?;
        type_safety::verify(&self.resolver, &self.function_view, meter)?;
        locals_safety::verify(&self.resolver, &self.function_view, meter)?;
        reference_safety::verify(
            &self.resolver,
            &self.function_view,
            self.name_def_map,
            meter,
        )
    }
}
