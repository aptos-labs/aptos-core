// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_types::{
    account_address::AccountAddress,
    account_config::AccountResource,
    event::{EventHandle, EventKey},
    state_store::{state_key::StateKey, state_value::StateValue, NUM_STATE_SHARDS},
    transaction::Version,
    write_set::{WriteOp, WriteSet},
};
use arr_macro::arr;
use criterion::{criterion_group, criterion_main, Criterion};
use itertools::Itertools;
use rayon::prelude::*;
use std::collections::HashMap;

fn empty_shards<T: Default>() -> [T; NUM_STATE_SHARDS] {
    arr![T::default(); 16]
}

fn sharded_per_version_vecs<T: Clone + Default>(num_versions: usize) -> [Vec<T>; NUM_STATE_SHARDS] {
    arr![vec![T::default(); num_versions]; 16]
}

fn collect_write_sets(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_write_sets");

    group.bench_function("refs", |b| {
        b.iter(|| write_sets.iter().collect::<Vec<&WriteSet>>())
    });

    // 120x slower
    group.bench_function("par_refs", |b| {
        b.iter(|| write_sets.par_iter().collect::<Vec<&WriteSet>>())
    });

    group.bench_function("cloned", |b| b.iter(|| write_sets.to_vec()));

    // 3x speed up
    group.bench_function("par_cloned", |b| {
        b.iter(|| write_sets.par_iter().cloned().collect::<Vec<WriteSet>>())
    });
}

fn collect_per_version_write_op_refs(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_per_version_write_op_refs");

    group.bench_function("vecs", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.write_op_iter().collect_vec())
                .collect::<Vec<Vec<(&StateKey, &WriteOp)>>>()
        })
    });

    // little slow down
    group.bench_function("vecs_key_cloned", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.write_op_iter().map(|(k, v)| (k.clone(), v)).collect_vec())
                .collect::<Vec<Vec<(StateKey, &WriteOp)>>>()
        })
    });

    // little speed up
    group.bench_function("par_vecs", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| w.write_op_iter().collect_vec())
                .collect::<Vec<Vec<(&StateKey, &WriteOp)>>>()
        })
    });

    // 4x compared to vecs
    group.bench_function("maps", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.write_op_iter().collect())
                .collect::<Vec<HashMap<&StateKey, &WriteOp>>>()
        })
    });

    // 3x speed up
    group.bench_function("par_maps", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| w.write_op_iter().collect())
                .collect::<Vec<HashMap<&StateKey, &WriteOp>>>()
        })
    });
}

fn collect_per_version_state_value_refs(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_per_version_state_value_refs");

    // similar to write op refs, little slower
    group.bench_function("vecs", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.state_update_refs().collect())
                .collect::<Vec<Vec<(&StateKey, Option<&StateValue>)>>>()
        })
    });

    // little speed up
    group.bench_function("par_vecs", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| w.state_update_refs().collect())
                .collect::<Vec<Vec<(&StateKey, Option<&StateValue>)>>>()
        })
    });

    // 3x+ compared to vecs
    group.bench_function("maps", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.state_update_refs().collect())
                .collect::<Vec<HashMap<&StateKey, Option<&StateValue>>>>()
        })
    });

    // 3x speed up makes it comparable to non-par vecs
    group.bench_function("par_maps", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| w.state_update_refs().collect())
                .collect::<Vec<HashMap<&StateKey, Option<&StateValue>>>>()
        })
    });

    // conclusion: not too much different than write op refs
}

fn collect_per_version_sharded_state_values(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_per_version_sharded_state_values");

    // 3x non-sharded
    group.bench_function("vecs", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|write_set| {
                    let mut ret = empty_shards::<Vec<_>>();
                    for (k, v) in write_set.state_update_refs() {
                        ret[k.get_shard_id()].push((k, v));
                    }
                    ret
                })
                .collect::<Vec<[Vec<(&StateKey, Option<&StateValue>)>; NUM_STATE_SHARDS]>>()
        })
    });

    // 1.5x speed up
    group.bench_function("par_vecs", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|write_set| {
                    let mut ret = empty_shards::<Vec<_>>();
                    for (k, v) in write_set.state_update_refs() {
                        ret[k.get_shard_id()].push((k, v));
                    }
                    ret
                })
                .collect::<Vec<[Vec<(&StateKey, Option<&StateValue>)>; NUM_STATE_SHARDS]>>()
        })
    });

    group.bench_function("par_vecs_cloned", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|write_set| {
                    let mut ret = empty_shards::<Vec<_>>();
                    for (k, v) in write_set.state_updates_cloned() {
                        ret[k.get_shard_id()].push((k, v));
                    }
                    ret
                })
                .collect::<Vec<[Vec<(StateKey, Option<StateValue>)>; NUM_STATE_SHARDS]>>()
        })
    });

    // 2x time compared to vecs
    group.bench_function("maps", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|write_set| {
                    let mut ret = empty_shards::<HashMap<_, _>>();
                    for (k, v) in write_set.state_update_refs() {
                        ret[k.get_shard_id()].insert(k, v);
                    }
                    ret
                })
                .collect::<Vec<[HashMap<&StateKey, Option<&StateValue>>; NUM_STATE_SHARDS]>>()
        })
    });

    // 2x time compared to non-par, makes it comparable to non-par vecs
    group.bench_function("par_maps", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|write_set| {
                    let mut ret = empty_shards::<HashMap<_, _>>();
                    for (k, v) in write_set.state_update_refs() {
                        ret[k.get_shard_id()].insert(k, v);
                    }
                    ret
                })
                .collect::<Vec<[HashMap<&StateKey, Option<&StateValue>>; NUM_STATE_SHARDS]>>()
        })
    });
}

