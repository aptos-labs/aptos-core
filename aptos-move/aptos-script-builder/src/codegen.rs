// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ArgumentOperation, BatchArgument, BatchedFunctionCall, PreviousResult, APTOS_SCRIPT_BUILDER_KEY,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    builders::CompiledScriptBuilder,
    errors::{PartialVMError, PartialVMResult},
    file_format::*,
};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    metadata::Metadata,
    transaction_argument::TransactionArgument,
    vm_status::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Default)]
struct Context {
    script: CompiledScript,
    // Arguments to the compiled script.
    args: Vec<Vec<u8>>,
    // (nth move call) -> (type arguments for the call)
    ty_args: Vec<Vec<SignatureToken>>,
    // (nth move call) -> (starting position of its return values in the local pool)
    returned_val_to_local: Vec<u16>,
    // (nth move call) -> (starting position of its arguments in the argument pool)
    args_to_local: Vec<u16>,
    // (nth move call) -> how many return values it got
    return_counts: Vec<u16>,
    // Number of signers at the beginning of arugment pool
    signer_counts: u16,
    // Local signature pool of the script. We will use this pool to replace the pool in CompiledScript.
    locals: Vec<SignatureToken>,
    // Parameter signature pool of the script. We will use this pool to replace the pool in CompiledScript.
    parameters: Vec<SignatureToken>,
}

fn instantiate(
    token: &SignatureToken,
    subst_mapping: &BTreeMap<u16, SignatureToken>,
) -> SignatureToken {
    use SignatureToken::*;

    match token {
        Bool => Bool,
        U8 => U8,
        U16 => U16,
        U32 => U32,
        U64 => U64,
        U128 => U128,
        U256 => U256,
        Address => Address,
        Signer => Signer,
        Vector(ty) => Vector(Box::new(instantiate(ty, subst_mapping))),
        Struct(idx) => Struct(*idx),
        StructInstantiation(idx, struct_type_args) => StructInstantiation(
            *idx,
            struct_type_args
                .iter()
                .map(|ty| instantiate(ty, subst_mapping))
                .collect(),
        ),
        Reference(ty) => Reference(Box::new(instantiate(ty, subst_mapping))),
        MutableReference(ty) => MutableReference(Box::new(instantiate(ty, subst_mapping))),
        TypeParameter(idx) => subst_mapping.get(idx).unwrap().clone(),
    }
}

impl Context {
    // Add signers to the beginning of the script's parameter pool
    fn add_signers(&mut self, signer_counts: u16) -> PartialVMResult<()> {
        for _ in 0..signer_counts {
            self.parameters
                .push(SignatureToken::Reference(Box::new(SignatureToken::Signer)));
        }
        self.signer_counts = signer_counts;
        Ok(())
    }

    // Iterate through the batched calls and convert all arguments in the BatchedFunctionCall into
    // arguments to the script.
    fn allocate_parameters(&mut self, calls: &[BatchedFunctionCall]) -> PartialVMResult<()> {
        let mut total_args_count = self.parameters.len() as u16;
        for call in calls {
            self.args_to_local.push(total_args_count);
            for arg in call.args.iter() {
                // Only the serialized arguments need to be allocated as input to the script.
                // Return values from previous calls will be stored in the locals.
                if let BatchArgument::Raw(bytes) = arg {
                    if total_args_count == TableIndex::MAX {
                        return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                    }
                    self.args.push(bytes.clone());
                    total_args_count += 1;
                }
            }
        }
        Ok(())
    }

