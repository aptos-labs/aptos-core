// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A shared borrow of a stored enum's field, through the resource's index.
module 0x42::enum_storage_field_borrow {
    struct Inner has store, drop { value: u64 }

    enum Single has key { V1 { amount: u64 } }
    enum Both has key { V1 { amount: u64 }, V2 { amount: u64, extra: u64 } }
    enum Partial has key { V1 { amount: u64 }, V2 { extra: u64 } }
    enum Nested has key { V1 { inner: Inner } }

    fun read(value: &u64): u64 { *value }
    spec read {
        aborts_if false;
        ensures result == value;
    }

    fun single(addr: address): u64 {
        read(&Single[addr].amount)
    }
    spec single {
        aborts_if !exists<Single>(addr);
        ensures result == global<Single>(addr).amount;
    }

    fun both(addr: address): u64 {
        let amount = &Both[addr].amount;
        *amount
    }
    spec both {
        aborts_if !exists<Both>(addr);
        ensures result == global<Both>(addr).amount;
    }

    fun partial(addr: address): u64 {
        read(&Partial[addr].amount)
    }
    spec partial {
        aborts_if !exists<Partial>(addr);
        aborts_if global<Partial>(addr) is V2;
        ensures result == global<Partial>(addr).amount;
    }

    fun nested(addr: address): u64 {
        let value = &Nested[addr].inner.value;
        *value
    }
    spec nested {
        aborts_if !exists<Nested>(addr);
        ensures result == global<Nested>(addr).inner.value;
    }
}
