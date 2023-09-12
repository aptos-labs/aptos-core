// Copyright © Aptos Foundation

use anyhow::Result;
use aptos_aggregator::{
    delta_change_set::{serialize, DeltaOp},
    transaction::AggregatorValue,
};
use aptos_state_view::{in_memory_state_view::InMemoryStateView, StateViewId, TStateView};
use aptos_types::{
    serde_helper::vec_bytes::deserialize,
    state_store::{state_storage_usage::StateStorageUsage, state_value::StateValue},
    write_set::TransactionWrite,
};
use dashmap::{mapref::entry::Entry, DashMap};
use std::{cell::RefCell, hash::Hash, marker::PhantomData, ops::Deref};

pub trait WritableStateView: TStateView {
    type Value;

    fn write_value(&self, k: Self::Key, v: Self::Value);

    fn write_u128(&self, k: Self::Key, v: u128);

    /// Applies the delta to the value associated with key `k`.
    /// Returns the new value of the accumulator or an error in case of an overflow.
    fn apply_delta(&self, k: &Self::Key, delta: &DeltaOp) -> Result<u128>;

    fn as_state_view(&self) -> NonWritableView<Self> {
        NonWritableView(&self)
    }
}

/// Can be used to strip a WritableStateView of the write capabilities.
pub struct NonWritableView<'a, WV: WritableStateView + ?Sized>(&'a WV);

impl<'a, WV: WritableStateView> TStateView for NonWritableView<'a, WV> {
    type Key = WV::Key;

    fn id(&self) -> StateViewId {
        self.0.id()
    }

    fn get_state_value_u128(&self, state_key: &Self::Key) -> Result<Option<u128>> {
        self.0.get_state_value_u128(state_key)
    }

    fn get_state_value_bytes(&self, state_key: &Self::Key) -> Result<Option<Vec<u8>>> {
        self.0.get_state_value_bytes(state_key)
    }

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>> {
        self.0.get_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.0.get_usage()
    }

    fn as_in_memory_state_view(&self) -> InMemoryStateView {
        self.0.as_in_memory_state_view()
    }
}

/// Wraps any `TStateView` and records all read operations from a single thread.
pub struct ReadSetCapturingStateView<'view, K, S> {
    base_view: &'view S,
    captured_reads: RefCell<Vec<K>>,
}

impl<'view, K: Clone, S> ReadSetCapturingStateView<'view, K, S> {
    pub fn new(base_view: &'view S) -> Self {
        Self {
            base_view,
            captured_reads: Vec::new().into(),
        }
    }

    pub fn with_capacity(base_view: &'view S, capacity: usize) -> Self {
        Self {
            base_view,
            captured_reads: Vec::with_capacity(capacity).into(),
        }
    }

    /// Clears the information about the captured read set without deallocating the memory.
    pub fn clear_read_set(&self) {
        self.captured_reads.borrow_mut().clear();
    }

    pub fn get_read_set(&self) -> impl Deref<Target = Vec<K>> + '_ {
        self.captured_reads.borrow()
    }

    pub fn clone_read_set(&self) -> Vec<K> {
        self.captured_reads.borrow().clone()
    }
}

