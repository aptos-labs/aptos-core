// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_language_e2e_tests::account_universe::P2PTransferGen;
use velor_transaction_benchmarks::{
    measurement::wall_time_measurement, transactions::TransactionBencher,
};
use criterion::{criterion_group, criterion_main, measurement::Measurement, Criterion};
use proptest::prelude::*;

//
// Transaction benchmarks
//

fn peer_to_peer<M: Measurement + 'static>(c: &mut Criterion<M>) {
    c.bench_function("peer_to_peer", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench(b)
    });

    c.bench_function("peer_to_peer_parallel", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench_parallel(b)
    });
}

criterion_group!(
    name = txn_benches;
    config = wall_time_measurement().sample_size(10);
    targets = peer_to_peer
);

criterion_main!(txn_benches);
