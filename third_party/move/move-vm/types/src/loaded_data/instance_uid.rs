// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use dashmap::{
    mapref::entry::Entry::{Occupied, Vacant},
    DashMap,
};
use derivative::Derivative;
use std::{
    cmp::{Eq, Ordering},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

// TODO: add tests.
// TODO: break into files.
// TODO: micro benchmarks.

pub struct GlobalCache<'a, T: Eq + Hash> {
    instance_to_compressed: DashMap<Arc<T>, Weak<InstanceUIDInner<'a, T>>>,
}

// Type wrapping Arc with overloaded methods to consider the underlying pointer.
#[derive(Debug)]
struct ArcKeyByPtr<T> {
    key: Arc<T>,
}

impl<T: Eq + Hash> ArcKeyByPtr<T> {
    fn new(key: Arc<T>) -> Self {
        Self { key }
    }
}

impl<T: Eq + Hash> PartialEq for ArcKeyByPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        // TODO: https://github.com/rust-lang/rust/issues/106447, also
        // TODO: should we allocate int ID and compare that?
        Arc::ptr_eq(&self.key, &other.key)
    }
}
impl<T: Eq + Hash> Eq for ArcKeyByPtr<T> {}
impl<T: Eq + Hash> Ord for ArcKeyByPtr<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        Arc::as_ptr(&self.key).cmp(&Arc::as_ptr(&other.key))
    }
}
impl<T: Eq + Hash> PartialOrd for ArcKeyByPtr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T: Eq + Hash> Hash for ArcKeyByPtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.key).hash(state);
    }
}

#[derive(Derivative)]
#[derivative(Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub struct InstanceUIDInner<'a, T: Eq + Hash> {
    key: ArcKeyByPtr<T>,
    #[derivative(
        Debug = "ignore",
        Hash = "ignore",
        PartialEq = "ignore",
        PartialOrd = "ignore",
        Ord = "ignore"
    )]
    map: &'a DashMap<Arc<T>, Weak<InstanceUIDInner<'a, T>>>,
}

impl<'a, T: Eq + Hash> InstanceUIDInner<'a, T> {
    pub fn uncompressed(&self) -> &Arc<T> {
        &self.key.key
    }

    pub fn into_uncompressed(&self) -> Arc<T> {
        self.key.key.clone()
    }
}

impl<'a, T: Eq + Hash> Drop for InstanceUIDInner<'a, T> {
    fn drop(&mut self) {
        self.map.remove(&self.key.key);
    }
}

// TODO(zi) drop 'a assuming it's static....
pub type InstanceUID<'a, T> = Arc<InstanceUIDInner<'a, T>>;

impl<'a, T: Eq + Hash> GlobalCache<'a, T> {
    pub fn new() -> Self {
        Self {
            instance_to_compressed: DashMap::new(),
        }
    }

    pub fn compress(&'a self, instance: T) -> InstanceUID<'a, T> {
        let instance_arc = Arc::new(instance);

        loop {
            match self.instance_to_compressed.entry(instance_arc.clone()) {
                Occupied(entry) => {
                    if let Some(compressed) = Weak::upgrade(entry.get()) {
                        return compressed;
                    }
                },
                Vacant(entry) => {
                    let inner = InstanceUIDInner {
                        key: ArcKeyByPtr::new(instance_arc),
                        map: &self.instance_to_compressed,
                    };
                    let ret = Arc::new(inner);
                    entry.insert(Arc::downgrade(&ret));
                    return ret;
                },
            }
        }
    }
}

// user has to define
// static GLOBAL_CACHE: Lazy<GlobalCache<usize>> = Lazy::new(|| GlobalCache::new());
// for the data structure.
// Then call compressed on it, and start using InstanceUID<T>
