// Verification of element-wise inline higher-order functions similar to those
// in the vector module (map, find, for_each_ref, for_each_mut), applied with
// different kinds of lambdas, including lambdas which access values of the
// enclosing context or modify values through `&mut` parameters. Calls are
// expanded and lambdas beta-reduced; loop invariants in the function bodies
// constrain the function parameter via behavioral predicates, which are
// inlined per caller: they are replaced by the lambda's attached spec, or
// derived from its body. See `vector_hofs_fold.move` for the accumulating
// (fold) counterpart.
module 0x42::vector_hofs_for_each {
    use std::vector;

    // ===== map: invariants use `result_of` and `aborts_of` =====

    inline fun map<T, U>(v: &vector<T>, f: |&T| U): vector<U> {
        let result = vector::empty<U>();
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
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
        };
        result
    }

    /// Lambda with an attached spec.
    fun map_double(v: &vector<u64>): vector<u64> {
        map(v, |e| *e * 2 spec {
            aborts_if e * 2 > MAX_U64;
            ensures result == e * 2;
        })
    }
    spec map_double {
        requires forall i in 0..len(v): v[i] * 2 <= MAX_U64;
        aborts_if false;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == v[i] * 2;
    }

    /// Lambda reading `c` from the context.
    fun map_add(v: &vector<u64>, c: u64): vector<u64> {
        map(v, |e| *e + c spec {
            aborts_if e + c > MAX_U64;
            ensures result == e + c;
        })
    }
    spec map_add {
        requires forall i in 0..len(v): v[i] + c <= MAX_U64;
        aborts_if false;
        ensures len(result) == len(v);
        ensures forall i in 0..len(v): result[i] == v[i] + c;
    }

    /// A wrong lambda spec makes `map`'s invariant fail at this call site.
    fun map_double_wrong(v: &vector<u64>): vector<u64> {
        map(v, |e| *e * 2 spec {
            aborts_if e * 2 > MAX_U64;
            ensures result == e * 3; // error: map's loop invariant fails with this wrong lambda spec
        })
    }
    spec map_double_wrong {
        requires forall i in 0..len(v): v[i] * 2 <= MAX_U64;
    }



    // ===== find: invariant uses `result_of` over the predicate; the =====
    // ===== caller's spec-less lambda is derived by beta reduction    =====

    inline fun find<T>(v: &vector<T>, f: |&T| bool): (bool, u64) {
        let found = false;
        let index = 0;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            if (f(vector::borrow(v, i))) {
                found = true;
                index = i;
                break
            };
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant forall j in 0..i: !result_of<f>(v[j]);
        };
        (found, index)
    }

    /// Predicate lambda reading `x` from the context.
    fun find_value(v: &vector<u64>, x: u64): (bool, u64) {
        find(v, |e| *e == x)
    }
    spec find_value {
        ensures result_1 ==> result_2 < len(v) && v[result_2] == x;
        ensures !result_1 ==> (forall i in 0..len(v): v[i] != x);
    }

    // ===== for_each_ref: invariants use `ensures_of` and `aborts_of` =====

    inline fun for_each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(v[j]);
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
        };
    }

    /// The lambda validates each element against a context value; the caller
    /// derives its abort condition from `aborts_of` and the checked property
    /// from `ensures_of` of the lambda.
    fun check_all_bounded(v: &vector<u64>, cap: u64) {
        for_each_ref(v, |e| assert!(*e <= cap, 1) spec {
            aborts_if e > cap;
            ensures e <= cap;
        });
    }
    spec check_all_bounded {
        aborts_if exists i in 0..len(v): v[i] > cap;
        ensures forall i in 0..len(v): v[i] <= cap;
    }

    // ===== for_each_mut: invariants use dual-form `ensures_of` =====

    inline fun for_each_mut<T>(v: &mut vector<T>, f: |&mut T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant len(v) == len(old(v));
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
            invariant forall j in 0..i: !aborts_of<f>(old(v)[j]);
            invariant forall j in i..n: v[j] == old(v)[j];
        };
    }

    /// The lambda mutates the elements in place; its spec relates pre and
    /// post state of the `&mut u64` slot.
    fun increment_all(v: &mut vector<u64>) {
        for_each_mut(v, |e| {
            *e = *e + 1
        } spec {
            aborts_if e == MAX_U64;
            ensures e == old(e) + 1;
        });
    }
    spec increment_all {
        requires forall i in 0..len(v): v[i] < MAX_U64;
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v): v[i] == old(v)[i] + 1;
    }

    /// The lambda mutates the elements using a value from the context.
    fun scale_all(v: &mut vector<u64>, k: u64) {
        for_each_mut(v, |e| {
            *e = *e * k
        } spec {
            aborts_if e * k > MAX_U64;
            ensures e == old(e) * k;
        });
    }
    spec scale_all {
        requires forall i in 0..len(v): v[i] * k <= MAX_U64;
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v): v[i] == old(v)[i] * k;
    }

    /// The same as `increment_all`, with the lambda's spec derived from its
    /// body by the source-level WP analysis (dual-form `ensures_of` and the
    /// abort condition are inferred).
    fun increment_all_inferred(v: &mut vector<u64>) {
        for_each_mut(v, |e| *e = *e + 1);
    }
    spec increment_all_inferred {
        requires forall i in 0..len(v): v[i] < MAX_U64;
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v): v[i] == old(v)[i] + 1;
    }

    /// Spec-less imperative lambda with a let binding and control flow.
    fun clamp_all_inferred(v: &mut vector<u64>, cap: u64) {
        for_each_mut(v, |e| {
            let cur = *e;
            if (cur > cap) {
                *e = cap
            }
        });
    }
    spec clamp_all_inferred {
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v):
            v[i] == if (old(v)[i] > cap) cap else old(v)[i];
    }

    /// Control flow in the lambda; the spec relates pre and post via `if`.
    fun clamp_all(v: &mut vector<u64>, cap: u64) {
        for_each_mut(v, |e| {
            if (*e > cap) *e = cap
        } spec {
            aborts_if false;
            ensures e == if (old(e) > cap) cap else old(e);
        });
    }
    spec clamp_all {
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v):
            v[i] == if (old(v)[i] > cap) cap else old(v)[i];
    }

    // ===== lambdas reading global state: derived conditions stay     =====
    // ===== single-state and substitute into the loop invariants      =====

    struct Limit has key { max: u64 }

    /// The lambda reads global state directly.
    fun bump_all(v: &mut vector<u64>, a: address) {
        for_each_mut(v, |e| *e = *e + Limit[a].max);
    }
    spec bump_all {
        requires exists<Limit>(a);
        requires forall i in 0..len(v): v[i] + Limit[a].max <= MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(v): v[i] == old(v)[i] + Limit[a].max;
    }

    /// The lambda calls a function which reads global state; the callee's
    /// spec is used as a modular summary.
    fun limit_of(a: address): u64 {
        Limit[a].max
    }
    spec limit_of {
        pragma opaque;
        aborts_if !exists<Limit>(a);
        ensures result == Limit[a].max;
    }

    fun bump_all_via_call(v: &mut vector<u64>, a: address) {
        for_each_mut(v, |e| *e = *e + limit_of(a));
    }
    spec bump_all_via_call {
        requires exists<Limit>(a);
        requires forall i in 0..len(v): v[i] + Limit[a].max <= MAX_U64;
        aborts_if false;
        ensures forall i in 0..len(v): v[i] == old(v)[i] + Limit[a].max;
    }

    // ===== ensures_of/requires_of over a spec-less lambda are derived =====
    // ===== from the beta-reduced body                                  =====

    inline fun apply(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert requires_of<f>(x);
            assert ensures_of<f>(x, r);
        };
        r
    }

    fun apply_inc(x: u64): u64 {
        apply(|y| y + 1, x)
    }
    spec apply_inc {
        requires x < MAX_U64;
        ensures result == x + 1;
    }
}
