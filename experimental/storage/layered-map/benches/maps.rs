// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use velor_crypto::HashValue;
use velor_experimental_layered_map::{LayeredMap, MapLayer};
use velor_schemadb::{
    batch::WriteBatch,
    define_schema,
    schema::{KeyCodec, ValueCodec},
    DB, DEFAULT_COLUMN_FAMILY_NAME,
};
use velor_temppath::TempPath;
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BatchSize, BenchmarkGroup, Criterion,
};
use itertools::Itertools;
use once_cell::sync::OnceCell;
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

define_schema!(DbMap, Key, Value, DEFAULT_COLUMN_FAMILY_NAME);

impl KeyCodec<DbMap> for Key {
    fn encode_key(&self) -> Result<Vec<u8>> {
        Ok(self.to_be_bytes().to_vec())
    }

    fn decode_key(data: &[u8]) -> Result<Self> {
        assert_eq!(data.len(), 16);

        let mut buffer = [0u8; 16];
        buffer.copy_from_slice(data);
        Ok(Self::from_be_bytes(buffer))
    }
}

impl ValueCodec<DbMap> for Value {
    fn encode_value(&self) -> Result<Vec<u8>> {
        Ok(self.to_vec())
    }

    fn decode_value(data: &[u8]) -> Result<Self> {
        assert_eq!(data.len(), 32);
        Ok(Self::from_slice(data)?)
    }
}

const K: usize = 1024;

type BatchCache = HashMap<usize, HashMap<usize, Vec<Vec<(Key, Value)>>>>;

fn gen_update_batches(
    cache: &mut BatchCache,
    batch_size_k: usize,
    n_batches: usize,
) -> &Vec<Vec<(Key, Value)>> {
    cache
        .entry(batch_size_k)
        .or_default()
        .entry(n_batches)
        .or_insert_with(|| {
            println!();
            println!("    Generating batch. {batch_size_k}k per batch, {n_batches} batches.");
            let timer = std::time::Instant::now();
            let ret = repeat_with(|| {
                repeat_with(|| (random(), random()))
                    .take(batch_size_k * K)
                    .collect_vec()
            })
            .take(n_batches)
            .collect_vec();
            println!("    Done in {} secs.", timer.elapsed().as_secs());
            ret
        })
}

