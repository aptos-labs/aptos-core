// A loop body deviating from the spec'd application chain (applying the fun
// value twice per element) cannot prove the chain invariant: each application
// advances the value to a fresh family member, so the induction step needs
// exactly the one application `apply_all` describes.
module 0x42::inline_opaque_hof_for_each_sum_fail {
    use std::vector;

    spec fun apply_all(f: |&u64| has copy + drop, v: vector<u64>, end: u64): |&u64| has copy + drop {
        if (end == 0) f else fun_post_of<apply_all(f, v, end - 1)>(v[end - 1])
    }

    inline fun for_each_ref_twice(v: &vector<u64>, f: |&u64| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= len(v);
            invariant f == apply_all(old(f), v, i); // error: induction case of the loop invariant does not hold
        };
    }
    spec for_each_ref_twice {
        pragma opaque;
        requires forall j in 0..len(v): !aborts_of<apply_all(f, v, j)>(v[j]);
        ensures f == apply_all(old(f), v, len(v));
    }

    fun caller(v: &vector<u64>): u64 {
        let s = 0;
        for_each_ref_twice(v, |e| s = s + *e spec {
            aborts_if s + e > MAX_U64;
            ensures s == old(s) + e;
        });
        s
    }
    spec caller {
        requires len(v) == 2;
        requires forall i in 0..len(v): v[i] < 1000;
    }
}
