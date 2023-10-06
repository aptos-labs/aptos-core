// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use dashmap::{
    mapref::entry::Entry::{Occupied, Vacant},
    DashMap,
};
use std::{
    cmp::{Eq, Ordering},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub struct InstanceUniverse<'a, T: Eq + Hash> {
    instance_to_entry: DashMap<Arc<T>, Weak<InstanceUniverseTableEntry<'a, T>>>,
}

#[derive(Debug)]
pub struct InstanceUniverseTableEntry<'a, T: Eq + Hash> {
    ptr: Arc<T>,
    map: &'a DashMap<Arc<T>, Weak<InstanceUniverseTableEntry<'a, T>>>,
}

impl<'a, T: Eq + Hash> PartialEq for InstanceUniverseTableEntry<'a, T> {
    fn eq(&self, other: &Self) -> bool {
        // TODO: https://github.com/rust-lang/rust/issues/106447, also
        // TODO: should we allocate int ID and compare that?
        Arc::ptr_eq(&self.ptr, &other.ptr)
    }
}
impl<'a, T: Eq + Hash> Eq for InstanceUniverseTableEntry<'a, T> {}
impl<'a, T: Eq + Hash> Ord for InstanceUniverseTableEntry<'a, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        Arc::as_ptr(&self.ptr).cmp(&Arc::as_ptr(&other.ptr))
    }
}
impl<'a, T: Eq + Hash> PartialOrd for InstanceUniverseTableEntry<'a, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a, T: Eq + Hash> Hash for InstanceUniverseTableEntry<'a, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.ptr).hash(state);
    }
}

impl<'a, T: Eq + Hash> InstanceUniverseTableEntry<'a, T> {
    pub fn inner_ref(&self) -> &Arc<T> {
        &self.ptr
    }

    pub fn clone_inner(&self) -> Arc<T> {
        self.ptr.clone()
    }
}

impl<'a, T: Eq + Hash> Drop for InstanceUniverseTableEntry<'a, T> {
    fn drop(&mut self) {
        self.map.remove(&self.ptr);
    }
}

pub type InstanceUID<'a, T> = Arc<InstanceUniverseTableEntry<'a, T>>;

impl<'a, T: Eq + Hash> Default for InstanceUniverse<'a, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T: Eq + Hash> InstanceUniverse<'a, T> {
    pub fn new() -> Self {
        Self {
            instance_to_entry: DashMap::new(),
        }
    }

    pub fn get(&'a self, instance: T) -> InstanceUID<'a, T> {
        let instance_arc = Arc::new(instance);

        loop {
            if let Some(weak) = self.instance_to_entry.get(&instance_arc) {
                if let Some(compressed) = weak.upgrade() {
                    return compressed;
                }
            } else {
                let inner = InstanceUniverseTableEntry {
                    ptr: instance_arc.clone(),
                    map: &self.instance_to_entry,
                };
                let ret = Arc::new(inner);
                self.instance_to_entry
                    .insert(instance_arc, Arc::downgrade(&ret));
                return ret;
            }
            // match self.instance_to_entry.entry(instance_arc.clone()) {
            //     Occupied(entry) => {
            //         if let Some(compressed) = Weak::upgrade(entry.get()) {
            //             return compressed;
            //         }
            //     },
            //     Vacant(entry) => {
            //         let inner = InstanceUniverseTableEntry {
            //             ptr: instance_arc.clone(),
            //             map: &self.instance_to_entry,
            //         };
            //         let ret = Arc::new(inner);
            //         entry.insert(Arc::downgrade(&ret));
            //         return ret;
            //     },
            // }
        }
    }
}
