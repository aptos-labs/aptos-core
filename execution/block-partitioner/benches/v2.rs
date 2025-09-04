// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[macro_use]
extern crate criterion;

use aptos_block_partitioner::{
    BlockPartitioner, pre_partition::connected_component::ConnectedComponentPartitioner,
    test_utils::P2PBlockGenerator, v2::PartitionerV2,
};
use criterion::Criterion;
use rand::thread_rng;

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("v2");

    let num_accounts = 10000;
    let block_size = 1000;
    let num_shards = 5;

    let num_threads = 8;
    let num_rounds_limit = 4;
    let avoid_pct = 0.9;
    let dashmap_num_shards = 64;
    let merge_discards = true;

    let mut rng = thread_rng();
    let block_gen = P2PBlockGenerator::new(num_accounts);
    let partitioner = PartitionerV2::new(
        num_threads,
        num_rounds_limit,
        avoid_pct,
        dashmap_num_shards,
        merge_discards,
        Box::new(ConnectedComponentPartitioner {
            load_imbalance_tolerance: 2.0,
        }),
    );
    group.bench_function(format!("acc={num_accounts},blk={block_size},shd={num_shards}/thr={num_threads},rnd={num_rounds_limit},avd={avoid_pct},mds={merge_discards}"), move |b| {
        b.iter_with_setup(
            || {
                block_gen.rand_block(&mut rng, block_size)
            },
            |txns| {
                let _txns = partitioner.partition(txns, num_shards);
            },
        )
    });
    group.finish();
}

criterion_group!(
    name = v2_benches;
    config = Criterion::default(); //.measurement_time(Duration::from_secs(100));
    targets = bench_group);
criterion_main!(v2_benches);
