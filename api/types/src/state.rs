// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::HexEncodedBytes;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// Table Item request for the GetTableItemRaw API
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct RawStateValueRequest {
    pub key: HexEncodedBytes,
}
