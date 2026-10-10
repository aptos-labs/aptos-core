// Struct type arguments are invariant in spec expressions as well: the prover models each
// instantiation as a distinct type, so widening an integer inside a type argument (`S<u8>` to
// `S<u64>`) would produce an ill-typed verification problem.
module 0x42::m {
    struct S<T> has drop { v: T }

    spec fun mk<T>(x: T): S<T> { S { v: x } }

    spec fun ret_param(y: u8): S<u64> { mk(y) }

    spec fun join_branches(c: bool, y: u64): S<u64> { if (c) mk(0) else mk(y) }

    spec fun nested(y: u8): S<S<u64>> { mk(mk(y)) }
}
