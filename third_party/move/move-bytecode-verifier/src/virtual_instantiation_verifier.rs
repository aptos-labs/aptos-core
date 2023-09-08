// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module defines the transfer functions for verifying type safety of a procedure body.
//! It does not utilize control flow, but does check each block independently

use crate::type_safety::instantiate;
use move_binary_format::{
    binary_views::BinaryIndexedView,
    errors::{PartialVMError, PartialVMResult},
    file_format::{
        Bytecode, CompiledModule, CompiledScript, FunctionHandleIndex, FunctionInstantiation,
        FunctionInstantiationIndex, Signature, VirtualFunctionInstantiation,
    },
};
use move_core_types::vm_status::StatusCode;
use std::collections::BTreeSet;

fn verify_instantiation<'a>(
    resolver: &BinaryIndexedView<'a>,
    inst: &FunctionInstantiation,
) -> PartialVMResult<()> {
    let func_handle = resolver.function_handle_at(inst.handle);
    if func_handle.vtables.len() != inst.vtable_instantiation.len() {
        return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH).with_message(
            "Virtual function instantiation length doesn't match the handle".to_string(),
        ));
    }
    for (func_inst, func_ty) in inst
        .vtable_instantiation
        .iter()
        .zip(func_handle.vtables.iter())
    {
        let expected_parameters = Signature(
            resolver
                .signature_at(func_ty.parameters)
                .0
                .iter()
                .map(|tok| instantiate(tok, resolver.signature_at(inst.type_parameters)))
                .collect(),
        );
        let expected_returns = Signature(
            resolver
                .signature_at(func_ty.return_)
                .0
                .iter()
                .map(|tok| instantiate(tok, resolver.signature_at(inst.type_parameters)))
                .collect(),
        );
        match func_inst {
            VirtualFunctionInstantiation::Defined(handle) => {
                let func = resolver.function_handle_at(*handle);
                // Only need to compare index here as there's no duplication in signature pool.
                if resolver.signature_at(func.parameters) != &expected_parameters
                    || resolver.signature_at(func.return_) != &expected_returns
                {
                    return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH)
                        .with_message("Virtual function instantiation type mismatch".to_string()));
                }
            },
            VirtualFunctionInstantiation::Instantiated(handle) => {
                let func_inst = resolver.function_instantiation_at(*handle);
                let instantiated_handle = resolver.function_handle_at(func_inst.handle);
                let parameter_ty = Signature(
                    resolver
                        .signature_at(instantiated_handle.parameters)
                        .0
                        .iter()
                        .map(|tok| {
                            instantiate(tok, resolver.signature_at(func_inst.type_parameters))
                        })
                        .collect(),
                );
                let return_ty = Signature(
                    resolver
                        .signature_at(instantiated_handle.return_)
                        .0
                        .iter()
                        .map(|tok| {
                            instantiate(tok, resolver.signature_at(func_inst.type_parameters))
                        })
                        .collect(),
                );

                if expected_parameters != parameter_ty || expected_returns != return_ty {
                    return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH)
                        .with_message("Virtual function instantiation type mismatch".to_string()));
                }
            },
            VirtualFunctionInstantiation::Inherited(fh_idx, v_idx) => {
                let function_handle = resolver.function_handle_at(*fh_idx);
                let func_ty = &function_handle.vtables[*v_idx as usize];
                if &expected_parameters != resolver.signature_at(func_ty.parameters)
                    || &expected_returns != resolver.signature_at(func_ty.return_)
                {
                    return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH)
                        .with_message("Virtual function instantiation type mismatch".to_string()));
                }
            },
        }
    }
    Ok(())
}

