// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{layer::MapLayer, KeyHash};
use itertools::Itertools;
use proptest::{collection::vec, prelude::*};
use std::{
    collections::BTreeMap,
    hash::{Hash, Hasher},
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct HashCollide(u8);

impl Hash for HashCollide {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // artificially make collision
        state.write_u8(self.0 >> 2);
    }
}

fn naive_view_layer<K: Ord, V>(layer: Vec<(K, V)>) -> BTreeMap<K, V> {
    layer.into_iter().collect()
}

fn naive_view_layers<K: Ord, V>(layers: impl Iterator<Item = Vec<(K, V)>>) -> BTreeMap<K, V> {
    layers.flat_map(|layer| layer.into_iter()).collect()
}

fn arb_test_case() -> impl Strategy<Value = (Vec<Vec<(HashCollide, u8)>>, usize, usize, usize)> {
    vec(vec(any::<(u8, u8)>(), 0..100), 1..100).prop_flat_map(|items_per_layer| {
        let num_overlay_layers = items_per_layer.len();
        let items_per_update = items_per_layer.clone();
        vec(0..=num_overlay_layers, 3).prop_map(move |mut layer_indices| {
            layer_indices.sort();
            let ancestor = layer_indices[0];
            let base = layer_indices[1];
            let top = layer_indices[2];

            let items_per_update = items_per_update
                .iter()
                .map(|items| {
                    items
                        .iter()
                        .map(|(key, value)| (HashCollide(*key), *value))
                        .collect_vec()
                })
                .collect_vec();

            (items_per_update, ancestor, base, top)
        })
    })
}

fn layers(
    items_per_update: &[Vec<(HashCollide, u8)>],
    max_base_layer: u64,
) -> Vec<MapLayer<HashCollide, u8>> {
    let mut base_layer = MapLayer::new_family("test");
    let mut latest_layer = base_layer.clone();

    let mut base_layer_idx = 0;
    let mut layers = vec![base_layer.clone()];

    for (prev_layer_idx, layer_items) in items_per_update.iter().enumerate() {
        let layer_idx = prev_layer_idx + 1;
        let items_vec: Vec<_> = layer_items.iter().map(|(k, v)| (*k, *v)).collect();
        latest_layer = latest_layer
            .view_layers_after(&base_layer)
            .new_layer(&items_vec);
        layers.push(latest_layer.clone());

        // advance base layer occasionally to expose more edge cases
        if base_layer_idx < max_base_layer as usize && layer_idx % 2 == 0 {
            base_layer_idx += 1;
            base_layer = layers[base_layer_idx].clone();
        }
    }

    layers
}

#[test]
fn test_is_descendant_of() {
    //  Build a tree:
    //
    //       root (layer 0)
    //        |
    //      child1 (layer 1)
    //      /    \
    //  fork_a  fork_b (layer 2)
    //    |
    //  fork_a2 (layer 3)
    //
    let root = MapLayer::<u8, u8>::new_family("test");
    let child1 = root.view_layers_after(&root).new_layer(&[(1, 10)]);
    let fork_a = child1.view_layers_after(&root).new_layer(&[(2, 20)]);
    let fork_b = child1.view_layers_after(&root).new_layer(&[(3, 30)]);
    let fork_a2 = fork_a.view_layers_after(&root).new_layer(&[(4, 40)]);

    // Self-descendant (reflexive).
    assert!(root.is_descendant_of(&root));
    assert!(child1.is_descendant_of(&child1));
    assert!(fork_a.is_descendant_of(&fork_a));

    // Direct lineage.
    assert!(child1.is_descendant_of(&root));
    assert!(fork_a.is_descendant_of(&root));
    assert!(fork_a.is_descendant_of(&child1));
    assert!(fork_a2.is_descendant_of(&root));
    assert!(fork_a2.is_descendant_of(&child1));
    assert!(fork_a2.is_descendant_of(&fork_a));
    assert!(fork_b.is_descendant_of(&root));
    assert!(fork_b.is_descendant_of(&child1));

    // Ancestor is not a descendant of its descendants.
    assert!(!root.is_descendant_of(&child1));
    assert!(!root.is_descendant_of(&fork_a));
    assert!(!child1.is_descendant_of(&fork_a));

    // Cross-fork: fork_a and fork_b are NOT descendants of each other.
    assert!(!fork_a.is_descendant_of(&fork_b));
    assert!(!fork_b.is_descendant_of(&fork_a));

    // Deeper cross-fork: fork_a2 is NOT a descendant of fork_b.
    assert!(!fork_a2.is_descendant_of(&fork_b));
    assert!(!fork_b.is_descendant_of(&fork_a2));

    // Different family entirely.
    let other_root = MapLayer::<u8, u8>::new_family("other");
    assert!(!child1.is_descendant_of(&other_root));
    assert!(!other_root.is_descendant_of(&child1));
}

