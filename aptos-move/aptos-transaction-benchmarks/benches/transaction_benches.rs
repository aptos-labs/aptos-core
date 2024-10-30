// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_language_e2e_tests::account_universe::P2PTransferGen;
use aptos_transaction_benchmarks::{
    measurement::wall_time_measurement, transactions::TransactionBencher,
};
use criterion::{criterion_group, criterion_main, measurement::Measurement, Criterion};
use proptest::prelude::*;

//
// Transaction benchmarks
//

fn peer_to_peer<M: Measurement + 'static>(c: &mut Criterion<M>) {
    c.bench_function("peer_to_peer seq number", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench(b, false, false)
    });

    c.bench_function("peer_to_peer seq number payload v2 format", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench(b, true, false)
    });

    c.bench_function("peer_to_peer orderless", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench(b, true, true)
    });

    c.bench_function("peer_to_peer_parallel seq number", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench_parallel(b, false, false)
    });

    c.bench_function("peer_to_peer_parallel seq number payload v2 format", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench_parallel(b, true, false)
    });

    c.bench_function("peer_to_peer_parallel orderless", |b| {
        let bencher = TransactionBencher::new(any_with::<P2PTransferGen>((1_000, 1_000_000)));
        bencher.bench_parallel(b, true, true)
    });
}

criterion_group!(
    name = txn_benches;
    config = wall_time_measurement().sample_size(10);
    targets = peer_to_peer
);

criterion_main!(txn_benches);
