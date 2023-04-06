// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::delta_change_set::{deserialize, DeltaChangeSet};
use anyhow::bail;
use aptos_state_view::StateView;
use aptos_types::{
    resource::TransactionWrite,
    transaction::{ChangeSet, CheckChangeSet, TransactionOutput},
    write_set::{WriteOp, WriteSet, WriteSetMut},
};
use std::{collections::btree_map, sync::Arc};

/// Helpful trait for e.g. extracting u128 value out of TransactionWrite that we know is
/// for aggregator (i.e. if we have seen a DeltaOp for the same access path).
pub struct AggregatorValue(u128);

impl AggregatorValue {
    /// Returns None if the write doesn't contain a value (i.e deletion), and panics if
    /// the value raw bytes can't be deserialized into an u128.
    pub fn from_write(write: &dyn TransactionWrite) -> Option<Self> {
        let v = write.extract_raw_bytes();
        v.map(|bytes| Self(deserialize(&bytes)))
    }

    pub fn into(self) -> u128 {
        self.0
    }
}
