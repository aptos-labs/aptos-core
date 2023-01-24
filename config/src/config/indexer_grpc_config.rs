// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct IndexerGrpcConfig {
    pub enabled: bool,
    pub address: String,
    pub processor_task_count: u16,
    pub processor_batch_size: u16,
    pub output_batch_size: u16,
}
