// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{layer::MapLayer, KeyHash};
use itertools::Itertools;
use proptest::{collection::vec, prelude::*};
use std::{
    collections::{BTreeMap, HashMap},
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
    vec(vec(any::<(u8, u8)>(), 1..100), 1..100).prop_flat_map(|items_per_layer| {
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
fn test_basic1() {
    let persisted_layer = MapLayer::new_family("test_basic");
    let layer0 = persisted_layer.clone();

    let map00 = layer0.view_layers_after(&persisted_layer);
    println!("Constructing map00");
    let layer1 = map00.new_layer(&[
        (HashCollide(140), 0),
        (HashCollide(104), 1),
        (HashCollide(44), 2),
        (HashCollide(208), 3),
        (HashCollide(44), 4),
    ]);

    let map01 = layer1.view_layers_after(&layer0);
    assert_eq!(map01.get(&HashCollide(140)), Some(0));
    assert_eq!(map01.get(&HashCollide(104)), Some(1));
    assert_eq!(map01.get(&HashCollide(208)), Some(3));
    assert_eq!(map01.get(&HashCollide(44)), Some(4));
    let traversed: BTreeMap<_, _> = map01.iter().collect();
    assert_eq!(traversed, maplit::btreemap! {
        HashCollide(140) => 0,
        HashCollide(104) => 1,
        HashCollide(44) => 4,
        HashCollide(208) => 3,
    });

    println!("Constructing map12");
    let layer2 = map01.new_layer(&[(HashCollide(44), 5)]);
    let map12 = layer2.view_layers_after(&layer1);
    assert_eq!(map12.get(&HashCollide(140)), None);
    assert_eq!(map12.get(&HashCollide(104)), None);
    assert_eq!(map12.get(&HashCollide(44)), Some(5));
    assert_eq!(map12.get(&HashCollide(208)), None);
    let traversed: BTreeMap<_, _> = map12.iter().collect();
    assert_eq!(traversed, maplit::btreemap! { HashCollide(44) => 5 });
}

#[test]
fn basic_stuff() {
    let persisted_layer = MapLayer::new_family("test_basic");
    let layer0 = persisted_layer.clone();

    let map00 = layer0.view_layers_after(&persisted_layer);
    let layer1 = map00.new_layer(&[(1, "a"), (2, "b")]);

    let map01 = layer1.view_layers_after(&layer0);
    assert_eq!(map01.get(&1), Some("a"));
    assert_eq!(map01.get(&2), Some("b"));
    assert_eq!(map01.get(&3), None);
    assert_eq!(map01.get(&4), None);
    let traversed: BTreeMap<_, _> = map01.iter().collect();
    assert_eq!(traversed, maplit::btreemap! {1 => "a", 2 => "b"});

    let layer2 = map01.new_layer(&[(2, "c"), (3, "d")]);
    let map12 = layer2.view_layers_after(&layer1);
    assert_eq!(map12.get(&1), None);
    assert_eq!(map12.get(&2), Some("c"));
    assert_eq!(map12.get(&3), Some("d"));
    assert_eq!(map12.get(&4), None);
    let traversed: BTreeMap<_, _> = map12.iter().collect();
    assert_eq!(traversed, maplit::btreemap! { 2 => "c", 3 => "d" });

    let map02 = layer2.view_layers_after(&layer0);
    assert_eq!(map02.get(&1), Some("a"));
    assert_eq!(map02.get(&2), Some("c"));
    assert_eq!(map02.get(&3), Some("d"));
    assert_eq!(map02.get(&4), None);
    let traversed: BTreeMap<_, _> = map02.iter().collect();
    assert_eq!(
        traversed,
        maplit::btreemap! { 1 => "a", 2 => "c", 3 => "d" }
    );

    let map22 = layer2.view_layers_after(&layer2);
    assert_eq!(map22.get(&1), None);
    assert_eq!(map22.get(&2), None);
    assert_eq!(map22.get(&3), None);
    assert_eq!(map22.get(&4), None);
    let traversed: BTreeMap<_, _> = map22.iter().collect();
    assert!(traversed.is_empty());
}

#[test]
fn empty_layer() {
    let persisted_layer = MapLayer::new_family("test_basic");
    let layer0 = persisted_layer.clone();

    let map00 = layer0.view_layers_after(&persisted_layer);
    let layer1 = map00.new_layer(&[(1, "a")]);
    println!("layer1 peak: {:?}", layer1.inner.peak);

    let map01 = layer1.view_layers_after(&layer0);
    let layer2 = map01.new_layer(&[]);
    println!("layer2 peak: {:?}", layer2.inner.peak);

    let map12 = layer2.view_layers_after(&layer1);
    assert_eq!(map12.get(&1), None);
    assert_eq!(map12.get(&2), None);
    let traversed: BTreeMap<_, _> = map12.iter().collect();
    assert!(traversed.is_empty());
}

#[test]
fn basic_collide() {
    let persisted_layer = MapLayer::new_family("test_basic");
    let layer0 = persisted_layer.clone();

    let map00 = layer0.view_layers_after(&persisted_layer);
    let layer1 = map00.new_layer(&[
        (HashCollide(1), "a"),
        (HashCollide(2), "b"),
        (HashCollide(3), "c"),
        (HashCollide(4), "d"),
        (HashCollide(5), "e"),
        (HashCollide(6), "f"),
        (HashCollide(7), "g"),
        (HashCollide(8), "h"),
        (HashCollide(9), "i"),
        (HashCollide(10), "j"),
        (HashCollide(11), "k"),
        (HashCollide(12), "l"),
        (HashCollide(13), "m"),
        (HashCollide(14), "n"),
        (HashCollide(15), "o"),
    ]);

    let map01 = layer1.view_layers_after(&layer0);
    assert_eq!(map01.get(&HashCollide(1)), Some("a"));
    assert_eq!(map01.get(&HashCollide(2)), Some("b"));
    assert_eq!(map01.get(&HashCollide(3)), Some("c"));
    assert_eq!(map01.get(&HashCollide(4)), Some("d"));
    assert_eq!(map01.get(&HashCollide(5)), Some("e"));
    assert_eq!(map01.get(&HashCollide(6)), Some("f"));
    assert_eq!(map01.get(&HashCollide(7)), Some("g"));
    assert_eq!(map01.get(&HashCollide(8)), Some("h"));
    assert_eq!(map01.get(&HashCollide(9)), Some("i"));
    assert_eq!(map01.get(&HashCollide(10)), Some("j"));
    assert_eq!(map01.get(&HashCollide(11)), Some("k"));
    assert_eq!(map01.get(&HashCollide(12)), Some("l"));
    assert_eq!(map01.get(&HashCollide(13)), Some("m"));
    assert_eq!(map01.get(&HashCollide(14)), Some("n"));
    assert_eq!(map01.get(&HashCollide(15)), Some("o"));
    let traversed: BTreeMap<_, _> = map01.iter().collect();
    assert_eq!(traversed, maplit::btreemap! {
        HashCollide(1) => "a",
        HashCollide(2) => "b",
        HashCollide(3) => "c",
        HashCollide(4) => "d",
        HashCollide(5) => "e",
        HashCollide(6) => "f",
        HashCollide(7) => "g",
        HashCollide(8) => "h",
        HashCollide(9) => "i",
        HashCollide(10) => "j",
        HashCollide(11) => "k",
        HashCollide(12) => "l",
        HashCollide(13) => "m",
        HashCollide(14) => "n",
        HashCollide(15) => "o",
    });

    let layer2 = map01.new_layer(&[(HashCollide(2), "c"), (HashCollide(3), "d")]);
    let map12 = layer2.view_layers_after(&layer1);
    assert_eq!(map12.get(&HashCollide(1)), None);
    assert_eq!(map12.get(&HashCollide(2)), Some("c"));
    assert_eq!(map12.get(&HashCollide(3)), Some("d"));
    assert_eq!(map12.get(&HashCollide(4)), None);
    let traversed: BTreeMap<_, _> = map12.iter().collect();
    assert_eq!(
        traversed,
        maplit::btreemap! { HashCollide(2) => "c", HashCollide(3) => "d" }
    );

    let map02 = layer2.view_layers_after(&layer0);
    assert_eq!(map02.get(&HashCollide(1)), Some("a"));
    assert_eq!(map02.get(&HashCollide(2)), Some("c"));
    assert_eq!(map02.get(&HashCollide(3)), Some("d"));
    assert_eq!(map02.get(&HashCollide(4)), Some("d"));
    assert_eq!(map02.get(&HashCollide(5)), Some("e"));
    assert_eq!(map02.get(&HashCollide(6)), Some("f"));
    assert_eq!(map02.get(&HashCollide(7)), Some("g"));
    assert_eq!(map02.get(&HashCollide(8)), Some("h"));
    assert_eq!(map02.get(&HashCollide(9)), Some("i"));
    assert_eq!(map02.get(&HashCollide(10)), Some("j"));
    assert_eq!(map02.get(&HashCollide(11)), Some("k"));
    assert_eq!(map02.get(&HashCollide(12)), Some("l"));
    assert_eq!(map02.get(&HashCollide(13)), Some("m"));
    assert_eq!(map02.get(&HashCollide(14)), Some("n"));
    assert_eq!(map02.get(&HashCollide(15)), Some("o"));
    let traversed: BTreeMap<_, _> = map02.iter().collect();
    assert_eq!(traversed, maplit::btreemap! {
        HashCollide(1) => "a",
        HashCollide(2) => "c",
        HashCollide(3) => "d",
        HashCollide(4) => "d",
        HashCollide(5) => "e",
        HashCollide(6) => "f",
        HashCollide(7) => "g",
        HashCollide(8) => "h",
        HashCollide(9) => "i",
        HashCollide(10) => "j",
        HashCollide(11) => "k",
        HashCollide(12) => "l",
        HashCollide(13) => "m",
        HashCollide(14) => "n",
        HashCollide(15) => "o",
    });

    drop(map00);
    drop(map01);
    drop(map02);
    drop(persisted_layer);
    drop(layer0);
    assert_eq!(map12.get(&HashCollide(1)), None);
    assert_eq!(map12.get(&HashCollide(2)), Some("c"));
    assert_eq!(map12.get(&HashCollide(3)), Some("d"));
    assert_eq!(map12.get(&HashCollide(4)), None);
    let traversed: BTreeMap<_, _> = map12.iter().collect();
    assert_eq!(
        traversed,
        maplit::btreemap! { HashCollide(2) => "c", HashCollide(3) => "d" }
    );
}

proptest! {
    // #![proptest_config(ProptestConfig::with_cases(0))]

    #[test]
    fn test_layered_map_get(
        (mut items_per_update, ancestor, base, top) in arb_test_case()
    ) {
        let (_ancestor_layer, base_layer, top_layer) = {
            println!("items_per_update: {:?}", items_per_update);
            println!("ancestor: {ancestor}. base: {base}. top: {top}");
            let layers = layers(&items_per_update, ancestor as u64);
            println!("num layers: {}", layers.len());
            // layers[0], layers[1], layers[1]
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
        let traversed: BTreeMap<_, _> = layered_map.iter().collect();
        println!("traversed: {:?}", traversed);
        println!("all: {:?}", all);
        prop_assert_eq!(all.len(), traversed.len());
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
