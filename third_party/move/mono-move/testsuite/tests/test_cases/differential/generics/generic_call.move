// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x42::generic_call {
    fun identity<T>(x: T): T {
        x
    }

    fun call_u64(): u64 {
        identity<u64>(7)
    }

    fun call_bool(): bool {
        identity<bool>(true)
    }
    fun innermost<T: drop>(value: T): T { value }

    fun middle<T: drop>(value: T): T { innermost<T>(value) }

    fun outer<T: drop>(value: T): T { middle<T>(value) }

    public fun nested_entry(value: u64): u64 {
        outer<u64>(value)
    }
    fun pick_first<A: drop, B: drop>(first: A, _second: B): A { first }

    public fun first_entry(seed: u64): u64 {
        pick_first<u64, u64>(seed, 99)
    }
    fun pick_second<A: drop, B: drop>(_first: A, second: B): B { second }

    public fun second_entry(seed: u64): u64 {
        pick_second<u64, u64>(99, seed)
    }
    fun partial_identity<A: drop, B: drop>(kept: A, _dropped: B): A { kept }

    fun caller<T: drop>(kept: u64, dropped: T): u64 {
        partial_identity<u64, T>(kept, dropped)
    }

    public fun partial_entry(seed: u64): u64 {
        caller<u64>(42, seed)
    }
    fun repeat_identity<T: drop>(value: T): T { value }

    public fun repeat_entry(): u64 {
        let first = repeat_identity<u64>(10);
        let second = repeat_identity<u64>(20);
        let third = repeat_identity<u64>(12);
        first + second + third
    }
}

// RUN: execute 0x42::generic_call::call_u64
// CHECK: results: 7

// RUN: execute 0x42::generic_call::call_bool
// CHECK: results: true

// RUN: execute 0x42::generic_call::nested_entry --args 42
// CHECK: results: 42

// RUN: execute 0x42::generic_call::first_entry --args 42
// CHECK: results: 42

// RUN: execute 0x42::generic_call::second_entry --args 42
// CHECK: results: 42

// RUN: execute 0x42::generic_call::partial_entry --args 7
// CHECK: results: 42

// RUN: execute 0x42::generic_call::repeat_entry
// CHECK: results: 42
