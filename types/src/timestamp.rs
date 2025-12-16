// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TimestampResource {
    pub timestamp: Timestamp,
}

impl MoveStructType for TimestampResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("timestamp");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CurrentTimeMicroseconds");
}

impl MoveResource for TimestampResource {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Timestamp {
    pub microseconds: u64,
}
