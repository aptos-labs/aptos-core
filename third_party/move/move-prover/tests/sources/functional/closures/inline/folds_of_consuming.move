// Consuming `folds_of` with an expansion-entry snapshot.
module 0x42::folds_of_consuming {
    use std::vector;

    struct NoCopy has drop {
        value: u64,
    }

    spec fun spec_fold_idx<Acc>(
        f: |Acc, u64| Acc,
        init: Acc,
        end: u64,
    ): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold_idx(f, init, end - 1), end - 1)
    }

    inline fun each_reverse<T>(v: vector<T>, f: |T|) {
        let remaining = vector::length(&v);
        let n = remaining;
        while (remaining > 0) {
            f(vector::pop_back(&mut v));
            remaining = remaining - 1;
        } spec {
            invariant remaining <= n;
            invariant remaining == len(v);
            invariant n == len(old(v));
            invariant forall j in 0..remaining: v[j] == old(v)[j];
            invariant folds_of<f>(
                |j| old(v)[n - 1 - j],
                n - remaining
            );
        };
        vector::destroy_empty(v)
    }

    inline fun each<T>(v: vector<T>, f: |T|) {
        vector::reverse(&mut v);
        each_reverse(v, |x| f(x));
    }

    fun sum_literal(): u64 {
        let sum = 0;
        each_reverse(vector[1u64, 2u64, 3u64], |x| sum = sum + x);
        sum
    }
    spec sum_literal {
        aborts_if false;
        ensures result == 6;
    }

    fun digits_forward(): u64 {
        let digits = 0;
        each(vector[1u64, 2u64, 3u64], |x| digits = digits * 10 + x);
        digits
    }
    spec digits_forward {
        aborts_if false;
        ensures result == 123;
    }

    fun digits_reverse(): u64 {
        let digits = 0;
        each_reverse(
            vector[1u64, 2u64, 3u64],
            |x| digits = digits * 10 + x,
        );
        digits
    }
    spec digits_reverse {
        aborts_if false;
        ensures result == 321;
    }

    fun sum_noncopy(): u64 {
        let sum = 0;
        each_reverse(
            vector[NoCopy { value: 4 }, NoCopy { value: 5 }],
            |x| sum = sum + x.value,
        );
        sum
    }
    spec sum_noncopy {
        aborts_if false;
        ensures result == 9;
    }
}
