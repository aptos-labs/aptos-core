// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::types::{InputOutputKey, ReadWriteSummary};
use aptos_types::{
    state_store::TStateView,
    transaction::{block_epilogue::THotStateOp, BlockExecutableTransaction},
};
use std::collections::BTreeSet;

pub struct BlockHotStateOpAccumulator<'base_view, Txn: BlockExecutableTransaction, BaseView> {
    base_view: &'base_view BaseView,
    make_hot: BTreeSet<Txn::Key>,
    writes: hashbrown::HashSet<Txn::Key>,
}

impl<'base_view, Txn, BaseView> BlockHotStateOpAccumulator<'base_view, Txn, BaseView>
where
    Txn: BlockExecutableTransaction,
    BaseView: TStateView<Key = Txn::Key>,
{
    /// TODO(HotState): make on-chain config
    const MAX_PROMOTIONS_PER_BLOCK: usize = 1024 * 10;

    pub fn new(base_view: &'base_view BaseView) -> Self {
        Self {
            base_view,
            make_hot: BTreeSet::new(),
            writes: hashbrown::HashSet::new(),
        }
    }

    pub fn add_transaction(&mut self, rw_summary: &ReadWriteSummary<Txn>) {
        for write_key in rw_summary.writes.iter() {
            match write_key {
                InputOutputKey::Resource(key) | InputOutputKey::Group(key, _) => {
                    self.make_hot.remove(key);
                    self.writes.get_or_insert_owned(key);
                },
                InputOutputKey::DelayedField(_id) => {},
            }
        }

        for read_key in rw_summary.reads.iter() {
            match read_key {
                InputOutputKey::Resource(key) | InputOutputKey::Group(key, _) => {
                    if self.make_hot.len() >= Self::MAX_PROMOTIONS_PER_BLOCK {
                        continue;
                    }
                    if self.make_hot.contains(key) {
                        continue;
                    }
                    if self.writes.contains(key) {
                        continue;
                    }
                    // FIXME(aldenhu): expose access time and HotNonExistent
                    // FIXME(aldenhu): insert only for cold and stale keys
                    self.base_view
                        .get_state_value(key)
                        .expect("base_view.get_ failed.");
                    self.make_hot.insert(key.clone());
                },
                InputOutputKey::DelayedField(_id) => {},
            }
        }
    }

    pub fn get_hot_state_ops(&self) -> Vec<THotStateOp<Txn::Key>> {
        self.make_hot
            .iter()
            .cloned()
            .map(THotStateOp::MakeHot)
            .collect()
    }
}
