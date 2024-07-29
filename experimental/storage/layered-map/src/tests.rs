// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MapLayer;
use bitvec::{order::Msb0, view::BitView};
use proptest::{
    collection::{btree_map, vec},
    prelude::*,
};
use std::collections::BTreeMap;

impl crate::Key for u8 {
    fn iter_bits(&self) -> impl Iterator<Item = bool> {
        self.view_bits::<Msb0>().iter().by_vals()
    }

    fn bit(&self, depth: usize) -> bool {
        self.view_bits::<Msb0>()
            .get(depth)
            .map(|b| *b)
            .unwrap_or(false)
    }
}

fn naive_view_layers<K: Ord, V>(
    layers: impl Iterator<Item = BTreeMap<K, Option<V>>>,
) -> BTreeMap<K, Option<V>> {
    layers.flat_map(|layer| layer.into_iter()).collect()
}

fn arb_test_case() -> impl Strategy<Value = (Vec<BTreeMap<u8, Option<u8>>>, usize, usize, usize)> {
    vec(btree_map(any::<u8>(), any::<Option<u8>>(), 0..100), 1..100).prop_flat_map(
        |items_per_layer| {
            let num_layers = items_per_layer.len();
            let items_per_layer = items_per_layer.clone();
            vec(0..num_layers, 3).prop_map(move |mut layer_indices| {
                layer_indices.sort();
                let ancestor = layer_indices[0];
                let bottom = layer_indices[1];
                let top = layer_indices[2];
                (items_per_layer.clone(), ancestor, bottom, top)
            })
        },
    )
}

fn layers(
    items_per_layer: &[BTreeMap<u8, Option<u8>>],
    max_base_layer: u64,
) -> Vec<MapLayer<u8, u8>> {
    let mut base_layer = MapLayer::new_family("test");
    let mut latest_layer = base_layer.clone();

    let mut base_layer_idx = 0;
    let mut layers = Vec::new();

    for (layer_idx, layer_items) in items_per_layer.iter().enumerate() {
        let items_vec: Vec<_> = layer_items.iter().map(|(k, v)| (*k, *v)).collect();
        latest_layer = latest_layer
            .view_layers_since(&base_layer)
            .new_layer(&items_vec);
        layers.push(latest_layer.clone());

        // advance base layer occationally to expose more edge cases
        if base_layer_idx < max_base_layer as usize && layer_idx % 2 == 1 {
            base_layer_idx += 1;
            base_layer = layers[base_layer_idx].clone();
        }
    }

    layers
}

proptest! {
    #[test]
    fn test_layered_map_get(
        (mut items_per_layer, ancestor, bottom, top) in arb_test_case()
    ) {
        let (_ancestor_layer, bottom_layer, top_layer) = {
            let layers = layers(&items_per_layer, ancestor as u64);
            (layers[ancestor].clone(), layers[bottom].clone(), layers[top].clone())
        };

        let layered_map = top_layer.into_layers_view_since(bottom_layer);

        for (key, value ) in naive_view_layers(
            items_per_layer.drain(bottom..=top)
        ) {
            prop_assert_eq!(layered_map.get(&key), value);
        }

        // TODO(aldenhu): test that layered_map doesn't have any unexpected keys -- need ability to traverse
    }
}
