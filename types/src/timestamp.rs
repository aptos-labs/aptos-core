// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::move_resource::{MoveResource, MoveStructType};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TimestampResource {
    pub timestamp: Timestamp,
}

impl MoveStructType for TimestampResource {
    const MODULE_NAME: &'static str = "timestamp";
    const STRUCT_NAME: &'static str = "CurrentTimeMicroseconds";
}

impl MoveResource for TimestampResource {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Timestamp {
    pub microseconds: u64,
}