fn verify_inheritence<'a>(
    resolver: &BinaryIndexedView<'a>,
    fi_idx: FunctionInstantiationIndex,
    self_idx: Option<FunctionHandleIndex>,
    visited: &mut BTreeSet<FunctionInstantiationIndex>,
) -> PartialVMResult<()> {
    let func_inst = resolver.function_instantiation_at(fi_idx);
    for virt_func in func_inst.vtable_instantiation.iter() {
        match virt_func {
            VirtualFunctionInstantiation::Inherited(idx, _) => {
                if let Some(expected_idx) = self_idx {
                    if *idx != expected_idx {
                        return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH).with_message(
                            "Inhertied virtual function doesn't match function handle provided"
                                .to_string(),
                        ));
                    }
                } else {
                    return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH)
                        .with_message("Inhertied virtual function not expected".to_string()));
                }
            },
            VirtualFunctionInstantiation::Instantiated(idx) => {
                if visited.contains(idx) {
                    return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH).with_message(
                        "Virtual function instantiation contains cycle".to_string(),
                    ));
                }
                visited.insert(*idx);
                verify_inheritence(resolver, *idx, self_idx, visited)?
            },
            VirtualFunctionInstantiation::Defined(_) => (),
        }
    }
    Ok(())
}
fn verify_code_unit<'a>(
    resolver: &BinaryIndexedView<'a>,
    codes: &[Bytecode],
    self_idx: Option<FunctionHandleIndex>,
) -> PartialVMResult<()> {
    for opcode in codes.iter() {
        use Bytecode::*;
        match opcode {
            CallGeneric(fi_idx) => {
                verify_inheritence(resolver, *fi_idx, self_idx, &mut BTreeSet::new())?
            },
            PackGeneric(_)
            | UnpackGeneric(_)
            | ExistsGeneric(_)
            | MoveFromGeneric(_)
            | MoveToGeneric(_)
            | ImmBorrowGlobalGeneric(_)
            | MutBorrowGlobalGeneric(_)
            | ImmBorrowFieldGeneric(_)
            | MutBorrowFieldGeneric(_)
            | VecPack(_, _)
            | VecLen(_)
            | VecImmBorrow(_)
            | VecMutBorrow(_)
            | VecPushBack(_)
            | VecPopBack(_)
            | VecUnpack(_, _)
            | VecSwap(_)
            | Pop
            | Ret
            | Branch(_)
            | BrTrue(_)
            | BrFalse(_)
            | LdU8(_)
            | LdU16(_)
            | LdU32(_)
            | LdU64(_)
            | LdU128(_)
            | LdU256(_)
            | LdConst(_)
            | CastU8
            | CastU16
            | CastU32
            | CastU64
            | CastU128
            | CastU256
            | LdTrue
            | LdFalse
            | Call(_)
            | Pack(_)
            | Unpack(_)
            | ReadRef
            | WriteRef
            | FreezeRef
            | Add
            | Sub
            | Mul
            | Mod
            | Div
            | BitOr
            | BitAnd
            | Xor
            | Shl
            | Shr
            | Or
            | And
            | Not
            | Eq
            | Neq
            | Lt
            | Gt
            | Le
            | Ge
            | CopyLoc(_)
            | MoveLoc(_)
            | StLoc(_)
            | MutBorrowLoc(_)
            | ImmBorrowLoc(_)
            | MutBorrowField(_)
            | ImmBorrowField(_)
            | MutBorrowGlobal(_)
            | ImmBorrowGlobal(_)
            | Exists(_)
            | MoveTo(_)
            | MoveFrom(_)
            | Abort
            | Nop
            | CallVirtual(_) => (),
        }
    }
    Ok(())
}

pub(crate) fn verify_common(resolver: &BinaryIndexedView) -> PartialVMResult<()> {
    for func_inst in resolver.function_instantiations() {
        verify_instantiation(resolver, func_inst)?;
    }
    Ok(())
}

pub(crate) fn verify_module(module: &CompiledModule) -> PartialVMResult<()> {
    for def in module.function_defs.iter() {
        if let Some(code) = &def.code {
            verify_code_unit(
                &BinaryIndexedView::Module(module),
                &code.code,
                Some(def.function),
            )?;
        }
    }
    Ok(())
}

pub(crate) fn verify_script(script: &CompiledScript) -> PartialVMResult<()> {
    verify_code_unit(&BinaryIndexedView::Script(script), &script.code.code, None)
}