    // Generate instructions for invoking one batched call.
    fn compile_batched_call(
        &mut self,
        current_call_idx: usize,
        call: &BatchedFunctionCall,
        func_id: FunctionHandleIndex,
    ) -> PartialVMResult<()> {
        let func_handle = self.script.function_handle_at(func_id).clone();

        // Allocate generic type parameters to the script
        //
        // Creates mapping from local type argument index to type argument index in the script
        let mut subst_mapping = BTreeMap::new();
        for (idx, ty_param) in self.ty_args[current_call_idx].iter().enumerate() {
            subst_mapping.insert(idx as u16, ty_param.clone());
        }

        // Instructions for loading parameters
        for (idx, arg) in call.args.iter().enumerate() {
            match arg {
                BatchArgument::PreviousResult(PreviousResult {
                    call_idx,
                    return_idx,
                    operation_type,
                }) => {
                    if let Some(idx) = self.returned_val_to_local.get(*call_idx as usize) {
                        if *return_idx >= self.return_counts[*call_idx as usize] {
                            return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                        }
                        // Locals will need to be offset by:
                        // 1. The parameters to the script (signer + arguments)
                        // 2. All the previously returned values prior to the call.
                        let local_idx =
                            (*idx + *return_idx + self.args.len() as u16 + self.signer_counts)
                                as u8;
                        self.script.code.code.push(match operation_type {
                            ArgumentOperation::Borrow => Bytecode::ImmBorrowLoc(local_idx),
                            ArgumentOperation::BorrowMut => Bytecode::MutBorrowLoc(local_idx),
                            ArgumentOperation::Move => Bytecode::MoveLoc(local_idx),
                            ArgumentOperation::Copy => Bytecode::CopyLoc(local_idx),
                        });
                    }
                },
                BatchArgument::Signer(i) => {
                    // Signer will always appear at the beginning of locals. No offsets needed.
                    self.script.code.code.push(Bytecode::CopyLoc(*i as u8));
                },
                BatchArgument::Raw(_) => {
                    // A new serialized argument to the script. Push the type to the parameter pool of the script.
                    let type_ = &self.script.signature_at(func_handle.parameters).0[idx];
                    let inst_ty = if call.ty_args.is_empty() {
                        type_.clone()
                    } else {
                        instantiate(type_, &subst_mapping)
                    };
                    let param_idx = self.parameters.len() as u8;
                    self.parameters.push(inst_ty);
                    self.script.code.code.push(Bytecode::MoveLoc(param_idx));
                },
            }
        }

        // Make function call
        if call.ty_args.is_empty() {
            self.script.code.code.push(Bytecode::Call(func_id));
        } else {
            if self.script.function_instantiations.len() >= TableIndex::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            let fi_idx =
                FunctionInstantiationIndex(self.script.function_instantiations.len() as u16);

            // Generic call type arguments will always be instantiated with script's type parameters.
            let inst_sig = self.ty_args[current_call_idx].clone();

            let type_parameters = self.import_signature(Signature(inst_sig))?;
            self.script
                .function_instantiations
                .push(FunctionInstantiation {
                    handle: func_id,
                    type_parameters,
                });
            self.script.code.code.push(Bytecode::CallGeneric(fi_idx));
        }

        self.returned_val_to_local.push(self.locals.len() as u16);
        self.return_counts
            .push(self.script.signature_at(func_handle.return_).0.len() as u16);

        // Instruction for storing return values
        let ret_locals = self
            .script
            .signature_at(func_handle.return_)
            .0
            .iter()
            .map(|ret| {
                if call.ty_args.is_empty() {
                    ret.clone()
                } else {
                    instantiate(ret, &subst_mapping)
                }
            })
            .collect::<Vec<_>>();

        for ret_ty in ret_locals {
            let local_idx = self.locals.len() + self.args.len() + self.signer_counts as usize;
            if local_idx >= u8::MAX as usize {
                return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
            }
            self.locals.push(ret_ty);
            self.script.code.code.push(Bytecode::StLoc(local_idx as u8));
        }

        Ok(())
    }

