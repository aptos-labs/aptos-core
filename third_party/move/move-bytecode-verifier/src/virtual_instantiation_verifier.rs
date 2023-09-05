// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This module defines the transfer functions for verifying type safety of a procedure body.
//! It does not utilize control flow, but does check each block independently

use crate::type_safety::instantiate;
use move_binary_format::{
    binary_views::{BinaryIndexedView, FunctionView},
    errors::{PartialVMError, PartialVMResult},
    file_format::{Bytecode, FunctionInstantiation, Signature, VirtualFunctionInstantiation},
};
use move_core_types::vm_status::StatusCode;

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
        match func_inst {
            VirtualFunctionInstantiation::Defined(handle) => {
                let func = resolver.function_handle_at(*handle);
                // Only need to compare index here as there's no duplication in signature pool.
                if func.parameters != func_ty.parameters || func.return_ != func_ty.return_ {
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

                if resolver.signature_at(func_ty.parameters) != &parameter_ty
                    || resolver.signature_at(func_ty.return_) != &return_ty
                {
                    return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH)
                        .with_message("Virtual function instantiation type mismatch".to_string()));
                }
            },
            VirtualFunctionInstantiation::Virtual(_) => (),
        }
    }
    Ok(())
}

pub(crate) fn verify_function<'a>(
    resolver: &BinaryIndexedView<'a>,
    function_view: &'a FunctionView<'a>,
) -> PartialVMResult<()> {
    for opcode in function_view.code().code.iter() {
        use Bytecode::*;
        match opcode {
            CallGeneric(func_inst_idx) => {
                let inst = resolver.function_instantiation_at(*func_inst_idx);
                let func_handle = resolver.function_handle_at(inst.handle);
                for (func_inst, func_ty) in inst
                    .vtable_instantiation
                    .iter()
                    .zip(func_handle.vtables.iter())
                {
                    match func_inst {
                        VirtualFunctionInstantiation::Virtual(idx) => {
                            if function_view.vtables()[*idx as usize].parameters
                                != func_ty.parameters
                                || function_view.vtables()[*idx as usize].return_ != func_ty.return_
                            {
                                return Err(PartialVMError::new(StatusCode::TYPE_MISMATCH)
                                    .with_message(
                                        "Virtual function instantiation type mismatch".to_string(),
                                    ));
                            }
                        },
                        VirtualFunctionInstantiation::Defined(_)
                        | VirtualFunctionInstantiation::Instantiated(_) => (),
                    }
                }
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
