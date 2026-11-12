// Writes that leave a resource's bytes unchanged. V1 reports a modification
// for every resource it marks dirty (written through a mutable reference, or
// moved out and back), even with the bytes it already had; MonoMove drops a
// write whose value equals the value it read.
// Found by masm-smith fuzzing.

// RUN: publish
module 0x42::unchanged {
    struct Counter has key, copy, drop { value: u64 }

    public fun publish(s: signer, value: u64) {
        move_to(&s, Counter { value });
    }

    public fun set(a: address, value: u64) acquires Counter {
        *borrow_global_mut<Counter>(a) = Counter { value };
    }

    public fun borrow_only(a: address): u64 acquires Counter {
        borrow_global_mut<Counter>(a).value
    }

    public fun take_and_put_back(s: signer, a: address) acquires Counter {
        let counter = move_from<Counter>(a);
        move_to(&s, counter);
    }

    public fun take(a: address): u64 acquires Counter {
        let Counter { value } = move_from<Counter>(a);
        value
    }
}

// RUN: execute 0x42::unchanged::publish --args 0x42, 7 --commit
// CHECK: writes: create 0x42::unchanged::Counter at 0x42 0x0700000000000000

// Expected divergence: the same bytes written back.
// RUN: execute 0x42::unchanged::set --args 0x42, 7 --commit
// CHECK-V1: writes: modify 0x42::unchanged::Counter at 0x42 0x0700000000000000
// CHECK-V2: writes: none

// A mutable borrow with no write through it is not a write on either.
// RUN: execute 0x42::unchanged::borrow_only --args 0x42 --commit
// CHECK: results: 7 | writes: none

// Expected divergence: moved out and back unchanged.
// RUN: execute 0x42::unchanged::take_and_put_back --args 0x42, 0x42 --commit
// CHECK-V1: writes: modify 0x42::unchanged::Counter at 0x42 0x0700000000000000
// CHECK-V2: writes: none

// A changed value is a write on both.
// RUN: execute 0x42::unchanged::set --args 0x42, 8 --commit
// CHECK: writes: modify 0x42::unchanged::Counter at 0x42 0x0800000000000000

// RUN: execute 0x42::unchanged::take --args 0x42 --commit
// CHECK: results: 8 | writes: delete 0x42::unchanged::Counter at 0x42