#[test]
fn test_can_view_after() {
    //  Build a chain with an advancing base:
    //
    //       root (layer 0)
    //        |
    //      child1 (layer 1, base_layer=0)  -- spawned from LayeredMap(root, root)
    //        |
    //      child2 (layer 2, base_layer=0)  -- spawned from LayeredMap(root, child1)
    //        |
    //      child3 (layer 3, base_layer=1)  -- spawned from LayeredMap(child1, child2)
    //        |
    //      child4 (layer 4, base_layer=2)  -- spawned from LayeredMap(child2, child3)
    //
    let root = MapLayer::<u8, u8>::new_family("test");
    let child1 = root.view_layers_after(&root).new_layer(&[(1, 10)]);
    let child2 = child1.view_layers_after(&root).new_layer(&[(2, 20)]);
    let child3 = child2.view_layers_after(&child1).new_layer(&[(3, 30)]);
    let child4 = child3.view_layers_after(&child2).new_layer(&[(4, 40)]);

    // A layer can always be viewed after itself.
    assert!(root.can_view_after(&root));
    assert!(child3.can_view_after(&child3));

    // child1 and child2 have base_layer=0, so root (layer 0) is a valid base.
    assert!(child1.can_view_after(&root));
    assert!(child2.can_view_after(&root));

    // child3 has base_layer=1, so child1 (layer 1) is the earliest valid base.
    assert!(child3.can_view_after(&child1));
    assert!(child3.can_view_after(&child2));
    // root (layer 0) is too old for child3.
    assert!(!child3.can_view_after(&root));

    // child4 has base_layer=2, so child2 (layer 2) is the earliest valid base.
    assert!(child4.can_view_after(&child2));
    assert!(child4.can_view_after(&child3));
    assert!(!child4.can_view_after(&root));
    assert!(!child4.can_view_after(&child1));

    // A base cannot be newer than the top layer.
    assert!(!root.can_view_after(&child1));
    assert!(!child1.can_view_after(&child2));

    // Different family is always invalid.
    let other = MapLayer::<u8, u8>::new_family("other");
    assert!(!child1.can_view_after(&other));
    assert!(!other.can_view_after(&child1));
}

proptest! {
    #[test]
    fn test_layered_map_get(
        (mut items_per_update, ancestor, base, top) in arb_test_case()
    ) {
        let (_ancestor_layer, base_layer, top_layer) = {
            let layers = layers(&items_per_update, ancestor as u64);
            (layers[ancestor].clone(), layers[base].clone(), layers[top].clone())
        };

        let layered_map = top_layer.into_layers_view_after(base_layer);

        // n.b. notice items_per_update doesn't have a placeholder for the root layer
        let items_per_update = items_per_update.drain(base..top).collect_vec();
        let all = naive_view_layers(items_per_update.clone().into_iter());

        // get() individually
        for (k, v) in &all {
            prop_assert_eq!(layered_map.get(k), Some(*v));
        }

        // traversed via iterator
        let traversed = layered_map.iter().collect();
        prop_assert_eq!(all, traversed);

        for (inner_map, items) in layered_map.inner_maps().into_iter().zip_eq(items_per_update.into_iter()) {
            let all = naive_view_layer(items);
            for (k, v) in &all {
                prop_assert_eq!(inner_map.get(k), Some(*v));
            }
            let traversed = inner_map.iter().collect();
            prop_assert_eq!(all, traversed);
        }
    }

    #[test]
    fn test_key_hash_order(nums in vec(any::<u64>(), 0..100)) {
        let mut a = nums.into_iter().map(KeyHash).collect_vec();
        let mut b = a.clone();

        a.sort();
        b.sort_by_key(|num| num.iter_bits().collect_vec() );

        prop_assert_eq!(a, b);
    }
}
