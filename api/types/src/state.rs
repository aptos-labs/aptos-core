// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::HexEncodedBytes;
use serde::{Deserialize, Serialize};

/// Table Item request for the GetTableItemRaw API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawStateValueRequest {
    pub key: HexEncodedBytes,
}
