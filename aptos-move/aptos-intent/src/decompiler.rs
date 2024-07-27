// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{ArgumentOperation, BatchArgument, BatchedFunctionCall, PreviousResult};
use anyhow::{anyhow, bail, Result};
use move_binary_format::{
    access::ScriptAccess,
    file_format::{Bytecode, CompiledScript, SignatureToken},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
    transaction_argument::convert_txn_args,
};
use std::collections::BTreeMap;
use wasm_bindgen::{prelude::*, JsValue};

struct LocalState {
    local_idx_to_return_idx: BTreeMap<u8, (u16, u16)>,
    payload: Vec<BatchedFunctionCall>,
    current_call: CallState,
}

struct CallState {
    args: Vec<BatchArgument>,
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

    fn process_script(
        &mut self,
        script: &CompiledScript,
        ty_args: &[TypeTag],
        args: &[Vec<u8>],
    ) -> anyhow::Result<()> {
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
                self.process_input_argument(op_code, script, ty_args, args)?;
            }
            let op_code = op_codes
                .next()
                .ok_or_else(|| anyhow!("Unexpected end of opcode sequence"))?
                .clone();
            self.process_call(op_code, script, ty_args, args)?;
            while matches!(op_codes.peek(), Some(Bytecode::StLoc(_))) {
                let op_code = op_codes
                    .next()
                    .ok_or_else(|| anyhow!("Unexpected end of opcode sequence"))?
                    .clone();
                self.process_return(op_code, script, ty_args, args)?;
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
        self.payload.push(BatchedFunctionCall {
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
        _ty_args: &[TypeTag],
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
                .push(BatchArgument::PreviousResult(PreviousResult {
                    call_idx: *call_idx,
                    return_idx: *return_idx,
                    operation_type,
                }))
        } else {
            // Local is an argument type
            let signer_counts = script.signature_at(script.parameters).0.len() - args.len();
            if local_idx as usize >= signer_counts {
                // Local is a passed in parameter
                self.current_call.args.push(BatchArgument::Raw(
                    args.get(local_idx as usize - signer_counts)
                        .ok_or_else(|| anyhow!("Parameter access out of bound"))?
                        .clone(),
                ))
            } else {
                // Should signers be ignored to align with how we display script?
                self.current_call
                    .args
                    .push(BatchArgument::Signer(local_idx as u16))
            }
        }
        Ok(())
    }

    fn process_call(
        &mut self,
        op: Bytecode,
        script: &CompiledScript,
        ty_args: &[TypeTag],
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
                        .map(|tok| match tok {
                            SignatureToken::TypeParameter(idx) => ty_args
                                .get(*idx as usize)
                                .ok_or_else(|| anyhow!("Type parameter access out of bound"))
                                .cloned(),
                            _ => bail!("Unexpected type argument: {:?}", tok),
                        })
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
        _ty_args: &[TypeTag],
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
}

pub fn generate_intent_payload(script: Vec<u8>) -> anyhow::Result<Vec<BatchedFunctionCall>> {
    let script = bcs::from_bytes::<crate::codegen::Script>(&script)?;
    let compiled_script = CompiledScript::deserialize(&script.code)
        .map_err(|err| anyhow!("Failed to deserialize payload: {:?}", err))?;
    let mut state = LocalState::new();
    let args = convert_txn_args(&script.args);
    state.process_script(&compiled_script, &script.ty_args, &args)?;
    Ok(state.payload)
}

#[wasm_bindgen]
pub fn generate_intent_payload_wasm(script: Vec<u8>) -> Result<Vec<BatchedFunctionCall>, JsValue> {
    generate_intent_payload(script).map_err(|err| JsValue::from_str(format!("{:?}", err).as_str()))
}
