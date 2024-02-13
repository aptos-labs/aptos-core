// Copyright © Aptos Foundation

use criterion::Criterion;
use criterion::criterion_main;
use criterion::criterion_group;
use aptos_executor_types::{should_forward_to_subscription_service, should_forward_to_subscription_service_v2};
use aptos_types::contract_event::ContractEvent;

fn bench_group(c: &mut Criterion) {
    let mut group = c.benchmark_group("group1");
    group.bench_function("func1", move |b| {
        b.iter_with_setup(
            || ContractEvent::new_v2_with_type_tag_str("0x1::jwks::QuorumCertifiedUpdate", vec![0xff; 256]),
            |event| {
                should_forward_to_subscription_service(&event)
            },
        )
    });

    group.bench_function("func2", move |b| {
        b.iter_with_setup(
            || ContractEvent::new_v2_with_type_tag_str("0x1::jwks::QuorumCertifiedUpdate", vec![0xff; 256]),
            |event| {
                should_forward_to_subscription_service_v2(&event)
            },
        )
    });
}

criterion_group!(
    name = group1;
    config = Criterion::default();
    targets = bench_group
);

criterion_main!(group1);
