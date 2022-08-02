// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SfStreamerConfig {
    pub enabled: bool,
    pub starting_version: u64,
}
