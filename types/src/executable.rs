// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::state_store::state_key::{inner::StateKeyInner, StateKey};
use aptos_crypto::HashValue;

#[derive(PartialEq, Eq, Debug)]
pub enum ExecutableDescriptor {
    /// Possibly speculative, based on code published during the block.
    Published(HashValue),

    /// Based on code published (and committed) in previous blocks.
    Storage,
}

pub trait ModulePath {
    fn is_module_path(&self) -> bool;
}

impl ModulePath for StateKey {
    fn is_module_path(&self) -> bool {
        matches!(self.inner(), StateKeyInner::AccessPath(ap) if ap.is_code())
    }
}

/// For now we will handle the VM code cache / arena memory consumption on the
/// executor side, likely naively in the beginning (e.g. flushing after a threshold).
/// For the executor to manage memory consumption, executables should provide size.
/// Note: explore finer-grained eviction mechanisms, e.g. LRU-based, or having
/// different ownership for the arena / memory.
pub trait Executable: Clone + Send + Sync {
    fn size_bytes(&self) -> usize;
}

#[derive(Clone)]
pub struct ExecutableTestType(());

impl Executable for ExecutableTestType {
    fn size_bytes(&self) -> usize {
        0
    }
}
