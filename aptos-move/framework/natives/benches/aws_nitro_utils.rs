// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::{env, fs, path::PathBuf};
use webpki::{EndEntityCert, TrustAnchor};

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
    let root_cert = roots[0].as_slice();
    let total_bytes = attestation.len() + roots.iter().map(Vec::len).sum::<usize>();
    let intermediate_certs = decoded_doc
        .cabundle
        .iter()
        .map(|cert| cert.as_slice())
        .collect::<Vec<_>>();
    let signing_cert = attestation_doc_validation::parse_cert(&decoded_doc.certificate)
        .expect("Nitro attestation signing certificate should parse");
    let pub_key = aws_nitro_utils::NitroPublicKey::new(signing_cert.public_key())
        .expect("Nitro attestation signing key should parse");
    let end_entity_cert =
        EndEntityCert::try_from(decoded_doc.certificate.as_slice()).expect("leaf cert parses");
    let trust_anchor = TrustAnchor::try_from_cert_der(root_cert).expect("root cert parses");
    let trust_anchors = vec![trust_anchor];
    let server_trust_anchors = webpki::TlsServerTrustAnchors(&trust_anchors);
    let time = webpki::Time::from_seconds_since_unix_epoch(unix_time_secs);
    let (cose_sign_1_decoded, _) =
        attestation_doc_validation::attestation_doc::decode_attestation_document(&attestation)
            .expect("Nitro attestation fixture should decode");

    aws_nitro_utils::validate_and_parse_attestation_doc_with_roots(
        &attestation,
        &roots,
        unix_time_secs,
    )
    .expect("Nitro attestation fixture should validate");

    let mut group = c.benchmark_group("aws_nitro_utils");
    group.throughput(Throughput::Bytes(total_bytes as u64));
    group.bench_function("decode_attestation_document", |b| {
        b.iter(|| {
            attestation_doc_validation::attestation_doc::decode_attestation_document(black_box(
                &attestation,
            ))
            .expect("Nitro attestation fixture should decode")
        })
    });
    group.bench_function("validate_cert_trust_chain_with_roots", |b| {
        b.iter(|| {
            aws_nitro_utils::validate_cert_trust_chain_with_roots(
                black_box(&decoded_doc.certificate),
                black_box(&intermediate_certs),
                black_box(&roots),
                black_box(unix_time_secs),
            )
            .expect("Nitro attestation certificate chain should validate")
        })
    });
    group.bench_function("webpki_parse_leaf_cert", |b| {
        b.iter(|| {
            EndEntityCert::try_from(black_box(decoded_doc.certificate.as_slice()))
                .expect("leaf cert parses")
        })
    });
    group.bench_function("webpki_parse_root_trust_anchor", |b| {
        b.iter(|| TrustAnchor::try_from_cert_der(black_box(root_cert)).expect("root cert parses"))
    });
    group.bench_function("webpki_verify_preparsed_chain", |b| {
        b.iter(|| {
            end_entity_cert
                .verify_is_valid_tls_server_cert(
                    black_box(aws_nitro_utils::SUPPORTED_SIG_ALGS),
                    black_box(&server_trust_anchors),
                    black_box(&intermediate_certs),
                    black_box(time),
                )
                .expect("pre-parsed Nitro certificate chain should validate")
        })
    });
    group.bench_function("parse_attestation_signing_cert", |b| {
        b.iter(|| {
            attestation_doc_validation::parse_cert(black_box(&decoded_doc.certificate))
                .expect("Nitro attestation signing certificate should parse")
        })
    });
    group.bench_function("nitro_public_key_from_spki", |b| {
        b.iter(|| {
            aws_nitro_utils::NitroPublicKey::new(black_box(signing_cert.public_key()))
                .expect("Nitro attestation signing key should parse")
        })
    });
    group.bench_function("validate_cose_signature", |b| {
        b.iter(|| {
            aws_nitro_utils::validate_cose_signature(
                black_box(&pub_key),
                black_box(&cose_sign_1_decoded),
            )
            .expect("Nitro attestation COSE signature should validate")
        })
    });
    group.bench_with_input(
        BenchmarkId::new(
            "full_validate_and_parse_attestation_doc_with_roots",
            total_bytes,
        ),
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
