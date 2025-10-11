// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod new_layer_impl;

use crate::{iterator::DescendantIterator, node::NodeRawPtr, Key, KeyHash, MapLayer, Value};
use aptos_drop_helper::ArcAsyncDrop;
use std::marker::PhantomData;

pub(crate) type DefaultHashBuilder = core::hash::BuildHasherDefault<ahash::AHasher>;

/// A view of content within range (base_layer, top_layer] (n.b. left-exclusive, right-inclusive).
#[derive(Clone, Debug)]
pub struct LayeredMap<K: ArcAsyncDrop, V: ArcAsyncDrop, S = DefaultHashBuilder> {
    /// n.b. base layer content is not visible
    base_layer: MapLayer<K, V>,
    top_layer: MapLayer<K, V>,
    /// Hasher is needed only for spawning a new layer, i.e. for a read only map there's no need to
    /// pay the overhead of constructing it.
    hash_builder: S,
}

impl<K, V, S> LayeredMap<K, V, S>
where
    K: ArcAsyncDrop,
    V: ArcAsyncDrop,
    S: Default,
{
    pub fn new(base_layer: MapLayer<K, V>, top_layer: MapLayer<K, V>) -> Self {
        Self {
            base_layer,
            top_layer,
            hash_builder: Default::default(),
        }
    }

    pub fn new_with_hasher(
        base_layer: MapLayer<K, V>,
        top_layer: MapLayer<K, V>,
        hash_builder: S,
    ) -> Self {
        Self {
            base_layer,
            top_layer,
            hash_builder,
        }
    }

    pub fn unpack(self) -> (MapLayer<K, V>, MapLayer<K, V>) {
        let Self {
            base_layer,
            top_layer,
            hash_builder,
        } = self;

        (base_layer, top_layer)
    }

    pub(crate) fn base_layer(&self) -> u32 {
        self.base_layer.layer()
    }

    pub(crate) fn top_layer(&self) -> u32 {
        self.top_layer.layer()
    }
}

impl<K, V, S> LayeredMap<K, V, S>
where
    K: ArcAsyncDrop + Key + std::fmt::Debug,
    V: ArcAsyncDrop + Value + std::fmt::Debug,
    S: Default,
{
    pub fn get_with_hasher(&self, key: &K, hash_builder: &S) -> Option<V>
    where
        S: core::hash::BuildHasher,
    {
        let key_hash = KeyHash(hash_builder.hash_one(key));
        // println!("looking up key {:?}, key hash: {:?}", key, key_hash);
        let mut bits = key_hash.iter_bits();

        let peak = self.top_layer.peak();
        // println!("peak.height(): {}", peak.height());
        let mut foot = 0;
        for _ in 0..peak.height() - 1 {
            foot = (foot << 1) | bits.next().expect("bits exhausted") as usize;
        }
        // println!("foot: {}", foot);

        self.get_under_node(peak.expect_foot(foot, self.base_layer()), key, &mut bits)
    }

    fn get_under_node(
        &self,
        node: NodeRawPtr<K, V>,
        key: &K,
        remaining_key_bits: &mut impl Iterator<Item = bool>,
    ) -> Option<V> {
        // println!("looking up key {:?} under node", key);
        let mut cur_node = node;
        let bits = remaining_key_bits;

        loop {
            match cur_node {
                NodeRawPtr::Empty => {
                    // println!("see empty");
                    return None;
                },
                NodeRawPtr::Leaf(leaf) => {
                    let leaf = unsafe { leaf.as_ref() };
                    // println!("see leaf {:?}", leaf);
                    return leaf.get_value(key, self.base_layer()).cloned();
                },
                NodeRawPtr::Internal(internal) => {
                    // println!("see internal");
                    match bits.next() {
                        None => {
                            unreachable!("value on key-prefix not supported.");
                        },
                        Some(bit) => {
                            let internal = unsafe { internal.as_ref() };
                            if bit {
                                // println!("go right");
                                if internal.right_layer > self.base_layer() {
                                    cur_node = internal.right.get_raw(self.base_layer());
                                } else {
                                    return None;
                                }
                            } else {
                                // println!("go left");
                                if internal.left_layer > self.base_layer() {
                                    cur_node = internal.left.get_raw(self.base_layer());
                                } else {
                                    return None;
                                }
                            }
                        },
                    }
                },
            } // end match cur_node
        } // end loop
    }

    pub fn get(&self, key: &K) -> Option<V>
    where
        S: core::hash::BuildHasher + Default,
    {
        self.get_with_hasher(key, &Default::default())
    }

    // TODO(aldenhu): make `Item = (&K, &V)`
    pub fn iter(&self) -> Box<dyn Iterator<Item = (K, V)> + '_> {
        // println!(
        //     "iter... base_layer: {}, top_layer: {}",
        //     self.base_layer.layer(),
        //     self.top_layer.layer()
        // );
        // println!("peak: {:?}", self.top_layer.peak());
        if self.top_layer.layer() <= self.base_layer.layer() {
            return Box::new(std::iter::empty());
        }

        Box::new(
            self.top_layer
                .peak()
                .into_feet_iter(self.base_layer.layer())
                .flat_map(|node| DescendantIterator::new(node, self.base_layer())),
        )
    }
}
