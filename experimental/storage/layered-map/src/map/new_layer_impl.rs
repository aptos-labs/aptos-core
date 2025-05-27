// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    flatten_perfect_tree::{FlattenPerfectTree, FptRef, FptRefMut},
    metrics::TIMER,
    node::{CollisionCell, LeafContent, LeafNode, NodeRef, NodeStrongRef},
    utils::binary_tree_height,
    Key, KeyHash, LayeredMap, MapLayer, Value,
};
use aptos_drop_helper::ArcAsyncDrop;
use aptos_metrics_core::TimerHelper;
use itertools::Itertools;
use std::collections::BTreeMap;

impl<K, V, S> LayeredMap<K, V, S>
where
    K: ArcAsyncDrop + Key,
    V: ArcAsyncDrop + Value,
{
    pub fn new_layer_with_hasher(&self, kvs: &[(K, V)], hash_builder: &S) -> MapLayer<K, V>
    where
        S: core::hash::BuildHasher,
    {
        let _timer = TIMER.timer_with(&[self.top_layer.use_case(), "new_layer"]);

        // Hash the keys and sort items in key hash order.
        //
        // n.b. no need to dedup at this point, as we will do it anyway at the leaf level.
        let items = kvs
            .iter()
            .map(|kv| {
                let key = &kv.0;
                let key_hash = KeyHash(hash_builder.hash_one(key));
                Item { key_hash, kv }
            })
            .sorted_by_key(Item::full_key)
            .collect_vec();

        let height = Self::new_peak_height(self.top_layer.peak().num_leaves(), items.len());
        let mut new_peak = FlattenPerfectTree::new_with_empty_nodes(height);
        let builder = SubTreeBuilder {
            layer: self.top_layer.layer() + 1,
            base_layer: self.base_layer(),
            depth: 0,
            position_info: PositionInfo::new(self.top_layer.peak(), self.base_layer()),
            output_position_info: OutputPositionInfo::new(new_peak.get_mut()),
            items: &items,
        };
        builder.build().finalize();

        self.top_layer.spawn(new_peak, self.base_layer())
    }

    fn new_peak_height(previous_peak_feet: usize, items_in_new_layer: usize) -> usize {
        let old = binary_tree_height(previous_peak_feet);
        let new = 2.max(binary_tree_height(items_in_new_layer)) - 1;
        1.max(old - 1).max(new)
    }

    pub fn new_layer(&self, items: &[(K, V)]) -> MapLayer<K, V>
    where
        S: core::hash::BuildHasher + Default,
    {
        self.new_layer_with_hasher(items, &Default::default())
    }
}

pub(crate) struct Item<'a, K, V> {
    key_hash: KeyHash,
    kv: &'a (K, V),
}

impl<'a, K, V> Item<'a, K, V> {
    fn key_hash(&self) -> KeyHash {
        self.key_hash
    }

    fn key(&self) -> &'a K {
        &self.kv.0
    }

    /// Full key used for sorting and deduplication.
    ///
    /// Inequality is detected if key hash is different, keys only need to be compared in case of
    /// hash collision.
    fn full_key(&self) -> (KeyHash, &'a K) {
        (self.key_hash(), self.key())
    }

    fn kv(&self) -> &(K, V) {
        self.kv
    }
}

enum PositionInfo<'a, K, V> {
    AbovePeakFeet(FptRef<'a, K, V>),
    PeakFootOrBelow(NodeStrongRef<K, V>),
}

impl<'a, K, V> PositionInfo<'a, K, V> {
    fn new(peak: FptRef<'a, K, V>, base_layer: u64) -> Self {
        if peak.num_leaves() == 1 {
            Self::PeakFootOrBelow(peak.expect_single_node(base_layer))
        } else {
            Self::AbovePeakFeet(peak)
        }
    }

    fn is_above_peak_feet(&self) -> bool {
        matches!(self, Self::AbovePeakFeet(..))
    }

    fn expect_peak_foot_or_below(&self) -> NodeStrongRef<K, V> {
        match self {
            Self::AbovePeakFeet(..) => panic!("Still in Peak"),
            Self::PeakFootOrBelow(node) => node.clone(),
        }
    }

    fn children(self, depth: usize, base_layer: u64) -> (Self, Self) {
        use PositionInfo::*;

        match self {
            AbovePeakFeet(fpt) => {
                let (left, right) = fpt.expect_sub_trees();
                if left.is_single_node() {
                    (
                        PeakFootOrBelow(left.expect_single_node(base_layer)),
                        PeakFootOrBelow(right.expect_single_node(base_layer)),
                    )
                } else {
                    (AbovePeakFeet(left), AbovePeakFeet(right))
                }
            },
            PeakFootOrBelow(node) => {
                let (left, right) = node.children(depth, base_layer);
                (PeakFootOrBelow(left), PeakFootOrBelow(right))
            },
        }
    }
}

