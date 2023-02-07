// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::data_cache::{Cache, CachedData, OutputData, OutputDataArced, Readable};
use aptos_types::{contract_event::ContractEvent, state_store::state_key::StateKey};
use std::{collections::BTreeMap, io::Read, sync::Arc};

#[derive(Debug)]
pub struct ChangeSet {
    delta_change_set: ChangeSetContainer<DeltaChange>,
    write_change_set: ChangeSetContainer<WriteChange<OutputData>>,
    events: Vec<ContractEvent>,
}

/// Generic container which records changes fo each state key.
#[derive(Debug)]
pub struct ChangeSetContainer<T> {
    inner: BTreeMap<StateKey, T>,
}

impl<T> ChangeSetContainer<T> {
    pub fn empty() -> Self {
        ChangeSetContainer {
            inner: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn get(&self, key: &StateKey) -> Option<&T> {
        self.inner.get(key)
    }

    pub fn insert(&mut self, delta: (StateKey, T)) {
        self.inner.insert(delta.0, delta.1);
    }

    pub fn remove(&mut self, key: &StateKey) -> Option<T> {
        self.inner.remove(key)
    }

    #[inline]
    pub fn iter(&self) -> ::std::collections::btree_map::Iter<'_, StateKey, T> {
        self.into_iter()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<StateKey, T> {
        &mut self.inner
    }
}

impl<'a, T> IntoIterator for &'a ChangeSetContainer<T> {
    type IntoIter = ::std::collections::btree_map::Iter<'a, StateKey, T>;
    type Item = (&'a StateKey, &'a T);

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<T> ::std::iter::IntoIterator for ChangeSetContainer<T> {
    type IntoIter = ::std::collections::btree_map::IntoIter<StateKey, T>;
    type Item = (StateKey, T);

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

/// An item write for some state key. item can be created, modified or deleted.
#[derive(Debug)]
pub enum WriteChange<T> {
    Creation(T),
    Modification(T),
    Deletion,
}

impl Clone for WriteChange<OutputDataArced> {
    fn clone(&self) -> Self {
        match self {
            WriteChange::Creation(data) => WriteChange::Creation(match data {
                OutputDataArced::Serialized(blob) => OutputDataArced::Serialized(Arc::clone(blob)),
                OutputDataArced::MoveValue(value) => OutputDataArced::MoveValue(Arc::clone(value)),
            }),
            WriteChange::Modification(data) => WriteChange::Modification(match data {
                OutputDataArced::Serialized(blob) => OutputDataArced::Serialized(Arc::clone(blob)),
                OutputDataArced::MoveValue(value) => OutputDataArced::MoveValue(Arc::clone(value)),
            }),
            WriteChange::Deletion => WriteChange::Deletion,
        }
    }
}

impl Readable for WriteChange<OutputDataArced> {
    fn read_ref(&self) -> Option<CachedData> {
        match self {
            WriteChange::Creation(data) | WriteChange::Modification(data) => Some(match data {
                OutputDataArced::Serialized(blob) => CachedData::Serialized(Arc::clone(blob)),
                OutputDataArced::MoveValue(value) => CachedData::MoveValue(Arc::clone(value)),
            }),
            WriteChange::Deletion => None,
        }
    }

    fn read(self) -> Option<CachedData> {
        match self {
            WriteChange::Creation(data) | WriteChange::Modification(data) => Some(match data {
                OutputDataArced::Serialized(blob) => CachedData::Serialized(blob),
                OutputDataArced::MoveValue(value) => CachedData::MoveValue(value),
            }),
            WriteChange::Deletion => None,
        }
    }
}

impl Cache for WriteChange<OutputData> {
    type Target = WriteChange<OutputDataArced>;

    fn cache(self) -> Self::Target {
        match self {
            WriteChange::Creation(data) => WriteChange::Creation(match data {
                OutputData::Serialized(blob) => OutputDataArced::Serialized(Arc::new(blob)),
                OutputData::MoveValue(value) => OutputDataArced::MoveValue(Arc::new(value)),
            }),
            WriteChange::Modification(data) => WriteChange::Modification(match data {
                OutputData::Serialized(blob) => OutputDataArced::Serialized(Arc::new(blob)),
                OutputData::MoveValue(value) => OutputDataArced::MoveValue(Arc::new(value)),
            }),
            WriteChange::Deletion => WriteChange::Deletion,
        }
    }
}

// impl Cache for WriteChange {
//     fn as_cached_data(&self) -> Option<CachedData> {
//         match self {
//             WriteChange::Creation(data) | WriteChange::Modification(data) => {
//                 Some(
//                     match data
//                      {

//                     }
//                 )
//             },
//             WriteChange::Deletion => None,
//         }
//     }
// }

/// A delta to be applied for some state key.
#[derive(Debug)]
pub enum DeltaChange {
    // TODO: Move delta op here?
}
