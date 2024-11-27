// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module defines the transfer functions for verifying consistency of each bytecode
//! instruction, in particular, for the bytecode instructions that come in both generic and
//! non-generic flavors. It also checks constraints on instructions like VecPack/VecUnpack.

use move_binary_format::{
    access::ModuleAccess,
    binary_views::BinaryIndexedView,
    errors::{Location, PartialVMError, PartialVMResult, VMResult},
    file_format::{
        Bytecode, CodeOffset, CodeUnit, CompiledModule, CompiledScript, FieldHandleIndex,
        FunctionDefinitionIndex, FunctionHandleIndex, SignatureIndex, SignatureToken,
        StructDefinitionIndex, StructVariantHandleIndex, TableIndex, VariantFieldHandleIndex,
    },
};
use move_core_types::vm_status::StatusCode;

pub struct InstructionConsistency<'a> {
    resolver: BinaryIndexedView<'a>,
    current_function: Option<FunctionDefinitionIndex>,
}

impl<'a> InstructionConsistency<'a> {
    pub fn verify_module(module: &'a CompiledModule) -> VMResult<()> {
        Self::verify_module_impl(module).map_err(|e| e.finish(Location::Module(module.self_id())))
    }

    fn verify_module_impl(module: &'a CompiledModule) -> PartialVMResult<()> {
        let resolver = BinaryIndexedView::Module(module);

        for (idx, func_def) in module.function_defs().iter().enumerate() {
            match &func_def.code {
                None => (),
                Some(code) => {
                    let checker = Self {
                        resolver,
                        current_function: Some(FunctionDefinitionIndex(idx as TableIndex)),
                    };
                    checker.check_instructions(code)?
                },
            }
        }
        Ok(())
    }

    pub fn verify_script(module: &'a CompiledScript) -> VMResult<()> {
        Self::verify_script_impl(module).map_err(|e| e.finish(Location::Script))
    }

    pub fn verify_script_impl(script: &'a CompiledScript) -> PartialVMResult<()> {
        let checker = Self {
            resolver: BinaryIndexedView::Script(script),
            current_function: None,
        };
        checker.check_instructions(&script.code)
    }

