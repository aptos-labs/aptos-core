module 0x42::zero_arg_behavior {
    fun enabled(): bool {
        true
    }

    spec enabled {
        pragma opaque = true;
        aborts_if false;
        ensures result;
    }

    // Regression: the Boogie application for a zero-argument function's
    // `ensures_of` predicate must not start its result arguments with a comma.
    fun read_enabled(): bool {
        enabled()
    }

    spec read_enabled {
        ensures ensures_of<enabled>(result_of<enabled>());
    }
}
