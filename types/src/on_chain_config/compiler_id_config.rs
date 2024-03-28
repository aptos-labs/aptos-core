// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// Defines the disallow list of the compiler ids for the blockchain..
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DisallowList {
    pub disallow_list: Vec<Vec<u8>>,
}

impl OnChainConfig for DisallowList {
    const MODULE_IDENTIFIER: &'static str = "compiler_id_config";
    const TYPE_IDENTIFIER: &'static str = "DisallowList";
}
