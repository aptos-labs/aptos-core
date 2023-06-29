// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{EntryFunctionId, MoveType};
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
