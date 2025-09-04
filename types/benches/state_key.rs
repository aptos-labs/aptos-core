// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::non_canonical_partial_ord_impl)]

use velor_crypto::HashValue;
use velor_proptest_helpers::ValueGenerator;
use velor_types::{
    access_path::AccessPath,
    account_config::AccountResource,
    state_store::state_key::{inner::StateKeyInner, registry::StateKeyRegistry, StateKey},
};
use criterion::{criterion_group, criterion_main, Criterion};
use derivative::Derivative;
use fxhash::FxHasher;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, move_resource::MoveStructType,
};
use once_cell::sync::OnceCell;
use proptest::prelude::*;
use std::{
    collections::{hash_map::DefaultHasher, HashSet},
    hash::{Hash, Hasher},
};

#[derive(Clone, Derivative, Eq)]
#[derivative(Hash, PartialEq, PartialOrd, Ord)]
pub struct UnCached {
    inner: StateKeyInner,
    #[derivative(
        Hash = "ignore",
        Ord = "ignore",
        PartialEq = "ignore",
        PartialOrd = "ignore"
    )]
    hash: OnceCell<HashValue>,
}

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

fn construct(c: &mut Criterion) {
    let keys = ValueGenerator::new().generate(proptest::collection::hash_set(
        (any::<AccountAddress>(), any::<StructTag>()),
        1_000,
    ));

    let mut group = c.benchmark_group("construct");
    group.throughput(criterion::Throughput::Elements(keys.len() as u64));

    group.bench_function("construct_once_uncached", |b| {
        b.iter(|| {
            keys.iter()
                .map(|(address, struct_tag)| UnCached {
                    inner: StateKeyInner::AccessPath(
                        AccessPath::resource_access_path(*address, struct_tag.clone()).unwrap(),
                    ),
                    hash: OnceCell::new(),
                })
                .collect::<Vec<_>>()
        })
    });

    group.bench_function("construct_once_cached", |b| {
        b.iter(|| {
            keys.iter()
                .map(|(address, struct_tag)| StateKey::resource(address, struct_tag).unwrap())
                .collect::<Vec<_>>()
        })
    });

    group.bench_function("construct_twice_uncached", |b| {
        b.iter(|| {
            keys.iter()
                .chain(keys.iter())
                .map(|(address, struct_tag)| UnCached {
                    inner: StateKeyInner::AccessPath(
                        AccessPath::resource_access_path(*address, struct_tag.clone()).unwrap(),
                    ),
                    hash: OnceCell::new(),
                })
                .collect::<Vec<_>>()
        })
    });

    group.bench_function("construct_twice_cached", |b| {
        b.iter(|| {
            keys.iter()
                .chain(keys.iter())
                .map(|(address, struct_tag)| StateKey::resource(address, struct_tag).unwrap())
                .collect::<Vec<_>>()
        })
    });

    group.bench_function("construct_thrice_uncached", |b| {
        b.iter(|| {
            keys.iter()
                .chain(keys.iter())
                .chain(keys.iter())
                .map(|(address, struct_tag)| UnCached {
                    inner: StateKeyInner::AccessPath(
                        AccessPath::resource_access_path(*address, struct_tag.clone()).unwrap(),
                    ),
                    hash: OnceCell::new(),
                })
                .collect::<Vec<_>>()
        })
    });

    group.bench_function("construct_thrice_cached", |b| {
        b.iter(|| {
            keys.iter()
                .chain(keys.iter())
                .chain(keys.iter())
                .map(|(address, struct_tag)| StateKey::resource(address, struct_tag).unwrap())
                .collect::<Vec<_>>()
        })
    });
}

fn hashset(c: &mut Criterion) {
    let keys = ValueGenerator::new().generate(proptest::collection::hash_set(
        any::<StateKeyInner>(),
        1_000,
    ));

    let mut group = c.benchmark_group("hashset");
    group.throughput(criterion::Throughput::Elements(keys.len() as u64));

    let uncached = keys
        .iter()
        .map(|inner| UnCached {
            inner: inner.clone(),
            hash: OnceCell::new(),
        })
        .collect::<hashbrown::HashSet<_>>();
    let cached = keys
        .iter()
        .map(|inner| StateKey::from_deserialized(inner.clone()).unwrap())
        .collect::<hashbrown::HashSet<_>>();

    group.bench_function("hashset_uncached", |b| {
        b.iter(|| uncached.iter().cloned().collect::<HashSet<_>>())
    });

    group.bench_function("hashset_cached", |b| {
        b.iter(|| cached.iter().cloned().collect::<HashSet<_>>())
    });

    group.bench_function("hashbrown_uncached", |b| {
        b.iter(|| uncached.iter().cloned().collect::<hashbrown::HashSet<_>>())
    });

    group.bench_function("hashbrown_cached", |b| {
        b.iter(|| cached.iter().cloned().collect::<hashbrown::HashSet<_>>())
    });
}

criterion_group!(
    name = state_key;
    config = Criterion::default();
    targets = hashing, construct, hashset
);

criterion_main!(state_key);
