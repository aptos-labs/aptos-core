// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct FailpointConf {
    pub name: String,
    pub actions: String,
}
