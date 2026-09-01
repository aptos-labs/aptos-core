module 0x42::shared_abort_handler {
    inline fun identity(value: bool): bool {
        value
    }

    spec identity {
        pragma opaque = true;
        aborts_if false;
        ensures result == value;
        ensures [inferred] result == value;
        aborts_if [inferred] false;
    }

    // Regression: the compiler shares the callee-abort handler with the
    // explicit abort below. WP must preserve the ordinary jump into it.
    fun abort_when_false(value: bool): u64 {
        if (identity(value)) {
            0
        } else {
            abort 7
        }
    }
    spec abort_when_false(value: bool): u64 {
        pragma opaque = true;
        ensures [inferred] identity(value) ==> result == 0;
        aborts_if [inferred] !identity(value);
    }

}
/*
Verification: Succeeded.
*/
