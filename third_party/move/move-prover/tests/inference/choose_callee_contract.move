// flag: --verify-only=choose_callee_contract::caller
// A callee whose contract chooses its result: WP carries the `choose` into the
// caller's conditions, and the specification it writes must parse back.
module 0x42::choose_callee_contract {
    fun first_at_least(v: &vector<u64>, x: u64): u64 {
        let i = 0;
        let n = std::vector::length(v);
        while (i < n) {
            if (v[i] >= x) return v[i];
            i += 1;
        } spec {
            invariant i <= n;
            invariant forall k in 0..i: v[k] < x;
        };
        abort 1
    }
    spec first_at_least {
        pragma opaque;
        aborts_if !(exists j in 0..len(v): v[j] >= x);
        ensures result == v[choose min j in 0..len(v) where v[j] >= x];
    }

    fun caller(v: vector<u64>, x: u64, y: u64): bool {
        y < first_at_least(&v, x)
    }
}
