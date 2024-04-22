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

struct AddressAndNameHashHelper<'a> {
    address: &'a AccountAddress,
    name: &'a [u8],
}

impl<'a> Hash for AddressAndNameHashHelper<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(self.address.as_ref()[AccountAddress::LENGTH - 1]);
        if !self.name.is_empty() {
            state.write_u8(self.name[0]);
            state.write_u8(self.name[self.name.len() - 1]);
        }
    }
}

fn hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("hashing");
    let address = AccountAddress::from_hex_literal("0xA550C18").unwrap();
    let struct_tag = AccountResource::struct_tag();

    group.bench_function("hash_address_and_name", |b| {
        b.iter(|| StateKeyRegistry::hash_address_and_name(&address, struct_tag.name.as_bytes()))
    });

    group.bench_function("fxhasher_address_and_name_helper", |b| {
        b.iter(|| {
            let mut hasher = FxHasher::default();
            Hash::hash(
                &AddressAndNameHashHelper {
                    address: &address,
                    name: struct_tag.name.as_bytes(),
                },
                &mut hasher,
            );
            hasher.finish()
        })
    });

    group.bench_function("ahasher_address_and_name_helper", |b| {
        b.iter(|| {
            let mut hasher = ahash::AHasher::default();
            Hash::hash(
                &AddressAndNameHashHelper {
                    address: &address,
                    name: struct_tag.name.as_bytes(),
                },
                &mut hasher,
            );
            hasher.finish()
        })
    });

    group.bench_function("default_hasher_address", |b| {
        b.iter(|| {
            let mut hasher = DefaultHasher::default();
            Hash::hash(&address, &mut hasher);
            hasher.finish()
        })
    });

    group.bench_function("fxhasher_address", |b| {
        b.iter(|| {
            let mut hasher = FxHasher::default();
            Hash::hash(&address, &mut hasher);
            hasher.finish()
        })
    });

    group.bench_function("ahasher_address", |b| {
        b.iter(|| {
            let mut hasher = ahash::AHasher::default();
            Hash::hash(&address, &mut hasher);
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
