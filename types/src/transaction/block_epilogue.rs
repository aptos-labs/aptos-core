// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::{state_key::StateKey, state_slot::StateSlot};
use aptos_crypto::HashValue;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Debug};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum BlockEpiloguePayload {
    V0 {
        block_id: HashValue,
        block_end_info: BlockEndInfo,
    },
    V1 {
        block_id: HashValue,
        block_end_info: BlockEndInfoExt,
        fee_distribution: FeeDistribution,
    },
}

impl BlockEpiloguePayload {
    pub fn try_as_block_end_info(&self) -> Option<&BlockEndInfo> {
        match self {
            BlockEpiloguePayload::V0 { block_end_info, .. } => Some(block_end_info),
            BlockEpiloguePayload::V1 { block_end_info, .. } => Some(&block_end_info.inner),
        }
    }

    pub fn try_get_slots_to_make_hot(&self) -> Option<&BTreeMap<StateKey, StateSlot>> {
        match self {
            Self::V0 { .. } => None,
            Self::V1 { block_end_info, .. } => Some(&block_end_info.to_make_hot),
        }
    }

    pub fn try_get_keys_to_evict(&self) -> Option<&BTreeMap<StateKey, StateSlot>> {
        match self {
            Self::V0 { .. } => None,
            Self::V1 { block_end_info, .. } => Some(&block_end_info.to_evict),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum FeeDistribution {
    V0 {
        // Validator index -> Octa
        amount: BTreeMap<u64, u64>,
    },
}

impl FeeDistribution {
    pub fn new(amount: BTreeMap<u64, u64>) -> Self {
        Self::V0 { amount }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
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
    pub fn new_empty() -> Self {
        Self::V0 {
            block_gas_limit_reached: false,
            block_output_limit_reached: false,
            block_effective_block_gas_units: 0,
            block_approx_output_size: 0,
        }
    }

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TBlockEndInfoExt<Key: Debug + Ord> {
    inner: BlockEndInfo,
    to_make_hot: BTreeMap<Key, StateSlot>,
    to_evict: BTreeMap<Key, StateSlot>,
}

pub type BlockEndInfoExt = TBlockEndInfoExt<StateKey>;

impl<Key: Debug + Ord> TBlockEndInfoExt<Key> {
    pub fn new_empty() -> Self {
        Self {
            inner: BlockEndInfo::new_empty(),
            to_make_hot: BTreeMap::new(),
            to_evict: BTreeMap::new(),
        }
    }

    pub fn new(
        inner: BlockEndInfo,
        to_make_hot: BTreeMap<Key, StateSlot>,
        to_evict: BTreeMap<Key, StateSlot>,
    ) -> Self {
        Self {
            inner,
            to_make_hot,
            to_evict,
        }
    }

    pub fn to_persistent(&self) -> BlockEndInfo {
        self.inner.clone()
    }
}

impl<Key> Serialize for TBlockEndInfoExt<Key>
where
    Key: Debug + Ord,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, Key> Deserialize<'de> for TBlockEndInfoExt<Key>
where
    Key: Debug + Ord,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = BlockEndInfo::deserialize(deserializer)?;
        Ok(Self::new(inner, BTreeMap::new(), BTreeMap::new()))
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl<Key: Debug + Ord> Arbitrary for TBlockEndInfoExt<Key> {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        // TODO(HotState): it's used in db tests (encode/decode), so we need to make sure that
        // serializing the data and then deserializing it reproduces the original value.
        any::<BlockEndInfo>()
            .prop_map(|inner| Self::new(inner, BTreeMap::new(), BTreeMap::new()))
            .boxed()
    }
}
