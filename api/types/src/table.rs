// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::MoveType;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableItemRequest {
    pub key_type: MoveType,
    pub value_type: MoveType,
    pub key: Value,
}
