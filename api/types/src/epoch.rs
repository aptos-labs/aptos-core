// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::U64;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

/// The version range for a sealed epoch.
#[derive(Clone, Debug, Deserialize, Eq, Object, PartialEq, Serialize)]
pub struct Epoch {
    pub epoch: U64,
    pub first_version: U64,
    pub last_version: U64,
}

impl Epoch {
    pub fn new(epoch: u64, first_version: u64, last_version: u64) -> Self {
        Self {
            epoch: epoch.into(),
            first_version: first_version.into(),
            last_version: last_version.into(),
        }
    }
}
