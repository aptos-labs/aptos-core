// A partial opaque callee can both return and abort without exposing a usable
// abort predicate. WP must retain a source-level partial boundary for the
// caller after discarding the uninformative `aborts_if true` candidate.
module 0x42::partial_abort_fallback {
    fun maybe_abort() {}

    spec maybe_abort {
        pragma opaque;
        pragma verify = false;
        pragma aborts_if_is_partial;
        pragma inference = none;
    }

    fun caller() {
        maybe_abort()
    }
}
