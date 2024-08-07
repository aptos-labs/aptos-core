// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub use crate::{
    builder::{BatchArgumentWASM, BatchedFunctionCallBuilder},
    decompiler::{generate_intent_payload, generate_intent_payload_wasm},
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::wasm_bindgen;

mod builder;
mod codegen;
mod decompiler;
#[cfg(test)]
pub mod tests;

// CompiledScript generated from intents will have this key in its metadata to distinguish from other scripts.
pub static APTOS_INTENT_KEY: &[u8] = "aptos::intent".as_bytes();

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviousResult {
    call_idx: u16,
    return_idx: u16,
    operation_type: ArgumentOperation,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
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
