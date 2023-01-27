// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::on_chain_config::OnChainConfig;
use serde::{Deserialize, Serialize};

/// The feature flags define in the Move source. This must stay aligned with the constants there.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_camel_case_types)]
pub enum FeatureFlag {
    CODE_DEPENDENCY_CHECK = 1,
    TREAT_FRIEND_AS_PRIVATE = 2,
    VM_BINARY_FORMAT_V6 = 5,
    GENERIC_GROUP_BASIC_OPERATIONS = 9,
    BLS12_381_GROUPS = 10,
    SHA256_TO_GROUP = 11,
}

/// Representation of features on chain as a bitset.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Features {
    #[serde(with = "serde_bytes")]
    pub features: Vec<u8>,
}

impl Default for Features {
    fn default() -> Self {
        Features {
            features: vec![0b00100000],
        }
    }
}

impl OnChainConfig for Features {
    const MODULE_IDENTIFIER: &'static str = "features";
    const TYPE_IDENTIFIER: &'static str = "Features";
}

impl Features {
    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        let val = flag as u64;
        let byte_index = (val / 8) as usize;
        let bit_mask = 1 << (val % 8);
        byte_index < self.features.len() && (self.features[byte_index] & bit_mask != 0)
    }

    pub fn enable(&mut self, flag: FeatureFlag) {
        let bit_id = flag as usize;
        let byte_id = bit_id / 8;
        if self.features.len() < byte_id + 1 {
            self.features.resize(byte_id + 1, 0);
        }
        self.features[byte_id] |= 1 << (bit_id % 8);

    }
}

// --------------------------------------------------------------------------------------------
// Code Publishing
