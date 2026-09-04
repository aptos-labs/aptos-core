// `pragma intrinsic` says the prover implements the function itself. The
// backend therefore emits no body for it. When no intrinsic is actually
// registered -- as for the `cmp::is_*` predicates, which rely on `pragma
// opaque` beside it -- there is no prelude procedure either, so a call site
// translated to a direct call named a procedure that was never declared and
// Boogie stopped with a name-resolution error: an internal failure carrying no
// source location instead of a verdict on the specification.
//
// Having no body the prover can use is what opaque means, so the call goes
// through the specification instead.
module 0x42::intrinsic_without_opaque {

    fun bounded(x: u64): u64 {
        if (x > 10) { 10 } else { x }
    }

    spec bounded {
        pragma intrinsic;
        ensures result <= 10;
    }

    fun caller(x: u64): u64 {
        bounded(x)
    }

    spec caller {
        ensures result <= 10;
    }

    fun caller_expecting_the_body(x: u64): u64 {
        bounded(x)
    }

    spec caller_expecting_the_body {
        // The contract is all the caller gets: the body says the result is `x`
        // whenever `x <= 10`, but an intrinsic's body is not the prover's.
        ensures result == x; // error: the specification of `bounded` does not say this
    }
}
