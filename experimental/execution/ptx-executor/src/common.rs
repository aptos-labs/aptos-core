// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

//! TODO(aldenhu): doc

use velor_types::state_store::state_key::StateKey;

pub(crate) type TxnIdx = usize;

pub(crate) type VersionedKey = (StateKey, TxnIdx);

pub(crate) const BASE_VERSION: TxnIdx = TxnIdx::MAX;
pub(crate) const EXPECTANT_BLOCK_SIZE: usize = 10_000;
pub(crate) const EXPECTANT_BLOCK_KEYS: usize = 100_000;

pub(crate) use hashbrown::{hash_map::Entry, HashMap, HashSet};
// pub(crate) use std::collections::{hash_map::Entry, HashMap, HashSet};

pub(crate) trait VersionedKeyHelper {
    fn key(&self) -> &StateKey;

    #[allow(unused)]
    fn txn_idx(&self) -> TxnIdx;

    fn txn_idx_shifted(&self) -> TxnIdx;
}

impl VersionedKeyHelper for VersionedKey {
    fn key(&self) -> &StateKey {
        let (key, _idx) = self;
        key
    }

    fn txn_idx(&self) -> TxnIdx {
        let (_key, idx) = self;
        *idx
    }

    fn txn_idx_shifted(&self) -> TxnIdx {
        let (_key, idx) = self;
        if *idx == BASE_VERSION {
            0
        } else {
            *idx + 1
        }
    }
}
