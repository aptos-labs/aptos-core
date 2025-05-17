// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{InputOutputKey, ReadWriteSummary};
use aptos_logger::error;
use aptos_types::{
    state_store::{state_slot::StateSlot, TStateView},
    transaction::{BlockExecutableTransaction, Version},
};
use std::collections::BTreeMap;

pub struct BlockHotStateOpAccumulator<'base_view, Txn: BlockExecutableTransaction, BaseView> {
    first_version: Version,
    base_view: &'base_view BaseView,
    to_make_hot: BTreeMap<Txn::Key, StateSlot>,
    keys_written: hashbrown::HashSet<Txn::Key>,
}

impl<'base_view, Txn, BaseView> BlockHotStateOpAccumulator<'base_view, Txn, BaseView>
where
    Txn: BlockExecutableTransaction,
    BaseView: TStateView<Key = Txn::Key>,
{
    /// TODO(HotState): make configurable, make on-chain config
    const MAX_PROMOTIONS_PER_BLOCK: usize = 1024 * 10;
    const REFRESH_INTERVAL_VERSIONS: usize = 1_000_000;

    pub fn new(base_view: &'base_view BaseView) -> Self {
        Self {
            first_version: base_view.next_version(),
            base_view,
            to_make_hot: BTreeMap::new(),
            keys_written: hashbrown::HashSet::new(),
        }
    }

    pub fn add_transaction(&mut self, rw_summary: &ReadWriteSummary<Txn>) {
        for write_key in rw_summary.writes.iter() {
            match write_key {
                InputOutputKey::Resource(key) | InputOutputKey::Group(key, _) => {
                    self.to_make_hot.remove(key);
                    self.keys_written.get_or_insert_owned(key);
                },
                InputOutputKey::DelayedField(_id) => {
                    // TODO(HotState): revisit -- hotness can't change for a in-place change,
                    //                 otherwise it's a problem when we charge differently for hot
                    //                 and cold reads
                },
            }
        }

        for read_key in rw_summary.reads.iter() {
            match read_key {
                InputOutputKey::Resource(key) | InputOutputKey::Group(key, _) => {
                    if self.to_make_hot.len() >= Self::MAX_PROMOTIONS_PER_BLOCK {
                        continue;
                    }
                    if self.to_make_hot.contains_key(key) {
                        continue;
                    }
                    if self.keys_written.contains(key) {
                        continue;
                    }
                    let slot = self
                        .base_view
                        .get_state_slot(key)
                        .expect("base_view.get_slot() failed.");
                    let make_hot = match slot.hot_since_version_opt() {
                        None => true,
                        Some(hot_since_version) => {
                            if hot_since_version >= self.first_version {
                                error!("Unexpected: hot_since_version > block first version");
                                false
                            } else {
                                hot_since_version + Self::REFRESH_INTERVAL_VERSIONS as Version
                                    >= self.first_version
                            }
                        },
                    };
                    if make_hot {
                        self.to_make_hot.insert(key.clone(), slot);
                    }
                },
                InputOutputKey::DelayedField(_id) => {},
            }
        }
    }

    pub fn get_slots_to_make_hot(&self) -> BTreeMap<Txn::Key, StateSlot> {
        self.to_make_hot.clone()
    }
}
