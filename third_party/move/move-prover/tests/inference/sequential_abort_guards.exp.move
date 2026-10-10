module 0x42::sequential_abort_guards {

    // Two abort sites in sequence. The weakest precondition conjoins the second
    // obligation with the guard that reached it, which restates the negation of
    // the first clause. Read as a disjunction the restatement adds nothing, so
    // the second clause should mention only its own condition.
    fun checked_divide(numerator: u64, denominator: u64): u64 {
        assert!(numerator >= denominator, 1);
        numerator / denominator
    }
    spec checked_divide(numerator: u64, denominator: u64): u64 {
        pragma opaque = true;
        ensures [inferred] numerator >= denominator ==> result == numerator / denominator;
        aborts_if [inferred] numerator < denominator;
        aborts_if [inferred] denominator == 0;
    }


    // Three in sequence, so the third clause would otherwise restate both
    // earlier guards.
    fun checked_scale(value: u64, factor: u64, divisor: u64): u64 {
        assert!(value > 0, 2);
        assert!(factor > 0, 3);
        value * factor / divisor
    }
    spec checked_scale(value: u64, factor: u64, divisor: u64): u64 {
        pragma opaque = true;
        ensures [inferred] value > 0 && factor > 0 ==> result == value * factor / divisor;
        aborts_if [inferred] value == 0;
        aborts_if [inferred] factor == 0;
        aborts_if [inferred] divisor == 0;
        aborts_if [inferred] value * factor > MAX_U64;
    }

}
/*
Verification: Succeeded.
*/
