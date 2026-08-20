// Error cases for behavioral predicates over lambda arguments of inline
// functions.
module 0x42::bp_inline_errors {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert !aborts_of<f>(x);
        };
        r
    }

    /// A spec cannot be derived from a lambda body containing a loop (no
    /// invariants are available at derivation time).
    fun aborts_of_underivable(x: u64): u64 {
        apply(|y| {
            let acc = y;
            while (acc > 10) {
                acc = acc - 10;
            };
            acc
        }, x) // error: cannot resolve `aborts_of`, lambda needs a spec block
    }

    inline fun modify(f: |&mut u64|, r: &mut u64) {
        f(r);
        spec {
            // The minimum form is not enough for a `&mut` parameter here;
            // the canonical form with an explicit post-state argument is
            // required.
            assert ensures_of<f>(r); // error: requires the canonical form with post-state arguments
        };
    }

    fun ensures_of_live_form(x: u64): u64 {
        let v = x;
        modify(|e| *e = *e + 1 spec { ensures e == old(e) + 1; }, &mut v);
        v
    }

    inline fun apply_result(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert r == result_of<f>(x);
        };
        r
    }

    /// `result_of` falls back to the body-derived value when the attached
    /// spec has no functional `ensures result == E` condition (see
    /// `result_of_attached_fallback.move`); if the body is underivable too,
    /// the error names both missing sources.
    fun result_of_no_functional_ensures(x: u64): u64 {
        apply_result(|y| {
            let acc = y;
            while (acc > 10) {
                acc = acc - 10;
            };
            acc
        } spec { ensures result <= 10; }, x) // error: the attached spec has no `ensures result == E` condition and no result value can be derived from the body
    }

    inline fun apply_state_twice(f: |address|, a: address) {
        f(a);
        f(a);
        spec {
            // Abort conditions are phrased over the application's pre-state;
            // without a unique application site there is no anchor to
            // resolve their global state reads against (evaluating them at
            // the assertion-site state would let a false claim verify).
            assert !aborts_of<f>(a); // error: requires a unique application to anchor the predicate's states
        };
    }

    fun aborts_of_state_multi_apply(a: address) {
        apply_state_twice(|x| {
            let r = &mut R[x];
            r.v = r.v + 1;
        }, a);
    }
    spec aborts_of_state_multi_apply {
        requires exists<R>(a) && R[a].v < MAX_U64 - 1;
    }

    fun with_requires(x: u64): u64 { x - 1 }
    spec with_requires {
        requires x > 0;
        ensures result == x - 1;
    }

    inline fun apply_requires(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            // The body derivation does not describe preconditions, so
            // `requires_of` cannot honestly resolve to `true` for a lambda
            // with callee-precondition material.
            assert requires_of<f>(x); // error: the lambda's body calls a function with a `requires` condition
        };
        r
    }

    fun requires_of_callee_requires(x: u64): u64 {
        apply_requires(|y| with_requires(y), x)
    }
    spec requires_of_callee_requires {
        requires x > 0;
    }

    /// The `requires` of an applied function value is not known either.
    fun requires_of_fun_value(g: |u64| u64 has copy + drop, x: u64): u64 {
        apply_requires(|y| g(y), x) // error: the lambda's body applies a function value whose `requires` is not known
    }

    inline fun apply2(f: |u64| u64, x: u64): u64 {
        let r = f(x);
        spec {
            assert ensures_of<f>(x, r);
        };
        r
    }

    /// `old(..)` in a lambda spec substituted for a behavioral predicate may
    /// only be applied directly to a lambda parameter.
    fun old_of_non_parameter(x: u64, c: u64): u64 {
        apply2(|y| y + c spec {
            ensures result == y + old(c); // error: old(..) must be applied directly to a lambda parameter
        }, x)
    }

    inline fun each_u64_aborts(v: &vector<u64>, f: |u64|) {
        let i = 0;
        let n = std::vector::length(v);
        while (i < n) {
            f(*std::vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant forall j in 0..i: !aborts_of<f>(v[j]);
        };
    }

    /// A lambda assigning a variable of the enclosing scope: the capture's
    /// value at an application is an inductive quantity the pointwise
    /// `aborts_of` cannot name; `folds_of` carries the cumulative effect
    /// and its abort-freeness instead. (`ensures_of` over such a lambda
    /// resolves by dropping the capture-mentioning conjuncts; see
    /// `folds_of.move`.)
    fun aborts_of_captured_assign(v: &vector<u64>): u64 {
        let sum = 0;
        each_u64_aborts(v, |y| sum = sum + y); // error: aborts_of cannot constrain a capture-writing lambda, use folds_of
        sum
    }

    /// A capture-writing lambda with a result: same for `result_of`, whose
    /// value would have to mention the capture's pre-state.
    fun result_of_captured_assign(x: u64): u64 {
        let sum = 0;
        apply_result(|y| {
            sum = sum + y;
            sum
        }, x) // error: result_of cannot constrain a capture-writing lambda, use folds_of
    }

    // ===== residual boundaries for lambdas with global state effects =====
    // ===== (see state_hofs.move for the supported patterns)          =====

    struct R has key { v: u64 }
    struct T has key { w: u64 }

    fun bump(a: address) {
        let r = &mut R[a];
        r.v = r.v + 1;
    }
    spec bump {
        pragma opaque;
        modifies global<R>(a);
        aborts_if !exists<R>(a) || R[a].v == MAX_U64;
        ensures R[a].v == old(R[a].v) + 1;
    }

    inline fun each_unchanged(v: &vector<address>, f: |address|) {
        let i = 0;
        let n = std::vector::length(v);
        while (i < n) {
            f(*std::vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant forall j in i..n: unchanged_of<f>(v[j]);
        };
    }

    /// `unchanged_of` builds its frame from the write footprint derived from
    /// the lambda body; a callee summary does not provide one, so it weakens.
    fun unchanged_of_callee_footprint(v: &vector<address>) {
        each_unchanged(v, |a| bump(a)); // warning: unchanged_of weakened
    }

    inline fun each_ensures(v: &vector<address>, f: |address|) {
        let i = 0;
        let n = std::vector::length(v);
        while (i < n) {
            f(*std::vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant forall j in 0..i: ensures_of<f>(v[j]);
        };
    }

    /// A body containing a nested loop has no source-derived summary. The
    /// generic HOF invariant is weakened instead of rejecting the call.
    fun ensures_of_underivable_loop(v: &vector<address>) {
        each_ensures(v, |a| {
            let i = 0;
            while (i < 1) {
                i = i + 1;
            };
            let _ = a;
        }); // warning: underivable ensures_of loop invariant weakened
    }

    /// Two sequential global state effects: the intermediate memory state
    /// between them cannot be named in a loop invariant.
    fun two_effects_in_loop(v: &vector<address>) {
        each_ensures(v, |a| {
            let r = &mut R[a];
            r.v = r.v + 1;
            let t = &mut T[a];
            t.w = t.w + 1;
        }); // warning: intermediate-state ensures_of invariant weakened
    }

    /// Spec function bodies are single-state contexts: a behavioral
    /// predicate in such a body, specialized over a state-modifying lambda,
    /// has no pre-state scope to resolve against.
    spec fun lambda_ensures(f: |address|, a: address): bool {
        ensures_of<f>(a)
    }

    inline fun apply_addr(f: |address|, a: address) {
        f(a);
        spec {
            assert lambda_ensures(f, a);
        };
    }

    fun state_bp_in_spec_fun(a: address) {
        apply_addr(|x| {
            let R { v: _ } = move_from<R>(x);
        }, a); // error: a lambda with global state effects cannot be constrained in the body of a spec function
    }

    inline fun any(v: &vector<u64>, p: |&u64|bool): bool {
        let result = false;
        let i = 0;
        while (i < std::vector::length(v)) {
            result = p(std::vector::borrow(v, i));
            if (result) {
                break
            };
            i = i + 1
        } spec {
            invariant i <= len(v);
            invariant !result;
            invariant forall j in 0..i: !result_of<p>(v[j]);
        };
        spec {
            assert result <==> (exists j in 0..len(v): result_of<p>(v[j]));
        };
        result
    }

    /// A failing `result_of` substitution in an `any`-style HOF must also be
    /// dropped from its post-loop assertion. Otherwise the assertion
    /// retains a literal lambda after the inline expansion and later lambda
    /// lifting fails while recovering from this diagnostic.
    fun any_result_of_underivable(v: &vector<u64>): bool {
        any(v, |e| {
            let i = 0;
            while (i < *e) {
                i = i + 1;
            };
            i == 0
        }) // error: cannot resolve `result_of`, lambda needs a spec block
    }
}
