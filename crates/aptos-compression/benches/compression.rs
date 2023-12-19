// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_compression::{client::CompressionClient, CompressedData};
use criterion::{
    criterion_group, criterion_main, measurement::WallTime, BenchmarkGroup, BenchmarkId, Criterion,
};
use rand::Rng;
use std::collections::BTreeMap;

// Useful test constants
const KIB: usize = 1024;
const MAX_COMPRESSION_SIZE: usize = 100 * MIB;
const MIB: usize = 1024 * 1024;

// The set of byte lengths to test when benchmarking random byte strings
const BYTE_LENGTHS_TO_TEST: &[(usize, &str)] = &[
    (KIB, "1 KiB"),
    (5 * KIB, "5 KiB"),
    (10 * KIB, "10 KiB"),
    (100 * KIB, "100 KiB"),
    (500 * KIB, "500 KiB"),
    (MIB, "1 MiB"),
    (5 * MIB, "5 MiB"),
    (10 * MIB, "10 MiB"),
    (25 * MIB, "25 MiB"),
    (50 * MIB, "50 MiB"),
];

/// The benchmark for regular compression over random bytes
pub fn benchmark_compression_random(criterion: &mut Criterion) {
    // Get the random bytes to benchmark
    let random_bytes_to_benchmark = get_random_bytes_to_benchmark();

    // Create the benchmark group
    let mut group = create_benchmark_group(criterion, "compression");

    // Benchmark regular compression across the byte lengths
    let mut compression_ratios = BTreeMap::new();
    for (bytes, label) in random_bytes_to_benchmark {
        group.bench_with_input(
            BenchmarkId::new(label.clone(), bytes.len()),
            &bytes,
            |bencher, bytes| {
                bencher.iter(|| {
                    compress(&label, bytes, &mut compression_ratios);
                });
            },
        );
    }

    // Print the compression ratios
    print_compression_ratios(compression_ratios);
}

/// The benchmark for variable compression over random bytes
pub fn benchmark_compression_variable_random(criterion: &mut Criterion) {
    // Get the random bytes to benchmark
    let random_bytes_to_benchmark = get_random_bytes_to_benchmark();

    // Create the benchmark group
    let mut group = create_benchmark_group(criterion, "compression_variable");

    // Benchmark variable compression across the byte lengths
    let mut compression_ratios = BTreeMap::new();
    for (bytes, label) in random_bytes_to_benchmark {
        group.bench_with_input(
            BenchmarkId::new(label.clone(), bytes.len()),
            &bytes,
            |bencher, bytes| {
                bencher.iter(|| {
                    compress_with_variable_compression(&label, bytes, &mut compression_ratios);
                });
            },
        );
    }

    // Print the compression ratios
    print_compression_ratios(compression_ratios);
}

/// The benchmark for regular decompression over compressed bytes
pub fn benchmark_decompression_random(criterion: &mut Criterion) {
    // Get the random bytes to benchmark
    let random_bytes_to_benchmark = get_random_bytes_to_benchmark();

    // Create the benchmark group
    let mut group = create_benchmark_group(criterion, "decompression");

    // Benchmark regular decompression across the byte lengths
    let mut compression_ratios = BTreeMap::new();
    for (bytes, label) in random_bytes_to_benchmark {
        // Compress the bytes
        let compressed_bytes = aptos_compression::compress(
            bytes.clone(),
            CompressionClient::Consensus,
            MAX_COMPRESSION_SIZE,
        )
        .unwrap();

        // Benchmark the decompression
        group.bench_with_input(
            BenchmarkId::new(label.clone(), bytes.len()),
            &bytes,
            |bencher, _| {
                bencher.iter(|| {
                    decompress(&label, &compressed_bytes, &mut compression_ratios);
                });
            },
        );
    }

    // Print the compression ratios
    print_compression_ratios(compression_ratios);
}

/// The benchmark for variable decompression over compressed bytes
pub fn benchmark_decompression_variable_random(criterion: &mut Criterion) {
    // Get the random bytes to benchmark
    let random_bytes_to_benchmark = get_random_bytes_to_benchmark();

    // Create the benchmark group
    let mut group = create_benchmark_group(criterion, "decompression_variable");

    // Benchmark variable decompression across the byte lengths
    let mut compression_ratios = BTreeMap::new();
    for (bytes, label) in random_bytes_to_benchmark {
        // Compress the bytes
        let compressed_bytes = aptos_compression::compress_with_variable_compression(
            bytes.clone(),
            CompressionClient::Consensus,
            MAX_COMPRESSION_SIZE,
        )
        .unwrap();

        // Benchmark the decompression
        group.bench_with_input(
            BenchmarkId::new(label.clone(), bytes.len()),
            &bytes,
            |bencher, _| {
                bencher.iter(|| {
                    decompress(&label, &compressed_bytes, &mut compression_ratios);
                });
            },
        );
    }

    // Print the compression ratios
    print_compression_ratios(compression_ratios);
}

