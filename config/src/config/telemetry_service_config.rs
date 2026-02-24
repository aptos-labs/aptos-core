// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetryServiceConfig {
    pub num_threads: Option<usize>,
}

impl Default for TelemetryServiceConfig {
    fn default() -> Self {
        Self {
            num_threads: Some(4),
        }
    }
}
