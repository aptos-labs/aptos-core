// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use move_binary_format::{access::ModuleAccess, errors::PartialVMResult, CompiledModule};
use move_core_types::{
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, TypeTag},
    resolver::ModuleResolver,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use wasm_bindgen::prelude::*;

mod codegen;

#[wasm_bindgen]
/// Arguments for each function.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum BatchArgumentType {
    Raw,
    Signer,
    PreviousResult,
}

#[wasm_bindgen]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviousResult {
    call_idx: u16,
    return_idx: u16,
    operation_type: ArgumentOperation,
}

#[wasm_bindgen]
/// Arguments for each function. Wasm bindgen only support C-style enum so use option to work around.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchArgument {
    ty: BatchArgumentType,
    raw: Option<Vec<u8>>,
    signer: Option<u16>,
    previous_result: Option<PreviousResult>,
}

#[wasm_bindgen]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArgumentOperation {
    Move,
    Copy,
    Borrow,
    BorrowMut,
}

#[wasm_bindgen]
/// Call a Move entry function.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct BatchedFunctionCall {
    module: String,
    function: String,
    ty_args: Vec<String>,
    args: Vec<BatchArgument>,
}

#[wasm_bindgen]
pub struct BatchedFunctionCallBuilder {
    calls: Vec<BatchedFunctionCall>,
    num_signers: u16,
    resolver: NullResolver,
}

#[wasm_bindgen]
impl BatchedFunctionCallBuilder {
    fn num_of_returns(&self, module_id: &ModuleId, func_name: &IdentStr) -> anyhow::Result<u16> {
        if let Some(bytes) = self.resolver.get_module(module_id)? {
            let module =
                CompiledModule::deserialize(&bytes).map_err(|err| anyhow::anyhow!("{:?}", err))?;
            for def in module.function_defs() {
                let handle = module.function_handle_at(def.function);
                if module.identifier_at(handle.name) == func_name {
                    return Ok(module.signature_at(handle.return_).0.len() as u16);
                }
            }
        }
        anyhow::bail!("{}::{} doesn't exist on chain", module_id, func_name)
    }

    pub fn single_signer() -> Self {
        Self {
            calls: vec![],
            num_signers: 1,
            resolver: NullResolver(),
        }
    }

    pub fn multi_signer(signer_count: u16) -> Self {
        Self {
            calls: vec![],
            num_signers: signer_count,
            resolver: NullResolver(),
        }
    }

    fn add_batched_call_impl(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<BatchArgument>,
    ) -> anyhow::Result<Vec<BatchArgument>> {
        self.calls.push(BatchedFunctionCall {
            module: module.clone(),
            function: function.clone(),
            ty_args,
            args,
        });
        let return_count = self.num_of_returns(
            &ModuleId::from_str(&module)?,
            &Identifier::new(function.clone())?,
        )?;
        Ok((0..return_count)
            .into_iter()
            .map(|return_idx| BatchArgument {
                ty: BatchArgumentType::PreviousResult,
                raw: None,
                signer: None,
                previous_result: Some(PreviousResult {
                    call_idx: self.calls.len() as u16,
                    return_idx,
                    operation_type: ArgumentOperation::Move,
                }),
            })
            .collect())
    }

    pub fn add_batched_call(
        &mut self,
        module: String,
        function: String,
        ty_args: Vec<String>,
        args: Vec<BatchArgument>,
    ) -> Result<Vec<BatchArgument>, JsValue> {
        self.add_batched_call_impl(module, function, ty_args, args)
            .map_err(|err| JsValue::from(format!("{:?}", err)))
    }

    pub fn generate_batched_calls(self) -> Result<Vec<u8>, JsValue> {
        crate::codegen::generate_script_from_batched_calls(
            &self.calls,
            self.num_signers,
            &NullResolver(),
        )
        .map_err(|err| JsValue::from(format!("{:?}", err)))
    }
}

#[wasm_bindgen]
impl BatchArgument {
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        Self {
            ty: BatchArgumentType::Raw,
            raw: Some(bytes),
            signer: None,
            previous_result: None,
        }
    }

    pub fn new_signer(signer_idx: u16) -> Self {
        Self {
            ty: BatchArgumentType::Signer,
            raw: None,
            signer: Some(signer_idx),
            previous_result: None,
        }
    }

    pub fn borrow(&self) -> Result<BatchArgument, JsValue> {
        self.change_op_type(ArgumentOperation::Borrow)
    }

    pub fn borrow_mut(&self) -> Result<BatchArgument, JsValue> {
        self.change_op_type(ArgumentOperation::BorrowMut)
    }

    pub fn copy(&self) -> Result<BatchArgument, JsValue> {
        self.change_op_type(ArgumentOperation::Copy)
    }

    fn change_op_type(&self, operation_type: ArgumentOperation) -> Result<BatchArgument, JsValue> {
        match (&self.ty, &self.previous_result) {
            (
                BatchArgumentType::PreviousResult,
                Some(PreviousResult {
                    call_idx,
                    return_idx,
                    operation_type: _,
                }),
            ) => Ok(BatchArgument {
                ty: BatchArgumentType::PreviousResult,
                raw: None,
                signer: None,
                previous_result: Some(PreviousResult {
                    call_idx: *call_idx,
                    return_idx: *return_idx,
                    operation_type,
                }),
            }),
            _ => Err(JsValue::from_str(
                "Unexpected argument type, can only borrow from previous function results",
            )),
        }
    }
}

#[wasm_bindgen]
struct NullResolver();

impl ModuleResolver for NullResolver {
    type Error = anyhow::Error;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Bytes>, Self::Error> {
        anyhow::bail!("non")
    }

    fn get_module_metadata(
        &self,
        module_id: &ModuleId,
    ) -> Vec<move_core_types::metadata::Metadata> {
        vec![]
    }
}
