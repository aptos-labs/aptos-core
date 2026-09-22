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
        ensures [inferred] result == R[a].v;
        aborts_if [inferred] !exists<R>(a);
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
        ensures [inferred] result == R[a].v;
        aborts_if [inferred] !exists<R>(a);
    }

    // Consumes the complete boundary: inferred clauses are ordinary.
    fun twice(a: address): u64 acquires R {
        read(a) + read(a)
    }
    spec twice(a: address): u64 {
        pragma opaque = true;
        ensures [inferred] ({
            let a_1 = ..S1 |~ result_of<read>(a);
            let b = S1.. |~ result_of<read>(a);
            result == a_1 + b
        });
        aborts_if [inferred] S1 |~ (aborts_of<read>(a));
        aborts_if [inferred] aborts_of<read>(a);
        aborts_if [inferred] ({
            let a_1 = ..S1 |~ result_of<read>(a);
            let b = S1.. |~ result_of<read>(a);
            a_1 + b > MAX_U64
        });
    }


    // Consumes the partial boundary: the caller inherits `aborts_if_is_partial`.
    fun twice_partial(a: address): u64 acquires R {
        read_partial(a) + read_partial(a)
    }
    spec twice_partial(a: address): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        ensures [inferred] ({
            let a_1 = ..S1 |~ result_of<read_partial>(a);
            let b = S1.. |~ result_of<read_partial>(a);
            result == a_1 + b
        });
        aborts_if [inferred] ({
            let a_1 = ..S1 |~ result_of<read_partial>(a);
            let b = S1.. |~ result_of<read_partial>(a);
            a_1 + b > MAX_U64
        });
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `trusted_boundary_callee::twice_partial` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = callee `0x42::trusted_boundary_callee::read_partial` has no trusted complete abort summary
   ┌─ tests/inference/trusted_boundary_callee.move:40:5
   │
40 │ ╭     fun twice_partial(a: address): u64 acquires R {
41 │ │         read_partial(a) + read_partial(a)
42 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
