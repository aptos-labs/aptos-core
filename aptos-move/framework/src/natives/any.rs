// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

/// Rust representation of the Move Any type
#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Any {
    pub type_name: String,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}
