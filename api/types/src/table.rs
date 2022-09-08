// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::MoveType;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Table Item request for the GetTableItem API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct TableItemRequest {
    /// The [`MoveType`] of the table item's key
    pub key_type: MoveType,
    /// The [`MoveType`] of the table item's value
    pub value_type: MoveType,
    /// The value of the table item's key
    pub key: Value,
}
