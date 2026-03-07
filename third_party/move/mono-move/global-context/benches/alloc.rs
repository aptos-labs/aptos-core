// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use bumpalo::Bump;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use global_context::{ArenaPool, GlobalArena};
use parking_lot::Mutex;
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

const ALLOC_COUNTS: &[usize] = &[100];
const THREAD_COUNTS: &[usize] = &[1, 2, 4, 8];

fn hash_str(s: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

fn run_threads<F>(num_threads: usize, make_thread: F) -> Duration
where
    F: Fn(usize, Arc<Barrier>) -> thread::JoinHandle<Duration>,
{
    let barrier = Arc::new(Barrier::new(num_threads));
    let handles: Vec<_> = (0..num_threads)
        .map(|tid| make_thread(tid, barrier.clone()))
        .collect();
    handles
        .into_iter()
        .map(|h| h.join().unwrap())
        .max()
        .unwrap()
}

fn bench_scenario<F>(c: &mut Criterion, group_name: &str, mut make_iter: F)
where
    F: FnMut(usize, usize, u64) -> Duration,
{
    let mut group = c.benchmark_group(group_name);
    for &num_threads in THREAD_COUNTS {
        for &allocations_per_thread in ALLOC_COUNTS {
            group.throughput(Throughput::Elements(
                (allocations_per_thread * num_threads) as u64,
            ));
            group.bench_with_input(
                BenchmarkId::from_parameter(format!(
                    "threads={num_threads}/allocations={allocations_per_thread}"
                )),
                &(num_threads, allocations_per_thread),
                |b, &(num_threads, allocations_per_thread)| {
                    b.iter_custom(|iters| make_iter(num_threads, allocations_per_thread, iters));
                },
            );
        }
    }
    group.finish();
}

/// Scenario 1: all threads share a single mutex-protected arena.
fn bench_single_mutex(c: &mut Criterion) {
    bench_scenario(
        c,
        "arena_pool/single_mutex",
        |num_threads, allocations_per_thread, iters| {
            let bump = Arc::new(Mutex::new(Bump::new()));
            run_threads(num_threads, |_tid, barrier| {
                let bump = bump.clone();
                let strings: Vec<String> = (0..allocations_per_thread)
                    .map(|i| format!("data_{}", i))
                    .collect();
                thread::spawn(move || {
                    barrier.wait();
                    let start = Instant::now();
                    for _ in 0..iters {
                        for s in &strings {
                            bump.lock().alloc_str(s);
                        }
                    }
                    start.elapsed()
                })
            })
        },
    );
}

/// Scenario 2: Pool of arenas with hash-based shard selection per allocation.
fn bench_random_shard(c: &mut Criterion) {
    bench_scenario(
        c,
        "arena_pool/random_shard",
        |num_threads, allocations_per_thread, iters| {
            let pool = Arc::new(ArenaPool::with_num_arenas(8 * num_threads));
            run_threads(num_threads, |_tid, barrier| {
                let pool = pool.clone();
                let entries: Vec<(usize, String)> = (0..allocations_per_thread)
                    .map(|i| {
                        let s = format!("data_{}", i);
                        let shard = (hash_str(&s) as usize) & (8 * num_threads - 1);
                        (shard, s)
                    })
                    .collect();
                thread::spawn(move || {
                    barrier.wait();
                    let start = Instant::now();
                    for _ in 0..iters {
                        for (shard, s) in &entries {
                            pool.lock_arena_blocking(*shard).alloc_str(s);
                        }
                    }
                    start.elapsed()
                })
            })
        },
    );
}

/// Scenario 3: Each thread pre-acquires its own dedicated shard guard once and
/// holds it for the entire duration - no per-allocation lock acquisition.
fn bench_assigned_shard(c: &mut Criterion) {
    bench_scenario(
        c,
        "arena_pool/assigned_shard",
        |num_threads, allocations_per_thread, iters| {
            let pool = Arc::new(ArenaPool::with_num_arenas(num_threads));
            run_threads(num_threads, |tid, barrier| {
                let pool = pool.clone();
                let strings: Vec<String> = (0..allocations_per_thread)
                    .map(|i| format!("data_{}", i))
                    .collect();
                thread::spawn(move || {
                    // Pre-acquire dedicated shard before the timed section.
                    let guard = pool.lock_arena_blocking(tid);
                    barrier.wait();
                    let start = Instant::now();
                    for _ in 0..iters {
                        for s in &strings {
                            guard.alloc_str(s);
                        }
                    }
                    start.elapsed()
                })
            })
        },
    );
}

criterion_group!(
    benches,
    bench_single_mutex,
    bench_random_shard,
    bench_assigned_shard
);
criterion_main!(benches);
