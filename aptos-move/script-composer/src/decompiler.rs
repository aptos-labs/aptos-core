// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Code to decode the series of individual Move calls from a compiled script.
//!
//! Core api is `generate_batched_call_payload` which takes in a compiled Move script and render it as a
//! series of `BatchedFunctionCall`.

use crate::{
    ArgumentOperation, CallArgument, ComposerVersion, MoveFunctionCall, PreviousResult,
    APTOS_SCRIPT_COMPOSER_KEY,
};
use anyhow::{anyhow, bail, Result};
use move_binary_format::{
    access::ScriptAccess,
    file_format::{Bytecode, CompiledScript, SignatureToken},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, StructTag, TypeTag},
    transaction_argument::{convert_txn_args, TransactionArgument},
};
use std::collections::BTreeMap;
use wasm_bindgen::{prelude::*, JsValue};

struct LocalState {
    local_idx_to_return_idx: BTreeMap<u8, (u16, u16)>,
    payload: Vec<MoveFunctionCall>,
    current_call: CallState,
}

struct CallState {
    args: Vec<CallArgument>,
    call_info: Option<(ModuleId, Identifier, Vec<TypeTag>)>,
    num_returns: u16,
}

impl LocalState {
    fn new() -> Self {
        Self {
            local_idx_to_return_idx: BTreeMap::new(),
            payload: vec![],
            current_call: CallState {
                args: vec![],
                call_info: None,
                num_returns: 0,
            },
        }
    }

    fn process_script(&mut self, script: &CompiledScript, args: &[Vec<u8>]) -> anyhow::Result<()> {
        let mut op_codes = script.code.code.iter().peekable();
        while !(matches!(op_codes.peek(), Some(Bytecode::Ret))) {
            while !matches!(
                op_codes.peek(),
                Some(Bytecode::Call(_)) | Some(Bytecode::CallGeneric(_))
            ) {
                let op_code = op_codes
                    .next()
                    .ok_or_else(|| anyhow!("Unexpected end of opcode sequence"))?
                    .clone();
                self.process_input_argument(op_code, script, args)?;
            }
            let op_code = op_codes
                .next()
                .ok_or_else(|| anyhow!("Unexpected end of opcode sequence"))?
                .clone();
            self.process_call(op_code, script, args)?;
            while matches!(op_codes.peek(), Some(Bytecode::StLoc(_))) {
                let op_code = op_codes
                    .next()
                    .ok_or_else(|| anyhow!("Unexpected end of opcode sequence"))?
                    .clone();
                self.process_return(op_code, script, args)?;
            }
            self.add_new_call()?;
        }
        Ok(())
    }

    fn add_new_call(&mut self) -> anyhow::Result<()> {
        let mut new_state = CallState {
            args: vec![],
            call_info: None,
            num_returns: 0,
        };
        std::mem::swap(&mut new_state, &mut self.current_call);
        let (module, function, ty_args) = new_state
            .call_info
            .ok_or_else(|| anyhow!("Call information not available"))?;
        self.payload.push(MoveFunctionCall {
            module,
            function,
            ty_args,
            args: new_state.args,
        });
        Ok(())
    }

    fn process_input_argument(
        &mut self,
        op: Bytecode,
        script: &CompiledScript,
        args: &[Vec<u8>],
    ) -> anyhow::Result<()> {
        let (operation_type, local_idx) = match op {
            Bytecode::MoveLoc(i) => (ArgumentOperation::Move, i),
            Bytecode::CopyLoc(i) => (ArgumentOperation::Copy, i),
            Bytecode::ImmBorrowLoc(i) => (ArgumentOperation::Borrow, i),
            Bytecode::MutBorrowLoc(i) => (ArgumentOperation::BorrowMut, i),
            _ => bail!("Unexpected opcode: {:?}", op),
        };
        if local_idx as usize >= script.signature_at(script.parameters).0.len() {
            // Local is a return value of previous output
            let (call_idx, return_idx) = self
                .local_idx_to_return_idx
                .get(&local_idx)
                .ok_or_else(|| anyhow!("Loading unknown index"))?;
            self.current_call
                .args
                .push(CallArgument::PreviousResult(PreviousResult {
                    call_idx: *call_idx,
                    return_idx: *return_idx,
                    operation_type,
                }))
        } else {
            // Local is an argument type
            let signer_counts = script.signature_at(script.parameters).0.len() - args.len();
            if local_idx as usize >= signer_counts {
                // Local is a passed in parameter
                self.current_call.args.push(CallArgument::Raw(
                    args.get(local_idx as usize - signer_counts)
                        .ok_or_else(|| anyhow!("Parameter access out of bound"))?
                        .clone(),
                ))
            } else {
                // Should signers be ignored to align with how we display script?
                self.current_call
                    .args
                    .push(CallArgument::Signer(local_idx as u16))
            }
        }
        Ok(())
    }

