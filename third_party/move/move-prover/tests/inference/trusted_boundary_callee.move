// A `verify = false` callee is a stated boundary: the caller consumes its
// contract like any opaque contract, and whether the body was proved is a
// separate obligation, so its clauses are ordinary. An incomplete contract --
// a partial abort characterization -- is tracked where it matters: the
// caller's specification carries `aborts_if_is_partial`; its clauses are not
// solver-hard for that reason.
module 0x42::trusted_boundary_callee {
    struct R has key {
        v: u64,
    }

    // Complete contract, body deliberately left unverified.
    fun read(a: address): u64 acquires R {
        R[a].v
    }
    spec read {
        pragma opaque;
        pragma verify = false;
        aborts_if !exists<R>(a);
        ensures result == R[a].v;
    }

    // Partial contract: the abort behavior is a lower bound.
    fun read_partial(a: address): u64 acquires R {
        R[a].v
    }
    spec read_partial {
        pragma opaque;
        pragma verify = false;
        pragma aborts_if_is_partial;
        ensures result == R[a].v;
    }

    // Consumes the complete boundary: inferred clauses are ordinary.
    fun twice(a: address): u64 acquires R {
        read(a) + read(a)
    }

    // Consumes the partial boundary: the caller inherits `aborts_if_is_partial`.
    fun twice_partial(a: address): u64 acquires R {
        read_partial(a) + read_partial(a)
    }
}
