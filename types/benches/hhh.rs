// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use aptos_types::jwks::jwk::{JWK, JWKMoveStruct};
use aptos_types::jwks::ProviderJWKs;
use aptos_types::jwks::unsupported::UnsupportedJWK;

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("hhh");
    for n in [72315] {
        group.bench_function(BenchmarkId::new("bcs", n), move |b| {
            b.iter_with_setup(
                || {
                    let jwk = JWKMoveStruct::from(JWK::Unsupported(UnsupportedJWK::new_for_testing("", "")));                    let x = ProviderJWKs {
                        issuer: vec![],
                        version: 0,
                        jwks: vec![jwk; n],
                    };
                    bcs::to_bytes(&x).unwrap()
                },
                |bytes| {
                    let _a = bcs::from_bytes::<ProviderJWKs>(&bytes.as_slice());
                },
            )
        });
    }

}

criterion_group!(benches, bench_group);
criterion_main!(benches);