    fn process_call(
        &mut self,
        op: Bytecode,
        script: &CompiledScript,
        _args: &[Vec<u8>],
    ) -> anyhow::Result<()> {
        let (fh_idx, ty_args) = match op {
            Bytecode::Call(idx) => (idx, vec![]),
            Bytecode::CallGeneric(idx) => {
                let inst = script.function_instantiation_at(idx);
                (
                    inst.handle,
                    script
                        .signature_at(inst.type_parameters)
                        .0
                        .iter()
                        .map(|tok| Self::type_tag_from_sig_token(script, tok))
                        .collect::<Result<Vec<_>>>()?,
                )
            },
            _ => bail!("Unexpected opcode: {:?}", op),
        };
        let handle = script.function_handle_at(fh_idx);
        let module = script.module_id_for_handle(script.module_handle_at(handle.module));
        let func_name = script.identifier_at(handle.name);
        self.current_call.call_info = Some((module, func_name.to_owned(), ty_args));
        Ok(())
    }

    fn process_return(
        &mut self,
        op: Bytecode,
        _script: &CompiledScript,
        _args: &[Vec<u8>],
    ) -> anyhow::Result<()> {
        match op {
            Bytecode::StLoc(i) => {
                self.local_idx_to_return_idx.insert(
                    i,
                    (self.payload.len() as u16, self.current_call.num_returns),
                );
                self.current_call.num_returns += 1;
            },
            _ => bail!("Unexpected opcode: {:?}", op),
        }
        Ok(())
    }

    fn type_tag_from_sig_token(
        script: &CompiledScript,
        token: &SignatureToken,
    ) -> anyhow::Result<TypeTag> {
        Ok(match token {
            SignatureToken::Address => TypeTag::Address,
            SignatureToken::U8 => TypeTag::U8,
            SignatureToken::U16 => TypeTag::U16,
            SignatureToken::U32 => TypeTag::U32,
            SignatureToken::U64 => TypeTag::U64,
            SignatureToken::U128 => TypeTag::U128,
            SignatureToken::U256 => TypeTag::U256,
            SignatureToken::Bool => TypeTag::Bool,
            SignatureToken::Signer => TypeTag::Signer,
            SignatureToken::Vector(s) => {
                TypeTag::Vector(Box::new(Self::type_tag_from_sig_token(script, s)?))
            },
            SignatureToken::Function(..) => {
                // TODO(LAMBDA)
                bail!("function types not yet implemented for script composer")
            },
            SignatureToken::Struct(s) => {
                let module_handle = script.module_handle_at(script.struct_handle_at(*s).module);
                TypeTag::Struct(Box::new(StructTag {
                    module: script.identifier_at(module_handle.name).to_owned(),
                    address: *script.address_identifier_at(module_handle.address),
                    name: script
                        .identifier_at(script.struct_handle_at(*s).name)
                        .to_owned(),
                    type_args: vec![],
                }))
            },
            SignatureToken::TypeParameter(_)
            | SignatureToken::Reference(_)
            | SignatureToken::MutableReference(_) => {
                bail!("Not supported arugment type: {:?}", token);
            },
            SignatureToken::StructInstantiation(s, ty_args) => {
                let module_handle = script.module_handle_at(script.struct_handle_at(*s).module);
                TypeTag::Struct(Box::new(StructTag {
                    module: script.identifier_at(module_handle.name).to_owned(),
                    address: *script.address_identifier_at(module_handle.address),
                    name: script
                        .identifier_at(script.struct_handle_at(*s).name)
                        .to_owned(),
                    type_args: ty_args
                        .iter()
                        .map(|tok| Self::type_tag_from_sig_token(script, tok))
                        .collect::<anyhow::Result<Vec<_>>>()?,
                }))
            },
        })
    }
}

/// Public function to decompiled a script and return the batched form for better readability.
pub fn generate_batched_call_payload(
    script_code: &[u8],
    args: &[TransactionArgument],
) -> anyhow::Result<Vec<MoveFunctionCall>> {
    let compiled_script = CompiledScript::deserialize(script_code)
        .map_err(|err| anyhow!("Failed to deserialize payload: {:?}", err))?;
    let expected_version = bcs::to_bytes(&ComposerVersion::V1)
        .map_err(|err| anyhow!("Failed to serialize version metadata: {:?}", err))?;
    if !compiled_script
        .metadata
        .iter()
        .any(|md| md.key == APTOS_SCRIPT_COMPOSER_KEY && md.value == expected_version)
    {
        bail!("Script is not generated by script builder. Skip decompilation steps.");
    }
    let mut state = LocalState::new();
    let converted_args = convert_txn_args(args);
    state.process_script(&compiled_script, &converted_args)?;
    Ok(state.payload)
}

/// Wrapper to decompile script in its serialized form.
pub fn generate_batched_call_payload_serialized(
    script: &[u8],
) -> anyhow::Result<Vec<MoveFunctionCall>> {
    let script = bcs::from_bytes::<crate::helpers::Script>(script)?;
    generate_batched_call_payload(&script.code, &script.args)
}

/// Wrapper to decompile script in its serialized form and wrap it with wasm errors.
#[wasm_bindgen]
pub fn generate_batched_call_payload_wasm(
    script: Vec<u8>,
) -> Result<Vec<MoveFunctionCall>, JsValue> {
    generate_batched_call_payload_serialized(&script)
        .map_err(|err| JsValue::from_str(format!("{:?}", err).as_str()))
}
