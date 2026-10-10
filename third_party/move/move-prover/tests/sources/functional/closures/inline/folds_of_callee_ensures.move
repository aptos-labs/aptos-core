// Accumulation into a capture *through a callee*, resolved by routing the
// callee's exact `&mut` post value into the fold transformer: from the
// callee's attached functional `ensures` (proven, or `[abstract]` with
// `pragma opaque` — riding the same trust as any opaque spec), from
// complete per-field conjuncts, or from the callee's own body when it is
// summarizable as a value (see `folds_of_errors.move` for the shapes which
// remain rejected).
module 0x42::folds_of_callee_ensures {
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

    spec fun spec_sum(v: vector<u64>, n: num): num {
        if (n == 0) 0 else spec_sum(v, n - 1) + v[n - 1]
    }

    spec lemma fold_is_sum(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n) == spec_sum(v, n);
    } proof {
        if (n > 0) {
            apply fold_is_sum(v, n - 1);
        }
    }

    spec lemma sum_step_bound(v: vector<u64>, i: u64, n: u64) {
        requires i < n && n <= len(v);
        ensures spec_sum(v, i) + v[i] <= spec_sum(v, n);
    } proof {
        if (i + 1 < n) {
            apply sum_step_bound(v, i, n - 1);
        }
    }

    /// A helper with a *proven* functional ensures for its `&mut`
    /// parameter.
    fun add_to(r: &mut u64, x: u64) {
        *r = *r + x;
    }
    spec add_to {
        aborts_if r + x > MAX_U64;
        ensures r == old(r) + x;
    }

    /// The through-callee accumulation, exact as if the update were
    /// performed directly in the lambda body.
    fun sum_via_callee(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| add_to(&mut sum, *e));
        sum
    }
    spec sum_via_callee {
        aborts_if spec_sum(v, len(v)) > MAX_U64;
        ensures result == spec_sum(v, len(v));
    } proof {
        apply fold_is_sum(v, len(v));
        forall n: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n)}
            apply fold_is_sum(v, n);
        forall i: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, i)}
            apply sum_step_bound(v, i, len(v));
    }

    /// A helper without any spec: the post value routes from the callee's
    /// own body (the memoized body value summary).
    fun add_to_bare(r: &mut u64, x: u64) {
        *r = *r + x;
    }

    fun sum_via_bare_callee(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| add_to_bare(&mut sum, *e));
        sum
    }
    spec sum_via_bare_callee {
        aborts_if spec_sum(v, len(v)) > MAX_U64;
        ensures result == spec_sum(v, len(v));
    } proof {
        apply fold_is_sum(v, len(v));
        forall n: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n)}
            apply fold_is_sum(v, n);
        forall i: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, i)}
            apply sum_step_bound(v, i, len(v));
    }

    /// An opaque helper whose `[abstract]` functional ensures is stated
    /// over an uninterpreted spec function: consumption rides the opaque
    /// trust, and the caller restates the fold directly.
    spec fun spec_combine(a: u64, b: u64): u64;

    fun mystery_combine(r: &mut u64, x: u64) {
        *r = *r ^ x;
    }
    spec mystery_combine {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] r == spec_combine(old(r), x);
    }

    fun fold_combine(v: &vector<u64>): u64 {
        let acc = 0;
        each_ref(v, |e| mystery_combine(&mut acc, *e));
        acc
    }
    spec fold_combine {
        aborts_if false;
        ensures result == spec_fold<u64, u64>(|a, e| spec_combine(a, e), v, 0, len(v));
    }

    /// A struct accumulator updated through an opaque helper specified by
    /// a complete set of per-field conjuncts (the `coin::merge` shape):
    /// the post value composes as a field update.
    struct Acc has copy, drop {
        total: u64,
    }

    fun bump_field(dst: &mut Acc, amount: u64) {
        dst.total = dst.total + amount;
    }
    spec bump_field {
        pragma opaque;
        aborts_if [abstract] dst.total + amount > MAX_U64;
        ensures [abstract] dst.total == old(dst.total) + amount;
    }

    fun sum_field(v: &vector<u64>): u64 {
        let acc = Acc { total: 0 };
        each_ref(v, |e| bump_field(&mut acc, *e));
        acc.total
    }
    spec sum_field {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, Acc>(
            |a: Acc, e| update_field(a, total, a.total + e), v, Acc { total: 0 }, len(v)).total;
    }

    /// An opaque MULTI-RESULT callee (the `simple_map::upsert` shape): the
    /// `result_of` carriers over its closure feed the per-iteration abort
    /// condition inside the quantified prefix invariant.
    fun two(x: u64): (u64, bool) {
        (x + 1, x > 10)
    }
    spec two {
        pragma opaque;
        aborts_if [abstract] false;
        ensures [abstract] result_1 == x + 1;
        ensures [abstract] result_2 == (x > 10);
    }

    fun count_small(v: &vector<u64>): u64 {
        let count = 0;
        each_ref(v, |e| {
            let (_a, b) = two(*e);
            if (b) {
                abort 1
            };
            count = count + 1;
        });
        count
    }
    spec count_small {
        pragma aborts_if_is_partial;
        ensures forall j in 0..len(v): v[j] <= 10;
    }

    /// Non-vacuity canary: a wrong equation must fail.
    fun sum_via_callee_wrong(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| add_to(&mut sum, *e));
        sum
    }
    spec sum_via_callee_wrong {
        pragma aborts_if_is_partial;
        ensures result == spec_sum(v, len(v)) + 1; // error: off by one
    } proof {
        apply fold_is_sum(v, len(v));
    }
}
