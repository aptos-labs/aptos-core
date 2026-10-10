// Calls into the Move stdlib's `vector` module. Those functions are `native`
// with no body: the two engines reach their semantics by different routes.
// MonoVM registers them as natives, while the Leaner frontend lowers them to
// the LIR vector operations that mean the same thing, so the Lean side needs
// no stdlib module in its unit at all.

// RUN: publish
module 0x4a::stdlib_vector {
    use std::vector;

    public fun size(): u64 {
        let v = vector[10u64, 20, 30];
        vector::length(&v)
    }

    public fun size_of_empty(): u64 {
        let v = vector::empty<u64>();
        vector::length(&v)
    }

    public fun size_after_literal(n: u64): u64 {
        let v = vector[n, n, n, n];
        vector::length(&v)
    }
}

// RUN: execute 0x4a::stdlib_vector::size
// RUN: execute 0x4a::stdlib_vector::size_of_empty
// RUN: execute 0x4a::stdlib_vector::size_after_literal --args 7