enum OutputPositionInfo<'a, K, V> {
    AboveOrAtPeakFeet(FptRefMut<'a, K, V>),
    BelowPeakFeet,
}

impl<'a, K, V> OutputPositionInfo<'a, K, V> {
    pub fn new(fpt_mut: FptRefMut<'a, K, V>) -> Self {
        Self::AboveOrAtPeakFeet(fpt_mut)
    }

    pub fn is_above_peak_feet(&self) -> bool {
        if let OutputPositionInfo::AboveOrAtPeakFeet(fpt) = self {
            !fpt.is_single_node()
        } else {
            false
        }
    }

    pub fn is_below_peak_feet(&self) -> bool {
        matches!(self, OutputPositionInfo::BelowPeakFeet)
    }

    pub fn into_pending_build(
        self,
    ) -> (
        PendingBuild<'a, K, V>,
        OutputPositionInfo<'a, K, V>,
        OutputPositionInfo<'a, K, V>,
    ) {
        match self {
            OutputPositionInfo::AboveOrAtPeakFeet(fpt_mut) => {
                if fpt_mut.is_single_node() {
                    (
                        PendingBuild::FootOfPeak(fpt_mut.expect_into_single_node_mut()),
                        OutputPositionInfo::BelowPeakFeet,
                        OutputPositionInfo::BelowPeakFeet,
                    )
                } else {
                    let (left, right) = fpt_mut.expect_into_sub_trees();
                    (
                        PendingBuild::AbovePeakFeet,
                        OutputPositionInfo::AboveOrAtPeakFeet(left),
                        OutputPositionInfo::AboveOrAtPeakFeet(right),
                    )
                }
            },
            OutputPositionInfo::BelowPeakFeet => (
                PendingBuild::BelowPeakFeet,
                OutputPositionInfo::BelowPeakFeet,
                OutputPositionInfo::BelowPeakFeet,
            ),
        }
    }
}

enum PendingBuild<'a, K, V> {
    AbovePeakFeet,
    FootOfPeak(&'a mut NodeRef<K, V>),
    BelowPeakFeet,
}

impl<K, V> PendingBuild<'_, K, V> {
    fn seal_with_node(&mut self, node: NodeRef<K, V>) -> BuiltSubTree<K, V> {
        match self {
            PendingBuild::AbovePeakFeet => unreachable!("Trying to put node above peak feet."),
            PendingBuild::FootOfPeak(ref_mut) => {
                **ref_mut = node;
                BuiltSubTree::InOrAtFootOfPeak
            },
            PendingBuild::BelowPeakFeet => BuiltSubTree::BelowPeak(node),
        }
    }

    fn seal_with_children(
        &mut self,
        left: BuiltSubTree<K, V>,
        right: BuiltSubTree<K, V>,
        layer: u64,
    ) -> BuiltSubTree<K, V> {
        match (left, right) {
            (BuiltSubTree::InOrAtFootOfPeak, BuiltSubTree::InOrAtFootOfPeak) => {
                assert!(
                    matches!(self, PendingBuild::AbovePeakFeet),
                    "Expecting nodes."
                );
                BuiltSubTree::InOrAtFootOfPeak
            },
            (BuiltSubTree::BelowPeak(left), BuiltSubTree::BelowPeak(right)) => {
                let internal_node = Self::merge_subtrees(left, right, layer);
                self.seal_with_node(internal_node)
            },
            _ => unreachable!("Children should be of same flavor."),
        }
    }

    fn merge_subtrees(left: NodeRef<K, V>, right: NodeRef<K, V>, layer: u64) -> NodeRef<K, V> {
        use crate::node::NodeRef::*;

        match (&left, &right) {
            (Empty, Leaf(..)) => right,
            (Leaf(..), Empty) => left,
            (Empty, Empty) => Empty,
            _ => NodeRef::new_internal(left, right, layer),
        }
    }
}

#[must_use = "Must finalize()"]
enum BuiltSubTree<K, V> {
    InOrAtFootOfPeak,
    BelowPeak(NodeRef<K, V>),
}

impl<K, V> BuiltSubTree<K, V> {
    fn finalize(self) {
        // note: need to carry height to assert more strongly
        // (that it's built all the way to the root)
        assert!(
            matches!(self, BuiltSubTree::InOrAtFootOfPeak),
            "Haven't reached the peak."
        );
    }
}

struct SubTreeBuilder<'a, K, V> {
    /// the layer being built
    layer: u64,
    /// anything at this layer or earlier is assumed invisible
    base_layer: u64,
    depth: usize,
    position_info: PositionInfo<'a, K, V>,
    output_position_info: OutputPositionInfo<'a, K, V>,
    items: &'a [Item<'a, K, V>],
}

