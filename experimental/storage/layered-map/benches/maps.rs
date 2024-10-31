// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use aptos_experimental_layered_map::MapLayer;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup, Criterion,
};
use itertools::Itertools;
use rand::random;
use std::{
    collections::{BTreeMap, HashMap},
    iter::repeat_with,
};

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

type Key = u128;
type Value = HashValue;

const K: usize = 1024;

fn gen_update_batches(batch_size_k: usize, n_batches: usize) -> Vec<Vec<(Key, Value)>> {
    repeat_with(|| {
        repeat_with(|| (random(), random()))
            .take(batch_size_k * K)
            .collect_vec()
    })
    .take(n_batches)
    .collect_vec()
}

fn insert_in_batches(group: &mut BenchmarkGroup<WallTime>, batch_size_k: usize, n_batches: usize) {
    let batches = gen_update_batches(batch_size_k, n_batches);
    let total_updates = (batch_size_k * K * n_batches) as u64;
    group.throughput(criterion::Throughput::Elements(total_updates));

    let name = format!("hash_map_{n_batches}_batches_of_{batch_size_k}k_updates");
    group.bench_function(&name, |b| {
        b.iter_batched(
            || batches.clone(),
            |batches| {
                let mut map = HashMap::new();
                for batch in batches {
                    map.extend(batch.into_iter());
                }
                map
            },
            BatchSize::SmallInput,
        )
    });

    let name = format!("btree_map_{n_batches}_batches_of_{batch_size_k}k_updates");
    group.bench_function(&name, |b| {
        b.iter_batched(
            || batches.clone(),
            |batches| {
                let mut map = BTreeMap::new();
                for batch in batches {
                    map.extend(batch.into_iter());
                }
                map
            },
            BatchSize::SmallInput,
        )
    });

    let name = format!("layered_map_{n_batches}_batches_of_{batch_size_k}k_updates");
    group.bench_function(&name, |b| {
        b.iter_batched(
            || batches.clone(),
            |batches| {
                let root_layer = MapLayer::new_family("bench");
                let mut latest_layer = root_layer.clone();
                for batch in batches {
                    latest_layer = latest_layer
                        .view_layers_after(&root_layer)
                        .new_layer(&batch)
                }
                (root_layer, latest_layer)
            },
            BatchSize::SmallInput,
        )
    });
}

fn get(
    group: &mut BenchmarkGroup<WallTime>,
    map_size_k: usize,
    items: &[(Key, Value)],
    keys_to_get: &[Key],
) {
    assert_eq!(map_size_k * K, items.len());
    group.throughput(criterion::Throughput::Elements(keys_to_get.len() as u64));

    let name = format!("hash_map_{map_size_k}k_items");
    let map: HashMap<Key, Value> = items.iter().cloned().collect();
    group.bench_function(&name, |b| {
        b.iter_batched(
            || (),
            |_| keys_to_get.iter().map(|key| map.get(key)).collect_vec(),
            BatchSize::SmallInput,
        )
    });

    let name = format!("btree_map_{map_size_k}k_items");
    let map: BTreeMap<Key, Value> = items.iter().cloned().collect();
    group.bench_function(&name, |b| {
        b.iter_batched(
            || (),
            |_| keys_to_get.iter().map(|key| map.get(key)).collect_vec(),
            BatchSize::SmallInput,
        )
    });

    let name = format!("layered_map_{map_size_k}k_items");
    let root_layer = MapLayer::new_family("bench");
    let top_layer = root_layer.view_layers_after(&root_layer).new_layer(items);
    let map = top_layer.into_layers_view_after(root_layer);
    group.bench_function(&name, |b| {
        b.iter_batched(
            || (),
            |_| keys_to_get.iter().map(|key| map.get(key)).collect_vec(),
            BatchSize::SmallInput,
        )
    });
}

fn get_existing(group: &mut BenchmarkGroup<WallTime>, map_size_k: usize) {
    let items = gen_update_batches(map_size_k, 1).pop().unwrap();
    let num_keys_to_get = map_size_k.min(10) * K;
    let keys_to_get = items
        .iter()
        .map(|(key, _v)| *key)
        .take(num_keys_to_get)
        .collect_vec();
    group.throughput(criterion::Throughput::Elements(num_keys_to_get as u64));

    get(group, map_size_k, &items, &keys_to_get);
}

fn get_non_existing(group: &mut BenchmarkGroup<WallTime>, map_size_k: usize) {
    let items = gen_update_batches(map_size_k, 1).pop().unwrap();
    let num_keys_to_get = map_size_k.min(10) * K;
    let keys_to_get = (0..num_keys_to_get).map(|_| random()).collect_vec();

    get(group, map_size_k, &items, &keys_to_get);
}

fn compare_maps(c: &mut Criterion) {
    {
        let mut group = c.benchmark_group("insert_in_batches");
        for batch_size_k in [1, 10, 100] {
            for n_batches in [1, 8] {
                insert_in_batches(&mut group, batch_size_k, n_batches);
            }
        }
    }

    {
        let mut group = c.benchmark_group("get_existing");
        for map_size_k in [100, 1000, 128_000] {
            get_existing(&mut group, map_size_k);
        }
    }

    {
        let mut group = c.benchmark_group("get_non_existing");
        for map_size_k in [100, 1000, 128_000] {
            get_non_existing(&mut group, map_size_k);
        }
    }
}

criterion_group!(
    name = maps;
    config = Criterion::default();
    targets = compare_maps
);

criterion_main!(maps);
