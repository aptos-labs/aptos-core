// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_executor_types::should_forward_to_subscription_service;
#[cfg(feature = "bench")]
use aptos_executor_types::should_forward_to_subscription_service_old;
use aptos_types::contract_event::ContractEvent;
use criterion::{Criterion, criterion_group, criterion_main};

fn default_targets(c: &mut Criterion) {
    let mut group = c.benchmark_group("should_forward_to_subscription_service");

    #[cfg(feature = "bench")]
    group.bench_function("v0", move |b| {
        b.iter_with_setup(
            || {
                ContractEvent::new_v2_with_type_tag_str(
                    "0x1::jwks::QuorumCertifiedUpdate",
                    vec![0xFF; 256],
                )
            },
            |event| should_forward_to_subscription_service_old(&event),
        )
    });

    group.bench_function("v1", move |b| {
        b.iter_with_setup(
            || {
                ContractEvent::new_v2_with_type_tag_str(
                    "0x1::jwks::QuorumCertifiedUpdate",
                    vec![0xFF; 256],
                )
            },
            |event| should_forward_to_subscription_service(&event),
        )
    });
}

criterion_group!(
    name = default_group;
    config = Criterion::default();
    targets = default_targets
);

criterion_main!(default_group);