fn collect_sharded_per_version_state_value_refs(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_sharded_per_version_state_value_refs");

    group.bench_function("vecs", |b| {
        b.iter(|| {
            let mut ret: [Vec<Vec<(&StateKey, Option<&StateValue>)>>; NUM_STATE_SHARDS] =
                sharded_per_version_vecs::<Vec<_>>(write_sets.len());

            write_sets.iter().enumerate().for_each(|(idx, write_set)| {
                for (k, v) in write_set.state_update_refs() {
                    ret[k.get_shard_id()][idx].push((k, v));
                }
            });
            ret
        })
    });

    // worse than non-par
    group.bench_function("par_vecs", |b| {
        b.iter(|| {
            let mut ret: [Vec<Vec<(&StateKey, Option<&StateValue>)>>; NUM_STATE_SHARDS] =
                sharded_per_version_vecs::<Vec<_>>(write_sets.len());

            ret.par_iter_mut()
                .enumerate()
                .for_each(|(shard_id, shard)| {
                    write_sets.iter().enumerate().for_each(|(idx, write_set)| {
                        for (k, v) in write_set.state_update_refs() {
                            if k.get_shard_id() == shard_id {
                                shard[idx].push((k, v));
                            }
                        }
                    });
                });
            ret
        })
    });

    group.bench_function("maps", |b| {
        b.iter(|| {
            let mut ret: [Vec<HashMap<&StateKey, Option<&StateValue>>>; NUM_STATE_SHARDS] =
                sharded_per_version_vecs::<HashMap<_, _>>(write_sets.len());

            write_sets.iter().enumerate().for_each(|(idx, write_set)| {
                for (k, v) in write_set.state_update_refs() {
                    ret[k.get_shard_id()][idx].insert(k, v);
                }
            });
            ret
        })
    });
}

fn collect_sharded_state_updates_with_version(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_sharded_state_updates_with_version");

    group.bench_function("non_par", |b| {
        b.iter(|| {
            let first_version: Version = 100;
            let mut ret: [Vec<(Version, &StateKey, Option<&StateValue>)>; NUM_STATE_SHARDS] =
                arr![Vec::with_capacity(write_sets.len()); 16];
            write_sets.iter().enumerate().for_each(|(idx, write_set)| {
                for (k, v) in write_set.state_update_refs() {
                    ret[k.get_shard_id()].push((first_version + idx as Version, k, v));
                }
            });

            ret
        })
    });
}

fn collect_per_version_cloned_write_ops(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_per_version_cloned_write_ops");

    // 4x compared to refs
    group.bench_function("vecs", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| {
                    w.write_op_iter()
                        .map(|(k, op)| (k.clone(), op.clone()))
                        .collect()
                })
                .collect::<Vec<Vec<(StateKey, WriteOp)>>>()
        })
    });

    // 2.5x speed up
    group.bench_function("par_vecs", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| {
                    w.write_op_iter()
                        .map(|(k, op)| (k.clone(), op.clone()))
                        .collect()
                })
                .collect::<Vec<Vec<(StateKey, WriteOp)>>>()
        })
    });

    // 1.7x compared to vecs
    group.bench_function("maps", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| {
                    w.write_op_iter()
                        .map(|(k, op)| (k.clone(), op.clone()))
                        .collect()
                })
                .collect::<Vec<HashMap<StateKey, WriteOp>>>()
        })
    });

    // 3x speed up
    group.bench_function("par_maps", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| {
                    w.write_op_iter()
                        .map(|(k, op)| (k.clone(), op.clone()))
                        .collect()
                })
                .collect::<Vec<HashMap<StateKey, WriteOp>>>()
        })
    });
}

fn collect_per_version_cloned_state_values(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_per_version_cloned_state_values");

    // 3.5x compared to refs
    group.bench_function("vecs", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.state_updates_cloned().collect())
                .collect::<Vec<Vec<(StateKey, Option<StateValue>)>>>()
        })
    });

    // 2x+ speed up
    group.bench_function("par_vecs", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| w.state_updates_cloned().collect())
                .collect::<Vec<Vec<(StateKey, Option<StateValue>)>>>()
        })
    });

    // a little slower than vecs
    group.bench_function("maps", |b| {
        b.iter(|| {
            write_sets
                .iter()
                .map(|w| w.state_updates_cloned().collect())
                .collect::<Vec<Vec<(StateKey, Option<StateValue>)>>>()
        })
    });

    // 2.5x speed up
    group.bench_function("par_maps", |b| {
        b.iter(|| {
            write_sets
                .par_iter()
                .map(|w| w.state_updates_cloned().collect())
                .collect::<Vec<Vec<(StateKey, Option<StateValue>)>>>()
        })
    });

    // conclusion: cloning dominates the cost, difference between vecs and maps is no longer significant.
}

