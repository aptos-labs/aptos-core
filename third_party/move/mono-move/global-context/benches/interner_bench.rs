// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Comprehensive benchmarks for concurrent interner implementations.

mod helpers;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use helpers::{create_interner, create_thread_pool, InternerType};
use rand::{rngs::StdRng, Rng, SeedableRng};
use rayon::prelude::*;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

static CORES: [usize; 5] = [1, 4, 8, 16, 30];

/// Benchmark 1: Pure Read Throughput
///
/// Measures lock-free vs locked reads under 100% cache hit.
fn bench_read_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_throughput");
    group.sample_size(50);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(5));

    for &cores in &CORES{
        for &impl_type in InternerType::all() {
            let bench_name = format!("{}_{}", impl_type.name(), cores);

            group.bench_with_input(
                BenchmarkId::from_parameter(&bench_name),
                &cores,
                |b, &cores| {
                    let interner = create_interner(impl_type, cores);
                    let pool = create_thread_pool(cores, false);

                    // Warmup: intern 10k strings
                    pool.install(|| {
                        (0..10_000).into_par_iter().for_each(|i| {
                            black_box(interner.intern_string(&format!("string_{}", i)));
                        });
                    });

                    // Benchmark: 100% cache hit
                    b.iter_custom(|iters| {
                        let start = Instant::now();

                        pool.install(|| {
                            (0..iters).into_par_iter().for_each(|_| {
                                // Each iteration: 100 lookups
                                for i in 0..100 {
                                    let idx = i % 10_000;
                                    black_box(interner.intern_string(&format!("string_{}", idx)));
                                }
                            });
                        });

                        start.elapsed()
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark 2: Pure Write Throughput
///
/// Measures arena allocation performance under 100% cache miss.
fn bench_write_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("write_throughput");
    group.sample_size(30);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    for &cores in &CORES {
        for &impl_type in InternerType::all() {
            let bench_name = format!("{}_{}", impl_type.name(), cores);

            group.bench_with_input(
                BenchmarkId::from_parameter(&bench_name),
                &cores,
                |b, &cores| {
                    let pool = create_thread_pool(cores, false);

                    b.iter_custom(|iters| {
                        let interner = create_interner(impl_type, cores);
                        let counter = Arc::new(AtomicUsize::new(0));

                        let start = Instant::now();

                        pool.install(|| {
                            (0..iters).into_par_iter().for_each(|_| {
                                // Each iteration: 100 unique allocations
                                for _ in 0..100 {
                                    let id = counter.fetch_add(1, Ordering::Relaxed);
                                    black_box(interner.intern_string(&format!("unique_{}", id)));
                                }
                            });
                        });

                        start.elapsed()
                    });
                },
            );
        }
    }

    group.finish();
}

/// Benchmark 3: Mixed Workload
///
/// Measures interaction effects with varying read/write ratios.
fn bench_mixed_workload(c: &mut Criterion) {
    let mut group = c.benchmark_group("mixed_workload");
    group.sample_size(30);
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));

    for &read_pct in &[50, 75, 90, 95, 99] {
        for &cores in &CORES {
            for &impl_type in InternerType::all() {
                let bench_name = format!("{}_{}%read_{}", impl_type.name(), read_pct, cores);

                group.bench_with_input(
                    BenchmarkId::from_parameter(&bench_name),
                    &(cores, read_pct),
                    |b, &(cores, read_pct)| {
                        let interner = create_interner(impl_type, cores);
                        let pool = create_thread_pool(cores, false);

                        // Pre-populate for reads
                        pool.install(|| {
                            (0..1000).into_par_iter().for_each(|i| {
                                interner.intern_string(&format!("common_{}", i));
                            });
                        });

                        b.iter_custom(|iters| {
                            let write_counter = Arc::new(AtomicUsize::new(0));

                            let start = Instant::now();

                            pool.install(|| {
                                (0..iters).into_par_iter().for_each(|_| {
                                    let mut rng = StdRng::seed_from_u64(
                                        rayon::current_thread_index().unwrap_or(0) as u64,
                                    );

                                    for _ in 0..100 {
                                        let r: u32 = rng.gen_range(0..100);

                                        if r < read_pct {
                                            // Read path
                                            let idx = rng.gen_range(0..1000);
                                            black_box(
                                                interner.intern_string(&format!("common_{}", idx)),
                                            );
                                        } else {
                                            // Write path
                                            let id = write_counter.fetch_add(1, Ordering::Relaxed);
                                            black_box(
                                                interner.intern_string(&format!("unique_{}", id)),
                                            );
                                        }
                                    }
                                });
                            });

                            start.elapsed()
                        });
                    },
                );
            }
        }
    }

    group.finish();
}

/// Benchmark 4: Warmup Performance
///
/// Measures cold-start to steady-state transition.
fn bench_warmup(c: &mut Criterion) {
    let mut group = c.benchmark_group("warmup");
    group.sample_size(20);
    group.measurement_time(Duration::from_secs(10));

    let cores = 8;

    for &impl_type in InternerType::all() {
        group.bench_function(impl_type.name(), |b| {
            let pool = create_thread_pool(cores, false);

            b.iter(|| {
                let interner = create_interner(impl_type, cores);

                pool.install(|| {
                    // Phase 1: Cold start (100 txns, 70% writes)
                    (0..100).into_par_iter().for_each(|txn| {
                        let mut rng = StdRng::seed_from_u64(txn as u64);
                        for i in 0..100 {
                            if i < 70 {
                                // Write
                                interner.intern_string(&format!("txn_{}_item_{}", txn, i));
                            } else {
                                // Read
                                let idx = rng.gen_range(0..10);
                                interner.intern_string(&format!("common_{}", idx));
                            }
                        }
                    });

                    // Phase 2: Steady state (900 txns, 99% reads)
                    (0..900).into_par_iter().for_each(|txn| {
                        let mut rng = StdRng::seed_from_u64(txn as u64 + 1000);
                        for i in 0..100 {
                            if i == 0 {
                                // 1% writes
                                interner.intern_string(&format!("rare_{}", txn));
                            } else {
                                // 99% reads
                                let idx = rng.gen_range(0..1000);
                                interner.intern_string(&format!("common_{}", idx));
                            }
                        }
                    });
                });

                black_box(&interner);
            });
        });
    }

    group.finish();
}

/// Benchmark 5: Latency Distribution
///
/// Measures tail latencies (P50, P90, P99, P99.9, P99.99).
fn bench_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("latency");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(1000);

    // Set thread index for the main benchmark thread (required for per-thread array interner)
    helpers::set_thread_index_for_current(0);

    for &impl_type in InternerType::all() {
        // Read latency
        group.bench_function(format!("{}_read", impl_type.name()), |b| {
            let interner = create_interner(impl_type, 1);

            // Warmup
            for i in 0..10_000 {
                interner.intern_string(&format!("string_{}", i));
            }

            let mut rng = StdRng::seed_from_u64(42);

            b.iter(|| {
                let idx = rng.gen_range(0..10_000);
                black_box(interner.intern_string(&format!("string_{}", idx)))
            });
        });

        // Write latency
        group.bench_function(format!("{}_write", impl_type.name()), |b| {
            let counter = AtomicUsize::new(0);

            b.iter(|| {
                let interner = create_interner(impl_type, 1);
                let id = counter.fetch_add(1, Ordering::Relaxed);
                black_box(interner.intern_string(&format!("unique_{}", id)))
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_read_throughput,
    bench_write_throughput,
    bench_mixed_workload,
    bench_warmup,
    bench_latency,
);

criterion_main!(benches);