impl<'view, K, S> TStateView for ReadSetCapturingStateView<'view, K, S>
where
    S: TStateView<Key = K>,
    K: Clone,
{
    type Key = S::Key;

    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>> {
        self.captured_reads.borrow_mut().push(state_key.clone());
        self.base_view.get_state_value(state_key)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}

/// Wraps any `TStateView` and allows making concurrent writes on top of it.
pub struct DashMapStateView<K, V, S> {
    base_view: S,
    updates: DashMap<K, CachedValue<V>>,
}

/// If a value is an accumulator, it is stored as an integer to avoid repeated
/// serialization-deserialization overhead.
enum CachedValue<V> {
    RawValue(V),
    Accumulator(u128),
}

impl<K, V, S> DashMapStateView<K, V, S>
where
    K: Eq + Hash + Sync + Send + Clone,
    S: TStateView<Key = K>,
    V: TransactionWrite,
{
    pub fn new(base_view: S) -> Self {
        Self {
            base_view,
            updates: DashMap::new(),
        }
    }

    pub fn with_capacity(base_view: S, capacity: usize) -> Self {
        Self {
            base_view,
            updates: DashMap::with_capacity(capacity),
        }
    }
}

impl<'view, K, V, S> TStateView for DashMapStateView<K, V, S>
where
    K: Eq + Hash + Sync + Send,
    V: TransactionWrite,
    S: TStateView<Key = K>,
{
    type Key = S::Key;

    fn id(&self) -> StateViewId {
        self.base_view.id()
    }

    fn get_state_value_u128(&self, state_key: &Self::Key) -> Result<Option<u128>> {
        if let Some(value_ref) = self.updates.get(state_key) {
            match value_ref.value() {
                CachedValue::RawValue(v) => {
                    Ok(Some(AggregatorValue::from_write(v).unwrap().into()))
                },
                CachedValue::Accumulator(v) => Ok(Some(*v)),
            }
        } else {
            self.base_view.get_state_value_u128(state_key)
        }
    }

    fn get_state_value_bytes(&self, state_key: &Self::Key) -> Result<Option<Vec<u8>>> {
        if let Some(value_ref) = self.updates.get(state_key) {
            match value_ref.value() {
                CachedValue::RawValue(v) => Ok(v.extract_raw_bytes()),
                CachedValue::Accumulator(v) => Ok(Some(serialize(v))),
            }
        } else {
            self.base_view.get_state_value_bytes(state_key)
        }
    }

    fn get_state_value(&self, state_key: &Self::Key) -> Result<Option<StateValue>> {
        if let Some(value_ref) = self.updates.get(state_key) {
            match value_ref.value() {
                CachedValue::RawValue(v) => Ok(v.as_state_value()),
                CachedValue::Accumulator(v) => Ok(Some(StateValue::new_legacy(serialize(v)))),
            }
        } else {
            self.base_view.get_state_value(state_key)
        }
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        self.base_view.get_usage()
    }
}

impl<'view, K, V, S> WritableStateView for DashMapStateView<K, V, S>
where
    K: Eq + Hash + Sync + Send + Clone,
    S: TStateView<Key = K>,
    V: TransactionWrite,
{
    type Value = V;

    fn write_value(&self, k: K, v: V) {
        self.updates.insert(k, CachedValue::RawValue(v));
    }

    fn write_u128(&self, k: K, v: u128) {
        self.updates.insert(k, CachedValue::Accumulator(v));
    }

    /// Applies the delta to the value associated with key `k`.
    /// Returns the new value of the accumulator or an error in case of an overflow.
    fn apply_delta(&self, k: &K, delta: &DeltaOp) -> Result<u128> {
        match self.updates.entry(k.clone()) {
            Entry::Occupied(mut entry) => {
                // `k` is already present in the hash map
                let old_value = match entry.get() {
                    CachedValue::RawValue(raw_value) => {
                        // TODO: check if unwrap is appropriate here.
                        AggregatorValue::from_write(raw_value).unwrap().into()
                    },
                    CachedValue::Accumulator(int_value) => *int_value,
                };

                let new_value = delta.apply_to(old_value)?;
                entry.insert(CachedValue::Accumulator(new_value));
                Ok(new_value)
            },
            Entry::Vacant(entry) => {
                // `k` is not present in the hash map
                // TODO: check if unwrap is appropriate here.
                let base_value = self.base_view.get_state_value_u128(k)?.unwrap();
                let new_value = delta.apply_to(base_value)?;
                entry.insert(CachedValue::Accumulator(new_value));
                Ok(new_value)
            },
        }
    }
}

/// A stub TStateView that doesn't store any data.
pub struct EmptyStateView<K> {
    phantom: PhantomData<K>,
}

impl<K> EmptyStateView<K> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<K> TStateView for EmptyStateView<K> {
    type Key = K;

    fn get_state_value(&self, _state_key: &Self::Key) -> Result<Option<StateValue>> {
        Ok(None)
    }

    fn get_usage(&self) -> Result<StateStorageUsage> {
        Err(anyhow::Error::msg(
            "Requested usage information from EmptyStateView",
        ))
    }
}
