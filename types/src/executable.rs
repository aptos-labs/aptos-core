// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::AccessPath,
    state_store::state_key::{StateKey, StateKeyInner},
};
use aptos_crypto::HashValue;
use std::hash::Hash;

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum ExecutableDescriptor {
    /// Possibly speculative, based on code published during the block.
    Published(HashValue),

    /// Based on code published (and committed) in previous blocks.
    Storage,
}

pub trait ModulePath {
    fn module_path(&self) -> Option<AccessPath>;
}

impl ModulePath for StateKey {
    fn module_path(&self) -> Option<AccessPath> {
        if let StateKeyInner::AccessPath(ap) = self.inner() {
            if ap.is_code() {
                return Some(ap.clone());
            }
        }
        None
    }
}

/// For now we will handle the VM code cache / arena memory consumption on the
/// executor side, likely naively in the beginning (e.g. flushing after a threshold).
/// For the executor to manage memory consumption, executables should provide size.
/// Note: explore finer-grained eviction mechanisms, e.g. LRU-based, or having
/// different ownership for the arena / memory.
///
/// The provided executable type is assumed to be effciently clonable, and in the
/// beginning, would probably be an Arc<> containing the actual executable data.
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

/// Provided Executable type is assumed to be efficiently clonable
pub enum FetchedModule<X: Executable> {
    Blob(Vec<u8>),
    // TODO: compiled module when available to avoid deserialization.
    Executable(X),
}

/// View for the VM for interacting with the multi-versioned executable cache.
pub trait ExecutableView {
    type Key;
    type Executable: Executable;

    /// This is an optimization to bypass transactional semantics and share the
    /// executable (and all the useful work for producing it) as early as possible
    /// other txns / VM sessions. It is safe as storage-version module can't change,
    /// and o.w. the key is the (cryptographic) hash of the module blob.
    ///
    /// W.o. this, we would have to include executables in TransactionOutputExt.
    /// This may occur much later leading to work duplication (producing the same
    /// executable by other sessions) in the common case when the executable isn't
    /// based on the module published by the transaction itself.
    ///
    /// We currently make no assumption about where the actual executable is stored,
    /// i.e. the provided Executable here can be the actual data, or just meta-data.
    /// This gives us flexibility to implement the executable arena either on the
    /// executor / adapter / MVHashMap side, or within Move-VM (populated during
    /// executable construction). In the later case, Executable would be meta-data
    /// and we would have to support re-using the arena from the Move-VM. For now,
    /// the Executable is just an Arc containing the executable data, providing
    /// ownership on the executor side and simplifying re-using across the blocks.
    fn store_executable(
        &self,
        key: &Self::Key,
        descriptor: ExecutableDescriptor,
        executable: Self::Executable,
    );

    /// Returns either the blob of the module, that will need to be deserialized into
    /// CompiledModule and then made into an executable, or executable directly, if
    /// the executable corresponding to the latest published module was already stored.
    /// TODO: Return CompiledModule directly to avoid deserialization.
    fn fetch_module(
        &self,
        key: &Self::Key,
    ) -> anyhow::Result<(ExecutableDescriptor, FetchedModule<Self::Executable>)>;
}
