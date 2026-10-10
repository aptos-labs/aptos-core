// Realistic vector code: build a vector, read it back, and sum it in a loop.
// Every `vector` native here is lowered to the LIR operation that means the
// same thing, so the Lean side needs no stdlib module in its unit, while
// MonoVM runs the natives it registers.

// RUN: publish
module 0x4b::stdlib_vector_push {
    use std::vector;

    public fun built(n: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, n);
        vector::push_back(&mut v, n + 1);
        vector::push_back(&mut v, n + 2);
        vector::length(&v)
    }

    public fun total(n: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, n);
        vector::push_back(&mut v, n + 1);
        vector::push_back(&mut v, n + 2);
        let sum = 0;
        let i = 0;
        while (i < vector::length(&v)) {
            sum = sum + *vector::borrow(&v, i);
            i = i + 1;
        };
        sum
    }

    public fun swapped_first(n: u64): u64 {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, n);
        vector::push_back(&mut v, n + 1);
        vector::swap(&mut v, 0, 1);
        *vector::borrow(&v, 0)
    }

    public fun element(n: u64, index: u64): u64 {
        let v = vector[n, n + 1, n + 2];
        v[index]
    }
}

// RUN: execute 0x4b::stdlib_vector_push::built --args 5
// RUN: execute 0x4b::stdlib_vector_push::total --args 5
// RUN: execute 0x4b::stdlib_vector_push::element --args 5, 2
// RUN: execute 0x4b::stdlib_vector_push::swapped_first --args 5