fn collect_state_update_maps(c: &mut Criterion, write_sets: &[WriteSet]) {
    let mut group = c.benchmark_group("collect_state_update_maps");

    group.bench_function("from_write_sets", |b| {
        b.iter(|| {
            let mut ret = HashMap::with_capacity(write_sets.len() * 2);
            ret.extend(write_sets.iter().flat_map(|w| w.state_update_refs()));
            ret
        })
    });

    let per_version_sharded = write_sets
        .iter()
        .map(|write_set| {
            let mut ret = empty_shards::<Vec<_>>();
            for (k, v) in write_set.state_update_refs() {
                ret[k.get_shard_id()].push((k, v));
            }
            ret
        })
        .collect::<Vec<[Vec<(&StateKey, Option<&StateValue>)>; NUM_STATE_SHARDS]>>();

    // 2x the time, but the majority of the time is combining the shards.
    //   i.e. if we can delete the combining to a later stage it might be beneficial to operate
    //        in a sharded way all the way
    group.bench_function("from_per_version_sharded", |b| {
        b.iter(|| {
            let mut update_shards = empty_shards::<HashMap<&StateKey, Option<&StateValue>>>();
            update_shards
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, shard)| {
                    shard.extend(
                        per_version_sharded
                            .iter()
                            .flat_map(|ver_shards| ver_shards[i].iter().map(|(k, v)| (*k, *v))),
                    )
                });
            let mut ret = HashMap::with_capacity(write_sets.len() * 2);
            ret.extend(update_shards.into_iter().flatten());
            ret
        })
    });

    group.bench_function("from_per_version_sharded_to_sharded", |b| {
        b.iter(|| {
            let mut ret = empty_shards::<HashMap<&StateKey, Option<&StateValue>>>();
            ret.par_iter_mut().enumerate().for_each(|(i, shard)| {
                shard.extend(
                    per_version_sharded
                        .iter()
                        .flat_map(|ver_shards| ver_shards[i].iter().map(|(k, v)| (*k, *v))),
                )
            });
            ret
        })
    });

    let sharded_with_version: [Vec<(Version, &StateKey, Option<&StateValue>)>; NUM_STATE_SHARDS] = {
        let first_version: Version = 100;
        let mut ret = arr![Vec::with_capacity(write_sets.len()); 16];
        write_sets.iter().enumerate().for_each(|(idx, write_set)| {
            for (k, v) in write_set.state_update_refs() {
                ret[k.get_shard_id()].push((first_version + idx as Version, k, v));
            }
        });
        ret
    };

    group.bench_function("from_sharded_with_version_to_sharded", |b| {
        b.iter(|| {
            let mut ret = empty_shards::<HashMap<&StateKey, Option<&StateValue>>>();
            ret.par_iter_mut()
                .zip(sharded_with_version.par_iter())
                .for_each(|(out_shard, in_shards)| {
                    *out_shard = in_shards.iter().map(|(_ver, k, v)| (*k, *v)).collect();
                });
        })
    });
}

fn targets(c: &mut Criterion) {
    rayon::ThreadPoolBuilder::new()
        .num_threads(32)
        .thread_name(|index| format!("rayon-global-{}", index))
        .build_global()
        .expect("Failed to build rayon global thread pool.");

    let account_resource = AccountResource::new(
        0,
        vec![0; 32],
        EventHandle::new(EventKey::new(0, AccountAddress::random()), 0),
        EventHandle::new(EventKey::new(1, AccountAddress::random()), 0),
    );
    let value_bytes = bcs::to_bytes(&account_resource).unwrap();

    let write_sets = (0..10000usize)
        .map(|idx| {
            let ws_size = idx % 10;
            WriteSet::new_for_test(
                std::iter::repeat_with(|| {
                    (
                        StateKey::resource_typed::<AccountResource>(&AccountAddress::random())
                            .unwrap(),
                        Some(StateValue::new_legacy(value_bytes.clone().into())),
                    )
                })
                .take(ws_size),
            )
        })
        .collect_vec();

    collect_write_sets(c, &write_sets);
    collect_per_version_write_op_refs(c, &write_sets);
    collect_per_version_state_value_refs(c, &write_sets);
    collect_per_version_cloned_write_ops(c, &write_sets);
    collect_per_version_cloned_state_values(c, &write_sets);
    collect_per_version_sharded_state_values(c, &write_sets);
    collect_sharded_per_version_state_value_refs(c, &write_sets);
    collect_sharded_state_updates_with_version(c, &write_sets);
    collect_state_update_maps(c, &write_sets);
}

criterion_group!(
    name = data_collection;
    config = Criterion::default();
    targets = targets,
);

criterion_main!(data_collection);
