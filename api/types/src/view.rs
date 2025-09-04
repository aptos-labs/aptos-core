// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{EntryFunctionId, MoveType};
use velor_types::serde_helper::vec_bytes;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// View request for the Move View Function API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct ViewRequest {
    pub function: EntryFunctionId,
    /// Type arguments of the function
    pub type_arguments: Vec<MoveType>,
    /// Arguments of the function
    pub arguments: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub struct ViewFunction {
    pub module: ModuleId,
    pub function: Identifier,
    pub ty_args: Vec<TypeTag>,
    #[serde(with = "vec_bytes")]
    pub args: Vec<Vec<u8>>,
}
