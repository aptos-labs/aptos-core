// `folds_of` warning and error cases; see `folds_of.move` for positive cases.
module 0x42::folds_of_errors {
    use std::vector;

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

    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    struct R has key {
        v: u64,
    }

    /// A lambda combining a capture write with a global state read: the
    /// per-iteration evaluation state of the transformer is not
    /// expressible in a loop invariant.
    fun capture_and_state(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| sum = sum + *e + R[@0x1].v); // warning: capture write combined with global state access
        sum
    }

    /// A lambda body containing a loop: the per-iteration effect cannot be
    /// derived (no invariants are available at derivation time).
    fun loop_in_body(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| {
            let x = *e;
            while (x > 0) {
                x = x - 1;
                sum = sum + 1;
            }
        }); // warning: the per-iteration effect cannot be derived exactly
        sum
    }

    /// An opaque callee with a relational-only spec: no functional
    /// `ensures p == E(old(p), ..)` to consume, and the body — being
    /// opaque — is not consulted.
    fun add_to_relational(r: &mut u64, x: u64) {
        *r = *r + x;
    }
    spec add_to_relational {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] r >= old(r);
    }

    /// A lambda accumulating into the capture through a callee whose
    /// effect on the `&mut` parameter is not expressible as a value: the
    /// updated capture is known only through the callee's `ensures_of`,
    /// not as a value the fold transformer could restate.
    fun accumulates_through_relational_callee(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| add_to_relational(&mut sum, *e)); // warning: accumulation through a relational callee weakened
        sum
    }

    /// A spec-less callee whose body is underivable (a loop): neither the
    /// spec nor the body value summary can route the post value.
    fun add_to_looping(r: &mut u64, x: u64) {
        while (x > 0) {
            *r = *r + 1;
            x = x - 1;
        }
    }

    fun accumulates_through_looping_callee(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| add_to_looping(&mut sum, *e)); // warning: accumulation through a looping callee weakened
        sum
    }

    fun looping_result(x: u64): u64 {
        while (x > 0) {
            x = x - 1;
        };
        x
    }

    /// An unresolved nested result weakens the fold invariant.
    fun transformer_with_underivable_result(v: &vector<u64>): vector<u64> {
        let out = vector[];
        each_ref(v, |e| vector::push_back(&mut out, looping_result(*e)));
        out
    }

    fun set_two(a: &mut u64, b: &mut u64) {
        *a = 1;
        *b = 2;
    }

    struct Pair has copy, drop {
        x: u64,
        y: u64,
    }

    /// Two `&mut` arguments rooted in the same capture: the post-value
    /// routing conservatively treats them as aliasing and falls back to
    /// symbolic post-states, which the fold transformer cannot restate.
    fun aliasing_mut_args(v: &vector<u64>): u64 {
        let p = Pair { x: 0, y: 0 };
        each_ref(v, |_e| set_two(&mut p.x, &mut p.y)); // warning: capture values not expressible
        p.x + p.y
    }

    inline fun apply_one(v: &vector<u64>, f: |&u64|) {
        f(vector::borrow(v, 0));
        spec {
            assert folds_of<f>(v, 1); // error: folds_of can only be used in a loop invariant
        };
    }

    /// `folds_of` outside of a loop invariant.
    fun outside_loop_invariant(v: &vector<u64>) {
        apply_one(v, |e| {
            let _ = 1 / *e;
        });
    }

    /// More captures than the generated recursion's tuple return supports.
    fun too_many_captures(v: &vector<u64>): u64 {
        let c1 = 0;
        let c2 = 0;
        let c3 = 0;
        let c4 = 0;
        let c5 = 0;
        let c6 = 0;
        let c7 = 0;
        let c8 = 0;
        let c9 = 0;
        each_ref(v, |e| {
            // error: more captures than the supported maximum
            c1 = c1 + *e;
            c2 = c2 + *e;
            c3 = c3 + *e;
            c4 = c4 + *e;
            c5 = c5 + *e;
            c6 = c6 + *e;
            c7 = c7 + *e;
            c8 = c8 + *e;
            c9 = c9 + *e;
        });
        c1 + c9
    }

    /// A lambda writing a parameter of the enclosing function: parameter
    /// reads and writes cannot be connected by the derivation.
    fun writes_parameter(v: &vector<u64>, acc: u64): u64 {
        let _ = &acc;
        each_ref(v, |e| acc = acc + *e); // error: writes a parameter of the enclosing function
        acc
    }

    inline fun zip_with_self(v: &vector<u64>, f: |u64, u64|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(*vector::borrow(v, i), *vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            // error: the index function must produce a literal tuple
            invariant folds_of<f>(|j| if (j > 0) (v[j], v[j]) else (v[0], v[0]), i);
        };
    }

    /// A general-form index function whose body is not a literal tuple of
    /// the target's arguments.
    fun non_literal_index_args(v: &vector<u64>): u64 {
        let sum = 0;
        zip_with_self(v, |x, y| sum = sum + x + y);
        sum
    }

    inline fun each_off(v: &vector<u64>, k: u64, f: |u64|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(k + i);
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(|j| k + j, i);
        };
    }

    /// The index function's arguments depend on the captured variable
    /// through the actual argument bound to `k`.
    fun index_args_depend_on_capture(v: &vector<u64>): u64 {
        let sum = 0;
        each_off(v, sum, |x| sum = sum + x); // error: index arguments depend on a written capture
        sum
    }

    // ===== Forwarding-deferral boundaries (see `folds_of_wrapper.move`
    // for the positive cases) =====

    /// A *mixed* forwarder shape: the lambda both writes a capture and
    /// forwards to the enclosing wrapper's function-typed parameter — the
    /// capture's transformer would have to be parameterized over the
    /// parameter's eventual lambda, which the deferral cannot express.
    inline fun each_mapped(v: &vector<u64>, f: |&u64| u64) {
        let out = vector[];
        each_ref(v, |e| vector::push_back(&mut out, f(e))); // error: capture write mixed with forwarding
    }

    fun map_double(v: &vector<u64>) {
        each_mapped(v, |e| *e * 2);
    }

    /// A forwarder to a genuine function value: the enclosing function is
    /// not inline, so the deferred occurrence would target a function
    /// value with no statically known body at the final level.
    fun apply_all(v: &vector<u64>, f: |&u64|) {
        each_ref(v, |e| f(e)); // error: folds_of deferral needs an inline enclosing function
    }

    fun read_r(a: address): u64 acquires R {
        borrow_global<R>(a).v
    }

    // `used_memory` does not include the behavioral evaluator's memory, so
    // this wrapper must be inspected transitively by the folds classifier.
    spec fun wrapped_stateful_index(): u64 {
        result_of<read_r>(@0x1)
    }

    inline fun each_stateful_index(n: u64, f: |u64|) {
        let i = 0;
        while (i < n) {
            f(i);
            i = i + 1;
        } spec {
            invariant i <= n;
            // error: a behavioral predicate can observe changing memory
            invariant folds_of<f>(|_j| wrapped_stateful_index(), i);
        };
    }

    /// A default-range behavioral predicate is still stateful: the target
    /// function's current memory is supplied to its evaluator.
    fun stateful_behavior_index(n: u64) {
        each_stateful_index(n, |_x| {});
    }
}
