// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::move_resource::{MoveResource, MoveStructType};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// A Rust representation of ObjectGroup.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ObjectGroupResource {}

impl MoveStructType for ObjectGroupResource {
    const MODULE_NAME: &'static str = "object";
    const STRUCT_NAME: &'static str = "ObjectGroup";
}

impl MoveResource for ObjectGroupResource {}
