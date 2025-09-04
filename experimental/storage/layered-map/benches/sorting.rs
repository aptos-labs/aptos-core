// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use bitvec::{index::BitIdx, prelude::*};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use itertools::Itertools;

#[cfg(unix)]
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

fn recursive_bin_search(sorted_data: &[u64], depth: u8) {
    if sorted_data.len() <= 1 || depth >= 64 {
        return;
    }

    let pivot = sorted_data.partition_point(|key| get_bit(*key, depth));
    recursive_bin_search(&sorted_data[..pivot], depth + 1);
    recursive_bin_search(&sorted_data[pivot..], depth + 1);
}

fn get_bit(value: u64, bit: u8) -> bool {
    value.get_bit::<Msb0>(BitIdx::new(bit).unwrap())
}

fn partition(data: &mut [u64], depth: u8) -> usize {
    if data.is_empty() {
        return 0;
    }

    let mut zero_cur = 0;
    let mut one_cur = data.len() - 1;

    loop {
        while zero_cur <= one_cur && !get_bit(data[zero_cur], depth) {
            zero_cur += 1;
        }
        while one_cur > zero_cur && get_bit(data[one_cur], depth) {
            one_cur -= 1;
        }
        if zero_cur >= one_cur {
            return zero_cur;
        }
        data.swap(zero_cur, one_cur);
    }
}

fn recursive_partition(data: &mut [u64], depth: u8) {
    if data.len() <= 1 || depth >= 64 {
        return;
    }

    let pivot = partition(data, depth);
    recursive_partition(&mut data[..pivot], depth + 1);
    recursive_partition(&mut data[pivot..], depth + 1);
}

fn partition_stable(data: &mut [u64], buffer: &mut [u64], depth: u8) -> usize {
    if buffer.is_empty() {
        return 0;
    }

    let mut zero_cur = 0;
    let mut one_cur = 0;
    for cur in 0..data.len() {
        if !get_bit(data[cur], depth) {
            // zero
            data[zero_cur] = data[cur];
            zero_cur += 1;
        } else {
            buffer[one_cur] = data[cur];
            one_cur += 1;
        }
    }
    data[zero_cur..].copy_from_slice(&buffer[..one_cur]);
    zero_cur
}

fn recursive_partition_stable(data: &mut [u64], buffer: &mut [u64], depth: u8) {
    if data.len() <= 1 || depth >= 64 {
        return;
    }

    let pivot = partition_stable(data, buffer, depth);
    recursive_partition_stable(&mut data[..pivot], &mut buffer[..pivot], depth + 1);
    recursive_partition_stable(&mut data[pivot..], &mut buffer[pivot..], depth + 1);
}

fn compare_sorting(c: &mut Criterion) {
    let mut group = c.benchmark_group("sorting");

    const SET_SIZE: usize = 100000;

    let data = std::iter::repeat_with(rand::random::<u64>)
        .take(SET_SIZE)
        .collect_vec();
    group.throughput(criterion::Throughput::Elements(SET_SIZE as u64));

    let mut data_sorted = data.clone();
    data_sorted.sort();

    group.bench_function("sort_then_bin_search", |b| {
        b.iter_batched(
            || data.clone(),
            |mut data| {
                data.sort();
                recursive_bin_search(&data, 0);
                data
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("recursive_partition", |b| {
        b.iter_batched(
            || data.clone(),
            |mut data| {
                recursive_partition(&mut data, 0);
                if data != data_sorted {
                    panic!()
                }
                data
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("recursive_partition_stable", |b| {
        b.iter_batched(
            || data.clone(),
            |mut data| {
                let mut buffer = vec![0; data.len()];
                recursive_partition_stable(&mut data, &mut buffer, 0);
                if data != data_sorted {
                    panic!()
                }
                data
            },
            BatchSize::SmallInput,
        )
    });

    let mut data = data;
    data.sort();

    group.bench_function("sort_then_bin_search_pre_sorted", |b| {
        b.iter_batched(
            || data.clone(),
            |mut data| {
                data.sort();
                recursive_bin_search(&data, 0);
                data
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("bin_search_pre_sorted", |b| {
        b.iter_batched(
            || data.clone(),
            |data| {
                recursive_bin_search(&data, 0);
                data
            },
            BatchSize::SmallInput,
        );
    });

    group.bench_function("recursive_partition_pre_sorted", |b| {
        b.iter_batched(
            || data.clone(),
            |mut data| {
                recursive_partition(&mut data, 0);
                if data != data_sorted {
                    panic!()
                }
                data
            },
            BatchSize::SmallInput,
        )
    });

    group.bench_function("recursive_partition_stable_pre_sorted", |b| {
        b.iter_batched(
            || data.clone(),
            |mut data| {
                let mut buffer = vec![0; data.len()];
                recursive_partition_stable(&mut data, &mut buffer, 0);
                if data != data_sorted {
                    panic!()
                }
                data
            },
            BatchSize::SmallInput,
        )
    });
}

criterion_group!(
    name = sorting;
    config = Criterion::default();
    targets = compare_sorting
);

criterion_main!(sorting);
