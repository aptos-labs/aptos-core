// Copies of a closure that captures a vector. The callee mutates the vector it receives by
// value, so every later call, through another copy or the original, must still see the
// vector as captured.

// RUN: publish
module 0x42::closure_copy_mutate {
    use std::vector;

    // Adds `delta` to element 0, appends `delta`, and reports the first two elements and
    // the length.
    fun bump(vals: vector<u64>, delta: u64): u64 {
        *vector::borrow_mut(&mut vals, 0) = *vector::borrow(&vals, 0) + delta;
        vector::push_back(&mut vals, delta);
        *vector::borrow(&vals, 0) * 10000 + *vector::borrow(&vals, 1) * 100 + vector::length(&vals)
    }

    // The first call runs on a copy of `bumper`, the second on `bumper` itself.
    public fun call_twice(seed: u64): u64 {
        let vals = vector[seed, seed + 1];
        let bumper: |u64|u64 has copy + drop = |delta| bump(vals, delta);
        let first = bumper(1);
        let second = bumper(2);
        first * 1000000 + second
    }

    // `dup` is an explicit copy of `bumper`; its first call runs on a further copy.
    public fun copy_then_call(seed: u64): u64 {
        let vals = vector[seed, seed + 1];
        let bumper: |u64|u64 has copy + drop = |delta| bump(vals, delta);
        let dup = copy bumper;
        let first = dup(1);
        let second = bumper(2);
        let third = dup(3);
        (first * 1000000 + second) * 1000000 + third
    }
}

// RUN: execute 0x42::closure_copy_mutate::call_twice --args 10
// CHECK: results: 111103121103

// RUN: execute 0x42::closure_copy_mutate::copy_then_call --args 10
// CHECK: results: 111103121103131103
