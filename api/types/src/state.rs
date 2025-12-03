// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::HexEncodedBytes;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Table Item request for the GetTableItemRaw API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct RawStateValueRequest {
    pub key: HexEncodedBytes,
}
