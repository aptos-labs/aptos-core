// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub use crate::{
    builder::TransactionComposer,
    decompiler::{generate_batched_call_payload, generate_batched_call_payload_wasm},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use serde::{Deserialize, Serialize};
use tsify_next::Tsify;
use wasm_bindgen::prelude::wasm_bindgen;

mod builder;
mod decompiler;
mod helpers;

#[cfg(test)]
pub mod tests;

// CompiledScript generated from script builder will have this key in its metadata to distinguish from other scripts.
pub static APTOS_SCRIPT_BUILDER_KEY: &[u8] = "aptos::script_builder".as_bytes();

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviousResult {
    call_idx: u16,
    return_idx: u16,
    operation_type: ArgumentOperation,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize, Tsify)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum BatchArgument {
    Raw(Vec<u8>),
    Signer(u16),
    PreviousResult(PreviousResult),
}

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
    module: ModuleId,
    function: Identifier,
    ty_args: Vec<TypeTag>,
    args: Vec<BatchArgument>,
}

impl BatchedFunctionCall {
    pub fn into_inner(self) -> (ModuleId, Identifier, Vec<TypeTag>, Vec<BatchArgument>) {
        (self.module, self.function, self.ty_args, self.args)
    }
}

#[wasm_bindgen]
impl BatchArgument {
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        BatchArgument::Raw(bytes)
    }

    pub fn new_signer(signer_idx: u16) -> Self {
        BatchArgument::Signer(signer_idx)
    }

    pub fn borrow(&self) -> Result<BatchArgument, String> {
        self.change_op_type(ArgumentOperation::Borrow)
    }

    pub fn borrow_mut(&self) -> Result<BatchArgument, String> {
        self.change_op_type(ArgumentOperation::BorrowMut)
    }

    pub fn copy(&self) -> Result<BatchArgument, String> {
        self.change_op_type(ArgumentOperation::Copy)
    }

    fn change_op_type(&self, operation_type: ArgumentOperation) -> Result<BatchArgument, String> {
        match &self {
            BatchArgument::PreviousResult(r) => {
                let mut result = r.clone();
                result.operation_type = operation_type;
                Ok(BatchArgument::PreviousResult(result))
            },
            _ => Err(
                "Unexpected argument type, can only borrow from previous function results"
                    .to_string(),
            ),
        }
    }
}
