// Spec function specialization inside a GENERIC caller: the expansion's
// material (the fold instantiation and the transformer) references the
// caller's type parameters, so the generated specialization is made
// parametric over exactly the mentioned parameters, and call sites pass
// them as type arguments. The backend then emits it per instantiation like
// any generic spec function — one copy for the generic verification target
// (`#0`) and one per concrete instantiation reached through callers — where
// a non-parametric declaration would hard-code `#0` and fail Boogie type
// checking at concrete call sites. Restatements in lemmas and proof blocks
// unify with the expansion when they mention the same type parameter
// indices. Covers both the `spec_fold` specialization and the bespoke
// multi-capture recursion.
module 0x42::specialize_generic_caller {
    use std::vector;

    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    inline fun each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(v, i);
        };
    }

    // Generic caller: the count transformer is specialized over the
    // CALLER's `T`.
    fun count_all<T>(v: &vector<T>): u64 {
        let count = 0;
        each_ref(v, |_e| count = count + 1);
        count
    }
    spec count_all {
        requires len(v) < 1000;
        ensures result == len(v);
    } proof {
        forall n: u64 {spec_fold<T, u64>(|acc, _e| acc + 1u64, v, 0u64, n)}
            apply count_is_len(v, n);
    }

    spec lemma count_is_len<T>(v: vector<T>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<T, u64>(|acc, _e| acc + 1u64, v, 0u64, n) == n;
    } proof {
        if (n > 0) {
            apply count_is_len(v, n - 1);
        }
    }

    // Concrete caller of the generic function: its verification reaches the
    // specialization at `T = u64`, exercising the concrete emission.
    fun use_concrete(v: &vector<u64>): u64 {
        count_all(v)
    }
    spec use_concrete {
        requires len(v) < 1000;
        ensures result == len(v);
    }

    // Multi-capture variant: the bespoke generated recursion
    // (`spec_fold$gen`) over the caller's `T`.
    fun count_two<T>(v: &vector<T>): (u64, u64) {
        let c1 = 0;
        let c2 = 0;
        each_ref(v, |_e| { c1 = c1 + 1; c2 = c2 + 2; });
        (c1, c2)
    }
    spec count_two {
        requires len(v) < 1000;
        pragma aborts_if_is_partial;
    }

    // Concrete caller reaching the multi-capture recursion at `T = u64`.
    fun use_concrete_two(v: &vector<u64>): (u64, u64) {
        count_two(v)
    }
    spec use_concrete_two {
        requires len(v) < 1000;
        pragma aborts_if_is_partial;
    }
}
