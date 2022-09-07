// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// Defines the features enabled on-chain.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Features {
    #[serde(with = "serde_bytes")]
    pub features: Vec<u8>,
}

impl OnChainConfig for Features {
    const MODULE_IDENTIFIER: &'static str = "features";
    const TYPE_IDENTIFIER: &'static str = "Features";
}

impl Features {
    pub fn is_enabled(&self, flag: u64) -> bool {
        let byte_index = (flag / 8) as usize;
        let bit_mask = 1 << (flag % 8);
        byte_index < self.features.len() && (self.features[byte_index] & bit_mask != 0)
    }
}

// --------------------------------------------------------------------------------------------
// Code Publishing

pub const CODE_DEPENDENCY_CHECK: u64 = 1;
