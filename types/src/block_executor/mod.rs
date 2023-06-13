// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod partitioner;

use crate::{executable::ModulePath, write_set::TransactionWrite};
use std::{fmt::Debug, hash::Hash};

/// Trait that defines a transaction type that can be executed by the block executor. A transaction
/// transaction will write to a key value storage as their side effect.
pub trait BlockExecutorTransaction: Sync + Send + 'static {
    type Key: PartialOrd + Ord + Send + Sync + Clone + Hash + Eq + ModulePath + Debug;
    type Value: Send + Sync + TransactionWrite;
}
