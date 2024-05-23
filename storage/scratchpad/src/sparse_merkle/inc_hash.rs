// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Play with Incremental Hash as a way to authenticate the state

use crate::sparse_merkle::metrics::GENERATION;
use aptos_crypto::HashValue;
use aptos_drop_helper::ArcAsyncDrop;
use aptos_experimental_layered_map::MapLayer;
use aptos_metrics_core::IntGaugeHelper;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use bitvec::{order::Msb0, view::BitView};
use fastcrypto::hash::EllipticCurveMultisetHash;
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HashAsKey(pub HashValue);

impl aptos_experimental_layered_map::Key for HashAsKey {
    fn iter_bits(&self) -> impl Iterator<Item = bool> {
        self.0.iter_bits()
    }

    fn bit(&self, depth: usize) -> bool {
        *self.0.as_slice().view_bits::<Msb0>().get(depth).unwrap()
    }
}

#[derive(Debug)]
pub(crate) struct Root<V: ArcAsyncDrop> {
    pub inc_hash: EllipticCurveMultisetHash,
    pub hash: HashValue,
    pub content: MapLayer<HashAsKey, Option<V>>,
}

fn hash_inc_hash(inc_hash: &EllipticCurveMultisetHash) -> HashValue {
    let mut hasher = aptos_crypto::hash::DefaultHasher::new(b"IncHash");
    hasher.update(&bcs::to_bytes(inc_hash).unwrap());
    hasher.finish()
}

impl<V: ArcAsyncDrop> Root<V> {
    pub fn new(
        inc_hash: EllipticCurveMultisetHash,
        content: MapLayer<HashAsKey, Option<V>>,
    ) -> Self {
        let hash = hash_inc_hash(&inc_hash);

        Root {
            inc_hash,
            hash,
            content,
        }
    }

    pub fn hash(&self) -> HashValue {
        self.hash
    }
}

#[derive(Debug)]
pub(crate) struct AuthByIncHash<V: ArcAsyncDrop> {
    pub root: Root<V>,
    pub usage: StateStorageUsage,
}

impl<V: ArcAsyncDrop> AuthByIncHash<V> {
    pub fn new(usage: StateStorageUsage) -> Self {
        let root = Root::new(
            EllipticCurveMultisetHash::default(),
            MapLayer::new_family("AuthByIncHash::new"),
        );

        AuthByIncHash { root, usage }
    }

    pub fn spawn(&self, child_root: Root<V>, child_usage: StateStorageUsage) -> Arc<Self> {
        Self {
            root: child_root,
            usage: child_usage,
        }
        .into()
    }

    pub fn root(&self) -> &Root<V> {
        &self.root
    }

    pub fn generation(&self) -> u64 {
        self.root.content.layer()
    }

    pub fn is_family(&self, other: &Self) -> bool {
        self.root.content.is_family(&other.root.content)
    }

    pub fn log_generation(&self, name: &'static str) {
        GENERATION.set_with(&[name], self.root.content.layer() as i64);
    }
}
