// Source old(v) denotes function entry even when an invariant follows a swap.
// Backward substitution must not apply the initial swap twice.
// inference-reject-mutation: // MUTATION_POINT => *v = vector[0, 0];
module 0x42::entry_state_after_write {
    use std::vector;

    spec module {
        fun swapped(v: vector<u64>): vector<u64> {
            update(update(v, 0, v[1]), 1, v[0])
        }
    }

    fun swap_then_loop(v: &mut vector<u64>, n: u64) {
        assert!(vector::length(v) == 2, 0);
        vector::swap(v, 0, 1);
        let i = 0;
        while (i < n) {
            vector::swap(v, 0, 1);
            i += 1;
        } spec {
            invariant i <= n;
            invariant len(v) == 2;
            invariant v == (if (i % 2 == 0) { swapped(old(v)) } else { old(v) });
        };
        // MUTATION_POINT
    }
    spec swap_then_loop(v: &mut vector<u64>, n: u64) {
        pragma opaque = true;
        ensures [inferred] len(old(v)) == 2 && (len(v) == 2 && len(if (n % 2 == 0) update(update(old(v), 0, old(v)[1]), 1, old(v)[0]) else old(v)) == 2) ==> v == (if (n % 2 == 0) update(update(old(v), 0, old(v)[1]), 1, old(v)[0]) else old(v));
        ensures [inferred] len(old(v)) != 2 ==> v == old(v);
        aborts_if [inferred] len(v) != 2;
    }

}
/*
Verification: Succeeded.
Mutation: Rejected by postcondition.
*/
