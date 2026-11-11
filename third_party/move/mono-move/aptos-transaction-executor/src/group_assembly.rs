// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Experiment switch: where resource-group blobs are assembled.

use mono_move_core::POLL_INTERVAL;
use std::sync::{atomic::AtomicU64, OnceLock};

/// Gas units between interrupt polls, `MONO_POLL_INTERVAL` or
/// [`POLL_INTERVAL`] when unset. 0 turns polling off.
pub fn poll_interval() -> u64 {
    static INTERVAL: OnceLock<u64> = OnceLock::new();
    *INTERVAL.get_or_init(|| match std::env::var("MONO_POLL_INTERVAL") {
        Ok(value) => value
            .parse()
            .unwrap_or_else(|_| panic!("invalid MONO_POLL_INTERVAL {value:?}")),
        Err(_) => POLL_INTERVAL,
    })
}

/// Where a transaction's resource-group writes become group blobs, chosen by
/// the `MONO_GROUP_ASSEMBLY` environment variable.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GroupAssembly {
    /// Each transaction assembles its groups at commit. Unset selects this.
    PerTxn,
    /// Transactions emit member changes, and the executor assembles every
    /// group at the end of the block.
    BlockEnd,
    /// Like [`GroupAssembly::BlockEnd`], and Block-STM versions each member
    /// under its own key instead of inside a group.
    BlockEndFlat,
    /// Block-STM versions each member under its own key, and each transaction
    /// assembles its groups at commit from a chain of links at the groups'
    /// slots.
    Chained,
}

impl GroupAssembly {
    pub fn get() -> Self {
        static MODE: OnceLock<GroupAssembly> = OnceLock::new();
        *MODE.get_or_init(|| match std::env::var("MONO_GROUP_ASSEMBLY").as_deref() {
            Err(_) | Ok("") | Ok("per_txn") => Self::PerTxn,
            Ok("block_end") => Self::BlockEnd,
            Ok("block_end_flat") => Self::BlockEndFlat,
            Ok("chained") => Self::Chained,
            Ok(other) => panic!("unknown MONO_GROUP_ASSEMBLY value {other:?}"),
        })
    }

    /// Whether transactions leave group assembly to the end of the block.
    pub fn at_block_end(self) -> bool {
        match self {
            Self::PerTxn | Self::Chained => false,
            Self::BlockEnd | Self::BlockEndFlat => true,
        }
    }

    /// Whether Block-STM versions group members as plain keys.
    pub fn flat_members(self) -> bool {
        match self {
            Self::PerTxn | Self::BlockEnd => false,
            Self::BlockEndFlat | Self::Chained => true,
        }
    }
}

/// CPU time spent assembling groups at commit under [`GroupAssembly::PerTxn`]
/// or [`GroupAssembly::Chained`], summed over all workers.
pub static PER_TXN_ASSEMBLY_NANOS: AtomicU64 = AtomicU64::new(0);

/// Groups assembled at commit under [`GroupAssembly::PerTxn`] or
/// [`GroupAssembly::Chained`].
pub static PER_TXN_ASSEMBLY_GROUPS: AtomicU64 = AtomicU64::new(0);
