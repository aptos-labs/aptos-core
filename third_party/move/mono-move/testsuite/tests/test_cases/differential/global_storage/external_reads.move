// Reads and writes of a resource committed by an earlier step: exists before and after
// move_from, borrow_global and borrow_global_mut held across collections, move_to over
// it, and move_from then move_to of a new value.

// RUN: publish
module 0x42::ext {
    use std::vector;

    struct Counter has key, drop { value: u64 }

    public fun publish(account: signer, value: u64) {
        move_to(&account, Counter { value });
    }

    public fun has(addr: address): bool {
        exists<Counter>(addr)
    }

    public fun read_across_gc(addr: address): u64 acquires Counter {
        let counter = borrow_global<Counter>(addr);
        0x0::test_utils::force_gc();
        let values = vector[1u64, 2];
        0x0::test_utils::force_gc();
        counter.value * 10 + vector::length(&values)
    }

    public fun bump_across_gc(addr: address, delta: u64): u64 acquires Counter {
        let counter = borrow_global_mut<Counter>(addr);
        0x0::test_utils::force_gc();
        counter.value = counter.value + delta;
        0x0::test_utils::force_gc();
        counter.value
    }

    public fun take_then_has(addr: address): bool acquires Counter {
        let Counter { value: _ } = move_from<Counter>(addr);
        exists<Counter>(addr)
    }

    public fun replace(account: signer, addr: address, value: u64): u64 acquires Counter {
        let Counter { value: old } = move_from<Counter>(addr);
        move_to(&account, Counter { value });
        old
    }

    public fun take(addr: address): u64 acquires Counter {
        let Counter { value } = move_from<Counter>(addr);
        value
    }
}

// RUN: execute 0x42::ext::publish --args 0x42, 7 --commit
// CHECK: writes: create 0x42::ext::Counter at 0x42 0x0700000000000000

// RUN: execute 0x42::ext::has --args 0x42
// CHECK: results: true

// RUN: execute 0x42::ext::read_across_gc --args 0x42
// CHECK: results: 72
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::ext::bump_across_gc --args 0x42, 5
// CHECK: results: 12
// CHECK-GC-COUNT: 2

// RUN: execute 0x42::ext::bump_across_gc --args 0x42, 5 --commit
// CHECK: results: 12 | writes: modify 0x42::ext::Counter at 0x42 0x0c00000000000000

// RUN: execute 0x42::ext::take_then_has --args 0x42
// CHECK: results: false

// RUN: execute 0x42::ext::publish --args 0x42, 1
// CHECK-V1-SUBSTR: RESOURCE_ALREADY_EXISTS
// CHECK-V2-SUBSTR: MoveTo: resource already exists
// CHECK-ERROR-PARITY

// RUN: execute 0x42::ext::replace --args 0x42, 0x42, 20 --commit
// CHECK: results: 12 | writes: modify 0x42::ext::Counter at 0x42 0x1400000000000000

// RUN: execute 0x42::ext::take --args 0x42 --commit
// CHECK: results: 20 | writes: delete 0x42::ext::Counter at 0x42

// RUN: execute 0x42::ext::has --args 0x42
// CHECK: results: false
