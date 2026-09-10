module 0x42::abort_only {
    spec module {
        pragma aborts_if_is_strict;
    }

    // Regression: an abort-only function has the exact condition `aborts_if true`.
    fun always_aborts() {
        abort 0
    }
    spec always_aborts() {
        pragma opaque = true;
        aborts_if [inferred] true;
    }

}
/*
Verification: Succeeded.
*/