    fn import_signature(&mut self, sig: Signature) -> PartialVMResult<SignatureIndex> {
        Ok(SignatureIndex(match self
            .script
            .signatures()
            .iter()
            .position(|item| item == &sig)
        {
            Some(idx) => idx,
            None => {
                let idx = self.script.signatures().len();
                if idx >= TableIndex::MAX as usize {
                    return Err(PartialVMError::new(StatusCode::INDEX_OUT_OF_BOUNDS));
                }
                self.script.signatures.push(sig);
                idx
            },
        } as u16))
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Script {
    #[serde(with = "serde_bytes")]
    pub code: Vec<u8>,
    pub ty_args: Vec<TypeTag>,
    pub args: Vec<TransactionArgument>,
}

// Given a module, return the handle idx of the named struct
fn find_struct<'a>(
    map: &'a BTreeMap<ModuleId, CompiledModule>,
    module_id: &ModuleId,
    struct_name: &IdentStr,
) -> PartialVMResult<(&'a CompiledModule, StructHandleIndex)> {
    if let Some(module) = map.get(module_id) {
        for (idx, handle) in module.struct_handles().iter().enumerate() {
            if module.identifier_at(handle.name) == struct_name {
                return Ok((module, StructHandleIndex::new(idx as TableIndex)));
            }
        }
        return Err(
            PartialVMError::new(StatusCode::LOOKUP_FAILED).with_message(format!(
                "Struct {}::{} doesn't yet exist in the cache",
                module_id, struct_name
            )),
        );
    }
    Err(
        PartialVMError::new(StatusCode::LOOKUP_FAILED).with_message(format!(
            "Module {} doesn't yet exist in the cache",
            module_id
        )),
    )
}

fn import_type_tag(
    script_builder: &mut CompiledScriptBuilder,
    type_tag: &TypeTag,
    module_resolver: &BTreeMap<ModuleId, CompiledModule>,
) -> PartialVMResult<SignatureToken> {
    Ok(match type_tag {
        TypeTag::Address => SignatureToken::Address,
        TypeTag::U8 => SignatureToken::U8,
        TypeTag::U16 => SignatureToken::U16,
        TypeTag::U32 => SignatureToken::U32,
        TypeTag::U64 => SignatureToken::U64,
        TypeTag::U128 => SignatureToken::U128,
        TypeTag::U256 => SignatureToken::U256,
        TypeTag::Bool => SignatureToken::Bool,
        TypeTag::Signer => SignatureToken::Signer,
        TypeTag::Vector(t) => SignatureToken::Vector(Box::new(import_type_tag(
            script_builder,
            t,
            module_resolver,
        )?)),
        TypeTag::Struct(s) => {
            let (module, handle_idx) =
                find_struct(module_resolver, &s.module_id(), s.name.as_ident_str())?;
            let struct_idx = script_builder.import_struct(module, handle_idx)?;
            if s.type_args.is_empty() {
                SignatureToken::Struct(struct_idx)
            } else {
                SignatureToken::StructInstantiation(
                    struct_idx,
                    s.type_args
                        .iter()
                        .map(|ty| import_type_tag(script_builder, ty, module_resolver))
                        .collect::<PartialVMResult<Vec<_>>>()?,
                )
            }
        },
    })
}

#[allow(clippy::field_reassign_with_default)]
pub fn generate_script_from_batched_calls(
    calls: &[BatchedFunctionCall],
    signer_count: u16,
    module_resolver: &BTreeMap<ModuleId, CompiledModule>,
) -> PartialVMResult<Vec<u8>> {
    let mut context = Context::default();
    let mut script_builder = CompiledScriptBuilder::new(CompiledScript::default());
    let call_idxs = calls
        .iter()
        .map(|call| {
            let target_module = match module_resolver.get(&call.module) {
                Some(module) => module,
                None => {
                    return Err(PartialVMError::new(StatusCode::LOOKUP_FAILED)
                        .with_message(format!("Module {} is not yet loaded", call.module)))
                },
            };
            script_builder.import_call_by_name(&call.function, target_module)
        })
        .collect::<PartialVMResult<Vec<_>>>()?;
    let call_type_params = calls
        .iter()
        .map(|call| {
            call.ty_args
                .iter()
                .map(|ty| import_type_tag(&mut script_builder, ty, module_resolver))
                .collect::<PartialVMResult<Vec<_>>>()
        })
        .collect::<PartialVMResult<Vec<_>>>()?;
    context.script = script_builder.into_script();
    context.ty_args = call_type_params;
    context.add_signers(signer_count)?;
    context.allocate_parameters(calls)?;
    for (call_count, (call, fh_idx)) in calls.iter().zip(call_idxs.into_iter()).enumerate() {
        context.compile_batched_call(call_count, call, fh_idx)?;
    }
    context.script.code.code.push(Bytecode::Ret);
    context.script.parameters = context.import_signature(Signature(context.parameters.clone()))?;
    context.script.code.locals = context.import_signature(Signature(context.locals.clone()))?;
    move_bytecode_verifier::verify_script(&context.script).map_err(|err| {
        err.to_partial()
            .append_message_with_separator(',', format!("{:?}", context.script))
    })?;
    context.script.metadata.push(Metadata {
        key: APTOS_SCRIPT_BUILDER_KEY.to_owned(),
        value: vec![],
    });
    let mut bytes = vec![];
    context
        .script
        .serialize(&mut bytes)
        .map_err(|_| PartialVMError::new(StatusCode::CODE_DESERIALIZATION_ERROR))?;
    Ok(bcs::to_bytes(&Script {
        code: bytes,
        ty_args: vec![],
        args: context
            .args
            .into_iter()
            .map(TransactionArgument::Serialized)
            .collect(),
    })
    .unwrap())
}
