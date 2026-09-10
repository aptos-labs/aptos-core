// The essential capture-writing HOF pattern: `let sum = 0;
// each_ref(v, |e| sum = sum + *e)`, specified exactly through the single
// `folds_of` predicate in the generic loop invariant of the inline HOF. The
// inliner derives the lambda's accumulator transformer `|acc, e| acc + e`
// from its body, snapshots the capture at expansion entry, and specializes
// the module's `spec_fold` recursion over the transformer — through the same
// literal-lambda path as caller restatements, so the bridging lemma below
// and the expanded invariant speak about the same specialized symbol
// (compare `fold_symbolic.move`, where the accumulator is threaded by
// value instead of captured).
module 0x42::folds_of {
    use std::vector;

    /// The generic HOF: one invariant set serves every lambda class through
    /// the single `folds_of` predicate — a capture-writing lambda gets the
    /// fold equation plus the prefix no-abort condition, a pure lambda
    /// degenerates to prefix no-abort alone.
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

    /// The fold recursion specialized by `folds_of` (and restatable by
    /// callers). `std::vector` does not declare one, so `folds_of` resolves
    /// it in the current module.
    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    /// The user's own abstraction: sum of the first `n` elements.
    spec fun spec_sum(v: vector<u64>, n: num): num {
        if (n == 0) 0 else spec_sum(v, n - 1) + v[n - 1]
    }

    /// Bridging lemma: the fold with the addition transformer equals
    /// `spec_sum`. The literal lambda restates the transformer derived from
    /// `|e| sum = sum + *e` and unifies onto the same specialization.
    spec lemma fold_is_sum(v: vector<u64>, n: u64) {
        requires n <= len(v);
        ensures spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n) == spec_sum(v, n);
    } proof {
        if (n > 0) {
            apply fold_is_sum(v, n - 1);
        }
    }

    /// A partial sum plus the next element is bounded by any later partial
    /// sum (elements are non-negative). Proven by induction.
    spec lemma sum_step_bound(v: vector<u64>, i: u64, n: u64) {
        requires i < n && n <= len(v);
        ensures spec_sum(v, i) + v[i] <= spec_sum(v, n);
    } proof {
        if (i + 1 < n) {
            apply sum_step_bound(v, i, n - 1);
        }
    }

    /// The essential case: exact functional postcondition and exact abort
    /// condition for a capture-accumulating loop.
    fun sum(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| sum = sum + *e);
        sum
    }
    spec sum {
        aborts_if spec_sum(v, len(v)) > MAX_U64;
        ensures result == spec_sum(v, len(v));
    } proof {
        apply fold_is_sum(v, len(v));
        forall n: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, n)}
            apply fold_is_sum(v, n);
        forall i: u64 {spec_fold<u64, u64>(|acc, e| acc + e, v, 0, i)}
            apply sum_step_bound(v, i, len(v));
    }

    /// A caller specified directly against `spec_fold` needs no lemma at
    /// all: the restated lambda unifies with the derived transformer, so
    /// the postcondition is the loop-exit fact verbatim.
    fun sum_direct(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| sum = sum + *e);
        sum
    }
    spec sum_direct {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>(|acc, e| acc + e, v, 0, len(v));
    }

    struct NoAbilities {
        total: u64,
    }

    /// Capture snapshots require no abilities.
    fun sum_without_abilities(v: &vector<u64>): u64 {
        let acc = NoAbilities { total: 0 };
        each_ref(v, |e| acc.total = acc.total + *e);
        let NoAbilities { total } = acc;
        total
    }
    spec sum_without_abilities {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, NoAbilities>(
            |acc: NoAbilities, e| update_field(acc, total, acc.total + e),
            v,
            NoAbilities { total: 0 },
            len(v),
        ).total;
    }

    /// Mutated parameter captures are normalized for derivation.
    fun accumulate_into_parameter(v: &vector<u64>, acc: &mut NoAbilities) {
        each_ref(v, |e| acc.total = acc.total + *e);
    }
    spec accumulate_into_parameter {
        pragma aborts_if_is_partial;
        ensures acc.total == spec_fold<u64, NoAbilities>(
            |acc: NoAbilities, e| update_field(acc, total, acc.total + e),
            v,
            old(acc),
            len(v),
        ).total;
    }

    /// Pure-lambda degeneration through the same HOF: no captures, so
    /// `folds_of` reduces to the prefix no-abort condition, giving the
    /// exact abort condition of the division.
    fun div_all(v: &vector<u64>, k: u64): u64 {
        each_ref(v, |e| { let _ = k / *e; });
        k
    }
    spec div_all {
        aborts_if exists j in 0..len(v): v[j] == 0;
        ensures result == k;
    }

    /// A generic pointwise `ensures_of` invariant (the shape used by HOFs
    /// which constrain their lambda per application) resolves for a
    /// capture-writing lambda by dropping the capture-mentioning conjuncts:
    /// a sound weakening which keeps such HOFs applicable, with the capture
    /// facts carried by `folds_of` instead.
    inline fun each_val(v: &vector<u64>, f: |u64|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(*vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant forall j in 0..i: ensures_of<f>(v[j]);
        };
    }

    fun ensures_of_weakened(v: &vector<u64>): u64 {
        let sum = 0;
        each_val(v, |y| sum = sum + y);
        sum
    }
    spec ensures_of_weakened {
        // The weakened predicate carries no facts about `sum`; this only
        // checks that the capture-writing lambda resolves without errors.
        pragma aborts_if_is_partial;
    }

    /// Non-vacuity canary: a wrong equation must fail.
    fun sum_wrong(v: &vector<u64>): u64 {
        let sum = 0;
        each_ref(v, |e| sum = sum + *e);
        sum
    }
    spec sum_wrong {
        pragma aborts_if_is_partial;
        ensures result == spec_sum(v, len(v)) + 1; // error: off by one
    } proof {
        apply fold_is_sum(v, len(v));
    }
}
