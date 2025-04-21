// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum BlockEpiloguePayload {
    V0 {
        block_id: HashValue,
        block_end_info: BlockEndInfo,
    },
}

impl BlockEpiloguePayload {
    pub fn try_as_block_end_info(&self) -> Option<&BlockEndInfo> {
        match self {
            BlockEpiloguePayload::V0 { block_end_info, .. } => Some(block_end_info),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockEndInfo {
    V0 {
        /// Whether block gas limit was reached
        block_gas_limit_reached: bool,
        /// Whether block output limit was reached
        block_output_limit_reached: bool,
        /// Total gas_units block consumed
        block_effective_block_gas_units: u64,
        /// Total output size block produced
        block_approx_output_size: u64,
    },
}

impl BlockEndInfo {
    pub fn limit_reached(&self) -> bool {
        match self {
            Self::V0 {
                block_gas_limit_reached,
                block_output_limit_reached,
                ..
            } => *block_gas_limit_reached || *block_output_limit_reached,
        }
    }

    pub fn block_effective_gas_units(&self) -> u64 {
        match self {
            Self::V0 {
                block_effective_block_gas_units,
                ..
            } => *block_effective_block_gas_units,
        }
    }
}
