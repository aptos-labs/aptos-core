// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
        let all = naive_view_layers(items_per_update.drain(base..top));

        // get() individually
        for (k, v) in &all {
            prop_assert_eq!(layered_map.get(k), Some(*v));
        }

        // traversed via iterator
        let traversed = layered_map.iter().collect();
        prop_assert_eq!(all, traversed);
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
