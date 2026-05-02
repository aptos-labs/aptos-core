// Tests behavioural-predicate `result_of` on functions that return references.
// The Move spec language treats references transparently: `result` for a
// function returning `&T` is auto-derefed to `T`. The BP machinery must declare
// `$bp_ensures_of_result'…'(args)` with that same value type — historically it
// declared with the raw return type (`$Mutation T`), causing Boogie type
// errors on the natural equality `result == result_of<f>(args)`.
//
// Non-generic so the apply-procedure call/declaration suffix issue (orthogonal
// bug for closure targets with type parameters) doesn't fire first.
//
// The functions here have explicit specs relating result to inputs so the BP
// equality is provable end-to-end, which is the strongest signal that the
// reference handling matches between `result` and `result_of`.
module 0x42::bp_ref_result {
    struct S has copy, drop, store {
        v: u64,
    }

    /// `&u64` return: the BP result-fun must declare type `int` (not
    /// `$Mutation int`) so the equality with `result` (also `int`) type-checks.
    fun get(s: &S): &u64 {
        &s.v
    }
    spec get {
        ensures result == s.v;
    }

    /// `&S` return: same as above for a struct value.
    fun mk(): &S {
        abort 0
    }
    spec mk {
        aborts_if true;
    }

    /// Caller exercising `result_of<get>(s)`. This is the equality that
    /// historically failed Boogie type-checking with `expected int, got
    /// $Mutation int`.
    fun call_and_check(s: &S): u64 {
        let r = get(s);
        *r
    }
    spec call_and_check {
        ensures result == result_of<get>(s);
    }
}
