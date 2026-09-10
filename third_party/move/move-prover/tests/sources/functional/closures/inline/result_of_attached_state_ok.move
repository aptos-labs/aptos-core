// Accepted forms of `result_of<f>(..)` resolved from a lambda's attached
// functional `ensures result == E` (the rejected form — a state-dependent
// `E` in a plain value position — is in `result_of_attached_state.move`):
// a state-free `E` splices anywhere, and in a loop invariant a
// state-dependent `E` is admitted, its `old(..)` resolving to the
// invariant's own entry scope.
module 0x42::result_of_attached_state_ok {
    use std::vector;

    struct R has key { v: u64 }

    // ===== state-free attached value in a plain assert =====

    inline fun apply(f: |u64| u64, x: u64): u64 {
        let res = f(x);
        spec {
            assert res == result_of<f>(x);
        };
        res
    }

    fun attached_pure_result(x: u64): u64 {
        apply(|y| y / 2 spec { ensures result == y / 2; }, x)
    }
    spec attached_pure_result {
        aborts_if false;
        ensures result == x / 2;
    }

    // ===== attached state-dependent value in a loop invariant, where =====
    // ===== `old(..)` resolves to the invariant's own entry scope     =====

    inline fun map(v: &vector<u64>, f: |&u64| u64): vector<u64> {
        let result = vector::empty<u64>();
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            vector::push_back(&mut result, f(vector::borrow(v, i)));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant len(result) == i;
            invariant forall j in 0..i: result[j] == result_of<f>(v[j]);
        };
        result
    }

    fun map_add_global(v: &vector<u64>, a: address): vector<u64> {
        map(v, |e| *e + R[a].v spec { ensures result == e + old(R[a].v); })
    }
    spec map_add_global {
        pragma aborts_if_is_partial;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == v[i] + R[a].v;
    }

    // A bare (un-`old`-ed) read in the attached value is likewise admitted
    // in a loop invariant, resolving to the current state (the loop does
    // not change `R`, so the point facts are provable).
    fun map_add_global_bare(v: &vector<u64>, a: address): vector<u64> {
        map(v, |e| *e + R[a].v spec { ensures result == e + R[a].v; })
    }
    spec map_add_global_bare {
        pragma aborts_if_is_partial;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == v[i] + R[a].v;
    }
}
