// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::collections::BTreeSet;

/// Deterministic, hotness-only view of a committed transaction's state accesses, used to feed the
/// block hot-state promotion accumulator.
///
/// This is deliberately separate from block-executor's conflict-oriented `ReadWriteSummary`:
/// conflict accounting stays a Block-STM concern (driven by speculative captured reads), while
/// hotness observation is a VM/execution-boundary concern. Two consequences follow:
///   - Reads include metadata/exists/size accesses, which the conflict summary excludes, because
///     hot-state KV can accelerate those access paths too.
///   - Resource-group accesses collapse to the group key (conflict accounting may stay
///     tag-granular).
///
/// All entries are concrete state keys (`StateKey` in production) in a deterministic (sorted)
/// container. It is generic over `Key` only so block-executor test harnesses can use mock keys;
/// delayed-field ids do not map to hot-state KV keys and are excluded from both sets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TxnHotnessSummary<Key> {
    reads: BTreeSet<Key>,
    writes: BTreeSet<Key>,
}

impl<Key: Ord> TxnHotnessSummary<Key> {
    pub fn new(reads: BTreeSet<Key>, writes: BTreeSet<Key>) -> Self {
        Self { reads, writes }
    }

    pub fn keys_read(&self) -> impl Iterator<Item = &Key> {
        self.reads.iter()
    }

    pub fn keys_written(&self) -> impl Iterator<Item = &Key> {
        self.writes.iter()
    }
}
