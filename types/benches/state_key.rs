// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_types::{account_config::AccountResource, state_store::state_key::StateKeyRegistry};
use criterion::{criterion_group, criterion_main, Criterion};
use fxhash::FxHasher;
use move_core_types::{account_address::AccountAddress, move_resource::MoveStructType};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

fn hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing");
    let address = AccountAddress::from_hex_literal("0xA550C18").unwrap();
    let struct_tag = AccountResource::struct_tag();

    group.bench_function("hash_address_and_name", |b| {
        b.iter(|| StateKeyRegistry::hash_address_and_name(&address, struct_tag.name.as_bytes()))
    });

    group.bench_function("default_hasher_struct_tag", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::default();
            Hash::hash(&struct_tag, &mut hasher);
            hasher.finish()
        })
    });

    group.bench_function("fxhasher_struct_tag", |b| {
        b.iter(|| {
            let mut hasher = FxHasher::default();
            Hash::hash(&struct_tag, &mut hasher);
            hasher.finish()
        })
    });

    group.bench_function("ahasher_struct_tag", |b| {
        b.iter(|| {
            let mut hasher = ahash::AHasher::default();
            Hash::hash(&struct_tag, &mut hasher);
            hasher.finish()
        })
    });
}

criterion_group!(
    name = state_key;
    config = Criterion::default();
    targets = hashing
);

criterion_main!(state_key);
