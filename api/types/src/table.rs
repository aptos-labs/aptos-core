// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{HexEncodedBytes, MoveType, VerifyInput, VerifyInputWithRecursion};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Table Item request for the GetTableItem API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableItemRequest {
    pub key_type: MoveType,
    pub value_type: MoveType,
    /// The value of the table item's key
    pub key: Value,
}

impl VerifyInput for TableItemRequest {
    fn verify(&self) -> anyhow::Result<()> {
        self.key_type.verify(0)?;
        self.value_type.verify(0)
    }
}

/// Table Item request for the GetTableItemRaw API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawTableItemRequest {
    pub key: HexEncodedBytes,
}
