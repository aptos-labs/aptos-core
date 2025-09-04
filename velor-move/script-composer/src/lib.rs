// Copyright © Velor Foundation
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
use wasm_bindgen::prelude::wasm_bindgen;

mod builder;
mod decompiler;
mod helpers;

#[cfg(test)]
pub mod tests;

/// CompiledScript generated from script composer will have this key in its metadata to
/// distinguish from other scripts.
pub static VELOR_SCRIPT_COMPOSER_KEY: &[u8] = "velor::script_composer".as_bytes();

/// Representing a returned value from a previous list of `MoveFunctionCall`s.
#[wasm_bindgen]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct PreviousResult {
    /// Refering to the return value in the `call_idx`th call.
    call_idx: u16,
    /// Refering to the `return_idx`th return value in that call, since move function call can
    /// return multiple values.
    return_idx: u16,
    /// How this result would be used.
    operation_type: ArgumentOperation,
}

/// Arguments to the `MoveFunctionCall`.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum CallArgument {
    /// Passing raw bytes to the function. The bytes must follows the existing constraints for
    /// transaction arguments.
    Raw(Vec<u8>),
    /// Refering to signer of the transaction. If it's a single signer transaction you will only
    /// be able to access `Signer(0)`. You will be able to access other signers if it's a multi
    /// agent transaction.
    Signer(u16),
    /// The arugment came from the returned value of a previous `MoveFunctionCall`.
    PreviousResult(PreviousResult),
}

/// How to consume the returned value coming from a previous `MoveFunctionCall`.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArgumentOperation {
    /// Move the returned value to the next caller. This can be used for values that don't have
    /// `copy` ability.
    Move,
    /// Copy the returned value and pass it to the next caller.
    Copy,
    /// Borrow an immutable reference from a returned value and pass it to the next caller.
    Borrow,
    /// Borrow a mutable reference from a returned value and pass it to the next caller.
    BorrowMut,
}

/// Calling a Move function.
///
/// Similar to a public entry function call, but the arguments could specified as `CallArgument`,
/// which can be a return value of a previous `MoveFunctionCall`.
#[wasm_bindgen]
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct MoveFunctionCall {
    module: ModuleId,
    function: Identifier,
    ty_args: Vec<TypeTag>,
    args: Vec<CallArgument>,
}

/// Version of the script composer so that decompiler will know how to decode the script as we
/// upgrade.
#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComposerVersion {
    V1,
}

impl MoveFunctionCall {
    pub fn into_inner(self) -> (ModuleId, Identifier, Vec<TypeTag>, Vec<CallArgument>) {
        (self.module, self.function, self.ty_args, self.args)
    }
}

impl CallArgument {
    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        CallArgument::Raw(bytes)
    }

    pub fn new_signer(signer_idx: u16) -> Self {
        CallArgument::Signer(signer_idx)
    }

    pub fn borrow(&self) -> Result<CallArgument, String> {
        self.change_op_type(ArgumentOperation::Borrow)
    }

    pub fn borrow_mut(&self) -> Result<CallArgument, String> {
        self.change_op_type(ArgumentOperation::BorrowMut)
    }

    pub fn copy(&self) -> Result<CallArgument, String> {
        self.change_op_type(ArgumentOperation::Copy)
    }

    pub(crate) fn change_op_type(
        &self,
        operation_type: ArgumentOperation,
    ) -> Result<CallArgument, String> {
        match &self {
            CallArgument::PreviousResult(r) => {
                let mut result = r.clone();
                result.operation_type = operation_type;
                Ok(CallArgument::PreviousResult(result))
            },
            _ => Err(
                "Unexpected argument type, can only borrow from previous function results"
                    .to_string(),
            ),
        }
    }
}