/// Calculates the compression ratio between the input and
/// output, i.e., (output / input) * 100.
fn calculate_compression_ratio(input: &[u8], output: &[u8]) -> f64 {
    (output.len() as f64 / input.len() as f64) * 100.0
}

/// Compresses the given bytes using regular compression and
/// inserts the ratio into the map (iff it doesn't already exist).
fn compress(label: &str, bytes: &Vec<u8>, compression_ratios: &mut BTreeMap<usize, (f64, String)>) {
    // Compress the bytes
    let compressed_bytes = aptos_compression::compress(
        bytes.clone(),
        CompressionClient::Consensus,
        MAX_COMPRESSION_SIZE,
    )
    .unwrap();

    // Add the compression ratio to the map
    let byte_length = bytes.len();
    compression_ratios.entry(byte_length).or_insert_with(|| {
        let compression_ratio = calculate_compression_ratio(bytes, &compressed_bytes);
        (compression_ratio, label.into())
    });
}

/// Compresses the given bytes using variable compression and
/// inserts the ratio into the map (iff it doesn't already exist).
fn compress_with_variable_compression(
    label: &str,
    bytes: &Vec<u8>,
    compression_ratios: &mut BTreeMap<usize, (f64, String)>,
) {
    // Compress the bytes
    let compressed_bytes = aptos_compression::compress_with_variable_compression(
        bytes.clone(),
        CompressionClient::StateSync,
        MAX_COMPRESSION_SIZE,
    )
    .unwrap();

    // Add the compression ratio to the map
    let byte_length = bytes.len();
    compression_ratios.entry(byte_length).or_insert_with(|| {
        let compression_ratio = calculate_compression_ratio(bytes, &compressed_bytes);
        (compression_ratio, label.into())
    });
}

/// Decompresses the given bytes and inserts the ratio
/// into the map (iff it doesn't already exist).
fn decompress(
    label: &str,
    compressed_bytes: &CompressedData,
    compression_ratios: &mut BTreeMap<usize, (f64, String)>,
) {
    // Decompress the bytes
    let raw_bytes = aptos_compression::decompress(
        compressed_bytes,
        CompressionClient::Consensus,
        MAX_COMPRESSION_SIZE,
    )
    .unwrap();

    // Add the compression ratio to the map
    let byte_length = raw_bytes.len();
    compression_ratios.entry(byte_length).or_insert_with(|| {
        let compression_ratio = calculate_compression_ratio(&raw_bytes, compressed_bytes);
        (compression_ratio, label.into())
    });
}

/// Creates a benchmark group with the given name
fn create_benchmark_group<'a>(
    criterion: &'a mut Criterion,
    group_name: &'a str,
) -> BenchmarkGroup<'a, WallTime> {
    // Create the group and limit the sample size
    let mut group = criterion.benchmark_group(group_name);
    group.sample_size(10);

    group
}

/// Returns a list of random byte vectors (alongside their
/// corresponding label) for benchmarking purposes.
fn get_random_bytes_to_benchmark() -> Vec<(Vec<u8>, String)> {
    let mut bytes_to_benchmark = Vec::new();

    // Create and collect the random byte strings
    for (length, label) in BYTE_LENGTHS_TO_TEST {
        let bytes: Vec<_> = (0..*length)
            .map(|_| rand::thread_rng().gen::<u8>())
            .collect();
        bytes_to_benchmark.push((bytes, label.to_string()));
    }

    bytes_to_benchmark
}

/// Prints the given compression ratios
fn print_compression_ratios(compression_ratios: BTreeMap<usize, (f64, String)>) {
    // If there are no compression ratios, return early
    if compression_ratios.is_empty() {
        return;
    }

    // Otherwise, print the compression ratios (sorted by byte length)
    println!("\nCompression Ratios:");
    for (_, (compression_ratio, label)) in compression_ratios {
        println!("Label: {}, Ratio: {}%", label, compression_ratio);
    }
}

criterion_group!(
    benches,
    benchmark_compression_random,
    benchmark_compression_variable_random,
    benchmark_decompression_random,
    benchmark_decompression_variable_random
);
criterion_main!(benches);
