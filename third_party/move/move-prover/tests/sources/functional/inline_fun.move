module 0x42::Test {
    use std::vector;

    public inline fun filter<X: drop>(v: &mut vector<X>, predicate: |&X| bool) {
        let i = 0;
        while ({
            spec {
                // TODO: this will cause a no-such-function error as `predicate`
                // is inlined away in the implementation. We are aware of this
                // issue and is working on a fix by bridging more information
                // from inlined functions into the spec.
                invariant forall k in 0..i: !predicate(v[k]);
                // TODO: complete the set of loop invariants
            };
            (i < vector::length(v))
        }) {
            if (predicate(vector::borrow(v, i))) {
                vector::swap_remove(v, i);
            } else {
                i = i + 1;
            };
        }
    }

    public fun test_filter(): vector<u64> {
        let v = vector[1u64, 2, 3];
        filter(&mut v, |e| *e > 1);
        v
    }
    spec test_filter {
        pragma verify = true;
        // TODO: turn-on the verification once inlining on spec side is done
    }
}
