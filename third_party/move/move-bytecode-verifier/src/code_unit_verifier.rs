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
        find_offset_for_public_api, retrieve_struct_full_name_for_public_api, Bytecode,
        CompiledModule, CompiledScript, FunctionDefinition, FunctionDefinitionIndex,
        IdentifierIndex, TableIndex,
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

    fn verify_module_impl(
        verifier_config: &VerifierConfig,
        module: &CompiledModule,
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

        if verifier_config.enable_public_api_borrow_check {
            let func_name = module
                .identifier_at(module.function_handle_at(function_definition.function).name)
                .as_str()
                .to_string();
            if let Some(splited_names) = retrieve_struct_full_name_for_public_api(&func_name) {
                if splited_names.len() >= 4 {
                    let oper = splited_names[0].clone();
                    if oper.contains("borrow") {
                        if find_offset_for_public_api(&func_name).is_some() {
                            // Since the function will be type checked, we only check the instructions in the body
                            if code.code.len() == 3 {
                                let borrow_field_op = if oper.contains("mut") {
                                    matches!(
                                        code.code[1],
                                        Bytecode::MutBorrowField(_)
                                            | Bytecode::MutBorrowFieldGeneric(_)
                                            | Bytecode::MutBorrowVariantField(_)
                                            | Bytecode::MutBorrowVariantFieldGeneric(_)
                                    )
                                } else {
                                    matches!(
                                        code.code[1],
                                        Bytecode::ImmBorrowField(_)
                                            | Bytecode::ImmBorrowFieldGeneric(_)
                                            | Bytecode::ImmBorrowVariantField(_)
                                            | Bytecode::ImmBorrowVariantFieldGeneric(_)
                                    )
                                };
                                let move_loc = matches!(code.code[0], Bytecode::MoveLoc(_));
                                let ret = matches!(code.code[2], Bytecode::Ret);
                                if !borrow_field_op && move_loc && ret {
                                    return Err(PartialVMError::new(
                                        StatusCode::PUBLIC_API_BORROW_NOT_PROPERLY_DEFINED,
                                    )
                                    .at_code_offset(index, 0));
                                }
                            } else {
                                return Err(PartialVMError::new(
                                    StatusCode::PUBLIC_API_BORROW_NOT_PROPERLY_DEFINED,
                                )
                                .at_code_offset(index, 0));
                            }
                        }
                        return Err(PartialVMError::new(
                            StatusCode::PUBLIC_API_BORROW_NOT_PROPERLY_DEFINED,
                        )
                        .at_code_offset(index, 0));
                    }
                }
            }
        }
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
            verifier_config.enable_public_api_borrow_check,
        )
    }
}
