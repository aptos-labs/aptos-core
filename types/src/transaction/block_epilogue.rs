// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    state_store::{state_key::StateKey, state_slot::StateSlot, NUM_STATE_SHARDS},
    transaction::TransactionOutput,
    write_set::{HotStateOp, WriteSet},
};
use anyhow::Result;
use aptos_crypto::HashValue;
use derive_more::Deref;
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
            BlockEpiloguePayload::V1 { block_end_info, .. } => Some(block_end_info),
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
#[derive(Clone, Debug, Deref, Eq, PartialEq)]
pub struct TBlockEndInfoExt<Key: Debug> {
    #[deref]
    inner: BlockEndInfo,
    /// TODO(HotState): remove
    /// Changes to the hot state.
    /// n.b. only involves keys that are not written to by the user transactions.
    /// TODO(HotState): add evictions
    /// TODO(HotState): once hot state is deterministic across all nodes, add BlockEndInfo::V1 and
    ///                 serialize the promoted and evicted keys in the transaction.
    slots_to_make_hot: BTreeMap<Key, StateSlot>,
    slots_to_evict: [Vec<Key>; NUM_STATE_SHARDS],
}

pub type BlockEndInfoExt = TBlockEndInfoExt<StateKey>;

impl<Key: Debug> TBlockEndInfoExt<Key> {
    pub fn new_empty() -> Self {
        Self {
            inner: BlockEndInfo::new_empty(),
            slots_to_make_hot: BTreeMap::new(),
            slots_to_evict: [(); NUM_STATE_SHARDS].map(|_| Vec::new()),
        }
    }

    pub fn new(
        inner: BlockEndInfo,
        slots_to_make_hot: BTreeMap<Key, StateSlot>,
        slots_to_evict: [Vec<Key>; NUM_STATE_SHARDS],
    ) -> Self {
        Self {
            inner,
            slots_to_make_hot,
            slots_to_evict,
        }
    }

    pub fn to_persistent(&self) -> BlockEndInfo {
        self.inner.clone()
    }
}

impl<Key> Serialize for TBlockEndInfoExt<Key>
where
    Key: Debug,
{
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de, Key> Deserialize<'de> for TBlockEndInfoExt<Key>
where
    Key: Debug,
{
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let inner = BlockEndInfo::deserialize(deserializer)?;
        Ok(Self {
            inner,
            slots_to_make_hot: BTreeMap::new(),
            slots_to_evict: [(); NUM_STATE_SHARDS].map(|_| Vec::new()),
        })
    }
}

#[cfg(any(test, feature = "fuzzing"))]
impl<Key> proptest::arbitrary::Arbitrary for TBlockEndInfoExt<Key>
where
    Key: Debug,
{
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        any::<BlockEndInfo>()
            .prop_map(|inner| Self {
                inner,
                slots_to_make_hot: BTreeMap::new(),
                slots_to_evict: [(); NUM_STATE_SHARDS].map(|_| Vec::new()),
            })
            .boxed()
    }
}

impl BlockEndInfoExt {
    pub fn to_transaction_output(&self) -> Result<TransactionOutput> {
        let write_ops = self
            .slots_to_make_hot
            .iter()
            .map(|(key, slot)| Ok((key.clone(), HotStateOp::make_hot(slot.clone()))))
            .collect::<Result<_>>()?;
        Ok(TransactionOutput::new_success_with_write_set(
            WriteSet::Hotness(write_ops),
        ))
    }
}