impl<K, V> SubTreeBuilder<'_, K, V>
where
    K: ArcAsyncDrop + Key,
    V: ArcAsyncDrop + Value,
{
    fn all_items_same_key_hash(&self) -> Option<KeyHash> {
        let items = &self.items;

        assert!(!items.is_empty());
        let first_key_hash = items[0].key_hash();
        if first_key_hash == items[items.len() - 1].key_hash() {
            Some(first_key_hash)
        } else {
            None
        }
    }

    fn still_in_peak(&self) -> bool {
        self.position_info.is_above_peak_feet() || self.output_position_info.is_above_peak_feet()
    }

    pub fn build(self) -> BuiltSubTree<K, V> {
        if self.still_in_peak() {
            // Can't start building up unless deep enough to see the bottoms of the peaks.
            self.branch_further()
        } else if self.items.is_empty() {
            // No new leaves to add in this branch, return weak ref to the current node.
            let node = self.position_info.expect_peak_foot_or_below().weak_ref();
            self.terminate_with_node(node)
        } else {
            match self.all_items_same_key_hash() {
                None => {
                    // Still multiple leaves to add, branch further down.
                    self.branch_further()
                },
                Some(key_hash) => {
                    // All new items belong to the same new leaf node.
                    match self.position_info.expect_peak_foot_or_below() {
                        NodeStrongRef::Empty => {
                            let node = self.new_leaf(key_hash, self.items);
                            self.terminate_with_node(node)
                        },
                        NodeStrongRef::Leaf(leaf) => {
                            if leaf.key_hash == key_hash {
                                let node =
                                    self.new_leaf_overwriting_old(key_hash, &leaf, self.items);
                                self.terminate_with_node(node)
                            } else {
                                self.branch_further()
                            }
                        },
                        NodeStrongRef::Internal(_) => self.branch_further(),
                    }
                }, // end Some(key_hash) == all_items_same_key_hash()
            } // end match
        } // end else
    }

    fn branch_further(self) -> BuiltSubTree<K, V> {
        let Self {
            layer,
            base_layer,
            depth,
            position_info,
            output_position_info,
            items,
        } = self;

        let (mut pending_build, out_left, out_right) = output_position_info.into_pending_build();
        let (pos_left, pos_right) = position_info.children(depth, base_layer);

        let pivot = items.partition_point(|item| !item.key_hash.bit(depth));
        let (items_left, items_right) = items.split_at(pivot);

        let left = Self {
            layer,
            base_layer,
            depth: depth + 1,
            position_info: pos_left,
            output_position_info: out_left,
            items: items_left,
        };
        let right = Self {
            layer,
            base_layer,
            depth: depth + 1,
            position_info: pos_right,
            output_position_info: out_right,
            items: items_right,
        };
        pending_build.seal_with_children(left.build(), right.build(), layer)
    }

    fn terminate_with_node(self, node: NodeRef<K, V>) -> BuiltSubTree<K, V> {
        let (mut pending_build, left, right) = self.output_position_info.into_pending_build();
        assert!(left.is_below_peak_feet() && right.is_below_peak_feet());

        pending_build.seal_with_node(node)
    }

    fn new_leaf(&self, key_hash: KeyHash, items: &[Item<K, V>]) -> NodeRef<K, V> {
        NodeRef::new_leaf(
            key_hash,
            Self::to_leaf_content(items, self.layer),
            self.layer,
        )
    }

    fn new_leaf_overwriting_old(
        &self,
        key_hash: KeyHash,
        old_leaf: &LeafNode<K, V>,
        new_items: &[Item<K, V>],
    ) -> NodeRef<K, V> {
        let old = old_leaf.content.clone();
        let new = Self::to_leaf_content(new_items, self.layer);
        let content = old.combined_with(old_leaf.layer, new, self.layer, self.base_layer);

        NodeRef::new_leaf(key_hash, content, self.layer)
    }

    fn to_leaf_content(items: &[Item<K, V>], layer: u64) -> LeafContent<K, V> {
        assert!(!items.is_empty());
        if items.len() == 1 {
            let (key, value) = items[0].kv().clone();
            LeafContent::UniqueLatest { key, value }
        } else {
            // deduplication
            let mut map: BTreeMap<_, _> = items
                .iter()
                .map(|item| {
                    let (key, value) = item.kv().clone();
                    (key, CollisionCell { value, layer })
                })
                .collect();
            if map.len() == 1 {
                let (key, cell) = map.pop_first().unwrap();
                LeafContent::UniqueLatest {
                    key,
                    value: cell.value,
                }
            } else {
                LeafContent::Collision(map)
            }
        }
    }
}