fn insert_in_batches(
    group: &mut BenchmarkGroup<WallTime>,
    cache: &mut BatchCache,
    batch_size_k: usize,
    n_batches: usize,
) {
    let total_updates = (batch_size_k * K * n_batches) as u64;
    group.throughput(criterion::Throughput::Elements(total_updates));

    let name = format!("hash_map_{n_batches}_batches_of_{batch_size_k}k_updates");
    group.bench_function(&name, |b| {
        b.iter_batched(
            || gen_update_batches(cache, batch_size_k, n_batches).clone(),
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
            || gen_update_batches(cache, batch_size_k, n_batches).clone(),
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
            || gen_update_batches(cache, batch_size_k, n_batches).clone(),
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
    batch_cache: &mut BatchCache,
    keys_cache: &mut KeysCache,
    map_size_k: usize,
) {
    let n_keys_to_get = map_size_k.min(10) * K;

    group.throughput(criterion::Throughput::Elements(n_keys_to_get as u64));

    get_hash_map(group, batch_cache, keys_cache, map_size_k, n_keys_to_get);

    get_btree_map(group, batch_cache, keys_cache, map_size_k, n_keys_to_get);

    get_layered_map(group, batch_cache, keys_cache, map_size_k, n_keys_to_get);

    get_rocksdb(group, batch_cache, keys_cache, map_size_k, n_keys_to_get);
}

fn get_hash_map(
    group: &mut BenchmarkGroup<WallTime>,
    batch_cache: &mut BatchCache,
    keys_cache: &mut KeysCache,
    map_size_k: usize,
    n_keys_to_get: usize,
) {
    let map: OnceCell<HashMap<Key, Value>> = OnceCell::new();

    for (prefix, existing) in [("existing", true), ("non_existing", false)] {
        let name = format!("{prefix}/hash_map_{map_size_k}k_items");
        let keys: OnceCell<Vec<Key>> = OnceCell::new();

        group.bench_function(&name, |b| {
            b.iter_batched(
                || {
                    let (items, keys_) =
                        gen_get(batch_cache, keys_cache, map_size_k, n_keys_to_get, existing);
                    let map = map.get_or_init(|| items.iter().cloned().collect());
                    let keys = keys.get_or_init(|| keys_.clone());
                    (map, keys)
                },
                |(map, keys)| keys.iter().map(|key| map.get(key)).collect_vec(),
                BatchSize::SmallInput,
            )
        });
    }
}

fn get_btree_map(
    group: &mut BenchmarkGroup<WallTime>,
    batch_cache: &mut BatchCache,
    keys_cache: &mut KeysCache,
    map_size_k: usize,
    n_keys_to_get: usize,
) {
    let map: OnceCell<BTreeMap<Key, Value>> = OnceCell::new();

    for (prefix, existing) in [("existing", true), ("non_existing", false)] {
        let name = format!("{prefix}/btree_map_{map_size_k}k_items");
        let keys: OnceCell<Vec<Key>> = OnceCell::new();

        group.bench_function(&name, |b| {
            b.iter_batched(
                || {
                    let (items, keys_) =
                        gen_get(batch_cache, keys_cache, map_size_k, n_keys_to_get, existing);
                    let map = map.get_or_init(|| items.iter().cloned().collect());
                    let keys = keys.get_or_init(|| keys_.clone());
                    (map, keys)
                },
                |(map, keys)| keys.iter().map(|key| map.get(key)).collect_vec(),
                BatchSize::SmallInput,
            )
        });
    }
}

fn get_layered_map(
    group: &mut BenchmarkGroup<WallTime>,
    batch_cache: &mut BatchCache,
    keys_cache: &mut KeysCache,
    map_size_k: usize,
    n_keys_to_get: usize,
) {
    let map: OnceCell<LayeredMap<Key, Value>> = OnceCell::new();
    for (prefix, existing) in [("existing", true), ("non_existing", false)] {
        let name = format!("{prefix}/layered_map_{map_size_k}k_items");
        let keys: OnceCell<Vec<Key>> = OnceCell::new();

        group.bench_function(&name, |b| {
            b.iter_batched(
                || {
                    let (items, keys_) =
                        gen_get(batch_cache, keys_cache, map_size_k, n_keys_to_get, existing);
                    let map = map.get_or_init(|| {
                        let root_layer = MapLayer::new_family("bench");
                        let top_layer = root_layer.view_layers_after(&root_layer).new_layer(items);
                        top_layer.into_layers_view_after(root_layer)
                    });
                    let keys = keys.get_or_init(|| keys_.clone());
                    (map, keys)
                },
                |(map, keys)| keys.iter().map(|key| map.get(key)).collect_vec(),
                BatchSize::SmallInput,
            )
        });
    }
}

fn get_rocksdb(
    group: &mut BenchmarkGroup<WallTime>,
    batch_cache: &mut BatchCache,
    keys_cache: &mut KeysCache,
    map_size_k: usize,
    n_keys_to_get: usize,
) {
    let map: OnceCell<DB> = OnceCell::new();

    for (prefix, existing) in [("existing", true), ("non_existing", false)] {
        let name = format!("{prefix}/rocksdb_{map_size_k}k_items");
        let keys: OnceCell<Vec<Key>> = OnceCell::new();
        group.bench_function(&name, |b| {
            b.iter_batched(
                || {
                    let (items, keys_) =
                        gen_get(batch_cache, keys_cache, map_size_k, n_keys_to_get, existing);
                    let keys = keys.get_or_init(|| keys_.clone());
                    let map = map.get_or_init(|| {
                        println!();
                        println!("    Opening rocksdb.");
                        let mut db_path = TempPath::new();
                        db_path.persist(); // leak the temp dir

                        let mut options = rocksdb::Options::default();
                        options.create_if_missing(true);

                        let db = DB::open(
                            &db_path,
                            "bench",
                            vec![DEFAULT_COLUMN_FAMILY_NAME],
                            &options,
                        )
                        .unwrap();

                        let mut total = 0;
                        items.iter().chunks(1_000_000).into_iter().for_each(|kvs| {
                            let mut batch = db.new_native_batch();
                            total += kvs
                                .map(|(key, value)| batch.put::<DbMap>(key, value).unwrap())
                                .count();
                            db.write_schemas(batch).unwrap();

                            println!("    Inserted {total}.");
                        });

                        println!("    Flushing mem-tables.");
                        db.flush_cf(DEFAULT_COLUMN_FAMILY_NAME).unwrap();

                        println!("    Preheat all keys to be queried.");
                        keys.iter().for_each(|key| {
                            db.get::<DbMap>(key).unwrap();
                        });

                        println!("    RocksDB ready.");
                        db
                    });

                    (map, keys)
                },
                |(map, keys)| {
                    keys.iter()
                        .map(|key| map.get::<DbMap>(key).unwrap())
                        .collect_vec()
                },
                BatchSize::SmallInput,
            )
        });
    }
}

type KeysCache = HashMap<usize, HashMap<bool, Vec<Key>>>;

fn gen_get<'a>(
    batch_cache: &'a mut BatchCache,
    keys_cache: &'a mut KeysCache,
    map_size_k: usize,
    n_keys_to_get: usize,
    existing: bool,
) -> (&'a Vec<(Key, Value)>, &'a Vec<Key>) {
    let items = &gen_update_batches(batch_cache, map_size_k, 1)[0];
    let keys = keys_cache
        .entry(map_size_k)
        .or_default()
        .entry(existing)
        .or_insert_with(|| {
            if existing {
                items.iter().map(|(k, _v)| *k).take(n_keys_to_get).collect()
            } else {
                repeat_with(random).take(n_keys_to_get).collect()
            }
        });

    (items, keys)
}

fn compare_maps(c: &mut Criterion) {
    let mut batch_cache = BatchCache::default();
    let mut keys_cache = KeysCache::default();

    {
        let mut group = c.benchmark_group("insert_in_batches");
        for batch_size_k in [1, 10, 100] {
            for n_batches in [1, 8] {
                insert_in_batches(&mut group, &mut batch_cache, batch_size_k, n_batches);
            }
        }
    }

    {
        let mut group = c.benchmark_group("get");
        for map_size_k in [100, 1000, 4_000, 16_000, 128_000] {
            get(&mut group, &mut batch_cache, &mut keys_cache, map_size_k);
        }
    }
}

criterion_group!(
    name = maps;
    config = Criterion::default();
    targets = compare_maps
);

criterion_main!(maps);
