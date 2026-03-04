// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{EntryFunctionId, MoveType};
use aptos_types::serde_helper::vec_bytes;
use move_core_types::{
    identifier::Identifier,
    language_storage::{ModuleId, TypeTag},
};
use serde::{Deserialize, Serialize};

/// View request for the Move View Function API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
