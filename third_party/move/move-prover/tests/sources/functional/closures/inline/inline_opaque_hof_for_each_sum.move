// The motivating use case: an accumulator lambda over a loop-invoking HOF.
// The HOF's spec describes the N applications as a `fun_post_of` chain over
// the advancing fun value (`apply_all`); the loop invariant
// `f == apply_all(old(f), v, i)` certifies the application discipline (one
// application per element, in order — see the _fail variant), and the caller
// unfolds the chain through the lambda's spec for concrete lengths.
module 0x42::opaque_inline_for_each_sum {
    use std::vector;

    spec fun apply_all(f: |&u64| has copy + drop, v: vector<u64>, end: u64): |&u64| has copy + drop {
        if (end == 0) f else fun_post_of<apply_all(f, v, end - 1)>(v[end - 1])
    }

    inline fun for_each_ref(v: &vector<u64>, f: |&u64| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= len(v);
            invariant f == apply_all(old(f), v, i);
        };
    }
    spec for_each_ref {
        pragma opaque;
        requires forall j in 0..len(v): !aborts_of<apply_all(f, v, j)>(v[j]);
        aborts_if false;
        ensures f == apply_all(old(f), v, len(v));
    }

    fun test_sum_two(v: &vector<u64>): u64 {
        let s = 0;
        for_each_ref(v, |e| s = s + *e spec {
            aborts_if s + e > MAX_U64;
            ensures s == old(s) + e;
        });
        s
    }
    spec test_sum_two {
        requires len(v) == 2;
        requires forall i in 0..len(v): v[i] < 1000;
        ensures result == v[0] + v[1];
    }
}
