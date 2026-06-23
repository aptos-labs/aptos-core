// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::{env, fs, path::PathBuf};

#[allow(dead_code, unused_imports)]
#[path = "../src/aws_nitro_utils.rs"]
mod aws_nitro_utils;

fn fixture_path(env_key: &str, default_path: &str) -> PathBuf {
    env::var_os(env_key)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_path))
}

fn bench_validate_and_parse(c: &mut Criterion) {
    let attestation_path = fixture_path("NITRO_ATTESTATION_DOC", "/tmp/attestation-3586.bin");
    let root_path = fixture_path("NITRO_ROOT_DER", "/tmp/aws-nitro-root.der");
    let attestation = fs::read(&attestation_path).unwrap_or_else(|err| {
        panic!(
            "failed to read Nitro attestation fixture {}: {err}",
            attestation_path.display()
        )
    });
    let root = fs::read(&root_path).unwrap_or_else(|err| {
        panic!(
            "failed to read Nitro root fixture {}: {err}",
            root_path.display()
        )
    });
    let (_, decoded_doc) =
        attestation_doc_validation::attestation_doc::decode_attestation_document(&attestation)
            .expect("Nitro attestation fixture should decode");
    let unix_time_secs = decoded_doc.timestamp / 1000;
    let roots = vec![root];
    let total_bytes = attestation.len() + roots.iter().map(Vec::len).sum::<usize>();

    aws_nitro_utils::validate_and_parse_attestation_doc_with_roots(
        &attestation,
        &roots,
        unix_time_secs,
    )
    .expect("Nitro attestation fixture should validate");

    let mut group = c.benchmark_group("aws_nitro_utils");
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_with_input(
        BenchmarkId::new("validate_and_parse_attestation_doc_with_roots", total_bytes),
        &total_bytes,
        |b, _| {
            b.iter(|| {
                aws_nitro_utils::validate_and_parse_attestation_doc_with_roots(
                    black_box(&attestation),
                    black_box(&roots),
                    black_box(unix_time_secs),
                )
                .expect("Nitro attestation fixture should validate")
            })
        },
    );
    group.finish();
}

criterion_group!(benches, bench_validate_and_parse);
criterion_main!(benches);