    fn check_instructions(&self, code: &CodeUnit) -> PartialVMResult<()> {
        for (offset, instr) in code.code.iter().enumerate() {
            use Bytecode::*;

            match instr {
                MutBorrowField(field_handle_index) | ImmBorrowField(field_handle_index) => {
                    self.check_field_op(offset, *field_handle_index, /* generic */ false)?;
                },
                MutBorrowFieldGeneric(field_inst_index)
                | ImmBorrowFieldGeneric(field_inst_index) => {
                    let field_inst = self.resolver.field_instantiation_at(*field_inst_index)?;
                    self.check_field_op(offset, field_inst.handle, /* generic */ true)?;
                },
                MutBorrowVariantField(field_handle_index)
                | ImmBorrowVariantField(field_handle_index) => {
                    self.check_variant_field_op(
                        offset,
                        *field_handle_index,
                        /* generic */ false,
                    )?;
                },
                MutBorrowVariantFieldGeneric(field_inst_index)
                | ImmBorrowVariantFieldGeneric(field_inst_index) => {
                    let field_inst = self
                        .resolver
                        .variant_field_instantiation_at(*field_inst_index)?;
                    self.check_variant_field_op(
                        offset,
                        field_inst.handle,
                        /* generic */ true,
                    )?;
                },
                Call(idx) => {
                    self.check_function_op(offset, *idx, /* generic */ false)?;
                },
                CallGeneric(idx) => {
                    let func_inst = self.resolver.function_instantiation_at(*idx);
                    self.check_function_op(offset, func_inst.handle, /* generic */ true)?;
                },
                LdFunction(idx) => {
                    self.check_ld_function_op(offset, *idx, /* generic */ false)?;
                },
                LdFunctionGeneric(idx) => {
                    let func_inst = self.resolver.function_instantiation_at(*idx);
                    self.check_ld_function_op(offset, func_inst.handle, /* generic */ true)?;
                },
                InvokeFunction(sig_idx) => {
                    // reuse code to check for signature issues.
                    self.check_bind_count(offset, *sig_idx, 0)?;
                },
                EarlyBindFunction(sig_idx, count) => {
                    self.check_bind_count(offset, *sig_idx, *count)?;
                },
                Pack(idx) | Unpack(idx) => {
                    self.check_struct_op(offset, *idx, /* generic */ false)?;
                },
                PackGeneric(idx) | UnpackGeneric(idx) => {
                    let struct_inst = self.resolver.struct_instantiation_at(*idx)?;
                    self.check_struct_op(offset, struct_inst.def, /* generic */ true)?;
                },
                PackVariant(idx) | UnpackVariant(idx) | TestVariant(idx) => {
                    self.check_variant_op(offset, *idx, /* generic */ false)?;
                },
                PackVariantGeneric(idx) | UnpackVariantGeneric(idx) | TestVariantGeneric(idx) => {
                    let struct_inst = self.resolver.struct_variant_instantiation_at(*idx)?;
                    self.check_variant_op(offset, struct_inst.handle, /* generic */ true)?;
                },
                MutBorrowGlobal(idx) | ImmBorrowGlobal(idx) => {
                    self.check_struct_op(offset, *idx, /* generic */ false)?;
                },
                MutBorrowGlobalGeneric(idx) | ImmBorrowGlobalGeneric(idx) => {
                    let struct_inst = self.resolver.struct_instantiation_at(*idx)?;
                    self.check_struct_op(offset, struct_inst.def, /* generic */ true)?;
                },
                Exists(idx) | MoveFrom(idx) | MoveTo(idx) => {
                    self.check_struct_op(offset, *idx, /* generic */ false)?;
                },
                ExistsGeneric(idx) | MoveFromGeneric(idx) | MoveToGeneric(idx) => {
                    let struct_inst = self.resolver.struct_instantiation_at(*idx)?;
                    self.check_struct_op(offset, struct_inst.def, /* generic */ true)?;
                },
                VecPack(_, num) | VecUnpack(_, num) => {
                    if *num > u16::MAX as u64 {
                        return Err(PartialVMError::new(StatusCode::CONSTRAINT_NOT_SATISFIED)
                            .at_code_offset(self.current_function(), offset as CodeOffset)
                            .with_message("VecPack/VecUnpack argument out of range".to_string()));
                    }
                },

                // List out the other options explicitly so there's a compile error if a new
                // bytecode gets added.
                FreezeRef | Pop | Ret | Branch(_) | BrTrue(_) | BrFalse(_) | LdU8(_) | LdU16(_)
                | LdU32(_) | LdU64(_) | LdU128(_) | LdU256(_) | LdConst(_) | CastU8 | CastU16
                | CastU32 | CastU64 | CastU128 | CastU256 | LdTrue | LdFalse | ReadRef
                | WriteRef | Add | Sub | Mul | Mod | Div | BitOr | BitAnd | Xor | Shl | Shr
                | Or | And | Not | Eq | Neq | Lt | Gt | Le | Ge | CopyLoc(_) | MoveLoc(_)
                | StLoc(_) | MutBorrowLoc(_) | ImmBorrowLoc(_) | VecLen(_) | VecImmBorrow(_)
                | VecMutBorrow(_) | VecPushBack(_) | VecPopBack(_) | VecSwap(_) | Abort | Nop => (),
            }
        }
        Ok(())
    }

    //
    // Helpers for instructions that come in a generic and non generic form.
    // Verifies the generic form uses a generic member and the non generic form
    // a non generic one.
    //

    fn check_field_op(
        &self,
        offset: usize,
        field_handle_index: FieldHandleIndex,
        generic: bool,
    ) -> PartialVMResult<()> {
        let field_handle = self.resolver.field_handle_at(field_handle_index)?;
        self.check_struct_op(offset, field_handle.owner, generic)
    }

