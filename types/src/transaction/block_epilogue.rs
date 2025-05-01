// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_key::StateKey;
use aptos_crypto::HashValue;
use derive_more::Deref;
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
            BlockEndInfo::V0 {
                block_gas_limit_reached,
                block_output_limit_reached,
                ..
            } => *block_gas_limit_reached || *block_output_limit_reached,
        }
    }

    pub fn block_effective_gas_units(&self) -> u64 {
        match self {
            BlockEndInfo::V0 {
                block_effective_block_gas_units,
                ..
            } => *block_effective_block_gas_units,
        }
    }
}

/// Wrapper type to temporarily host the hot_state_ops which will not serialize until
/// the hot state is made entirely deterministic
#[derive(Debug, Deref)]
pub struct TBlockEndInfoExt<Key: Debug> {
    #[deref]
    inner: BlockEndInfo,
    /// Changes to the hot state, with regard to keys that are not written to by the user
    /// transactions.
    ///
    /// TODO: once hot state is deterministic across all nodes, add BlockEndInfo::V1 and serialize the
    ///       ops there.
    hot_state_ops: Vec<THotStateOp<Key>>,
}

pub type BlockEndInfoExt = TBlockEndInfoExt<StateKey>;

impl<Key: Debug> TBlockEndInfoExt<Key> {
    pub fn new_empty() -> Self {
        Self {
            inner: BlockEndInfo::V0 {
                block_gas_limit_reached: false,
                block_output_limit_reached: false,
                block_effective_block_gas_units: 0,
                block_approx_output_size: 0,
            },
            hot_state_ops: vec![],
        }
    }

    pub fn new(inner: BlockEndInfo, hot_state_ops: Vec<THotStateOp<Key>>) -> Self {
        Self {
            inner,
            hot_state_ops,
        }
    }

    pub fn to_persistent(&self) -> BlockEndInfo {
        self.inner.clone()
    }

    pub fn hot_state_ops(&self) -> &[THotStateOp<Key>] {
        &self.hot_state_ops
    }
}

#[derive(Debug)]
pub enum THotStateOp<Key: Debug> {
    /// TODO(HotState): revisit
    /// Until the cold state is exclusive of the hot state items and further contemplation,
    /// MakeHot can mean either promotion from cold, refresh access time in hot or make
    /// HotNonExistent.
    MakeHot(Key),
    /// TODO(HotState): Not used for now, once speculative LRU is in, emit evictions via MakeCold.
    ///                 Also, maybe distinguish eviction of HotNonExistent
    MakeCold(Key),
}

pub type HotStateOp = THotStateOp<StateKey>;