    fn check_variant_field_op(
        &self,
        offset: usize,
        field_handle_index: VariantFieldHandleIndex,
        generic: bool,
    ) -> PartialVMResult<()> {
        let field_handle = self.resolver.variant_field_handle_at(field_handle_index)?;
        self.check_struct_op(offset, field_handle.struct_index, generic)
    }

    fn current_function(&self) -> FunctionDefinitionIndex {
        self.current_function.unwrap_or(FunctionDefinitionIndex(0))
    }

    fn check_struct_op(
        &self,
        offset: usize,
        struct_def_index: StructDefinitionIndex,
        generic: bool,
    ) -> PartialVMResult<()> {
        let struct_def = self.resolver.struct_def_at(struct_def_index)?;
        let struct_handle = self.resolver.struct_handle_at(struct_def.struct_handle);
        if struct_handle.type_parameters.is_empty() == generic {
            return Err(
                PartialVMError::new(StatusCode::GENERIC_MEMBER_OPCODE_MISMATCH)
                    .at_code_offset(self.current_function(), offset as CodeOffset),
            );
        }
        Ok(())
    }

    fn check_variant_op(
        &self,
        offset: usize,
        idx: StructVariantHandleIndex,
        generic: bool,
    ) -> PartialVMResult<()> {
        let variant_handle = self.resolver.struct_variant_handle_at(idx)?;
        let struct_def = self.resolver.struct_def_at(variant_handle.struct_index)?;
        let struct_handle = self.resolver.struct_handle_at(struct_def.struct_handle);
        if struct_handle.type_parameters.is_empty() == generic {
            return Err(
                PartialVMError::new(StatusCode::GENERIC_MEMBER_OPCODE_MISMATCH)
                    .at_code_offset(self.current_function(), offset as CodeOffset),
            );
        }
        Ok(())
    }

    fn check_ld_function_op(
        &self,
        offset: usize,
        func_handle_index: FunctionHandleIndex,
        generic: bool,
    ) -> PartialVMResult<()> {
        let function_handle = self.resolver.function_handle_at(func_handle_index);
        if function_handle.type_parameters.is_empty() == generic {
            return Err(
                PartialVMError::new(StatusCode::GENERIC_MEMBER_OPCODE_MISMATCH)
                    .at_code_offset(self.current_function(), offset as CodeOffset),
            );
        }
        Ok(())
    }

    fn check_function_op(
        &self,
        offset: usize,
        func_handle_index: FunctionHandleIndex,
        generic: bool,
    ) -> PartialVMResult<()> {
        let function_handle = self.resolver.function_handle_at(func_handle_index);
        if function_handle.type_parameters.is_empty() == generic {
            return Err(
                PartialVMError::new(StatusCode::GENERIC_MEMBER_OPCODE_MISMATCH)
                    .at_code_offset(self.current_function(), offset as CodeOffset),
            );
        }
        Ok(())
    }

    fn check_bind_count(
        &self,
        offset: usize,
        sig_index: SignatureIndex,
        count: u8,
    ) -> PartialVMResult<()> {
        let signature = self.resolver.signature_at(sig_index);
        if let Some(sig_token) = signature.0.first() {
            if let SignatureToken::Function(params, _returns, _abilities) = sig_token {
                if count as usize > params.len() {
                    return Err(
                        PartialVMError::new(StatusCode::NUMBER_OF_TYPE_ARGUMENTS_MISMATCH)
                            .at_code_offset(self.current_function(), offset as CodeOffset),
                    );
                }
            } else {
                return Err(PartialVMError::new(StatusCode::REQUIRES_FUNCTION)
                    .at_code_offset(self.current_function(), offset as CodeOffset));
            }
        } else {
            return Err(PartialVMError::new(StatusCode::UNKNOWN_SIGNATURE_TYPE)
                .at_code_offset(self.current_function(), offset as CodeOffset));
        }
        Ok(())
    }
}
