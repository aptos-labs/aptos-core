// Function-level specs on inline functions.
//
// Two flavors are exercised:
//   1. Non-opaque inline functions: body is inlined at call sites as today.
//      The body is also verified against the spec independently.
//   2. Opaque inline functions: at call sites the prover substitutes the spec
//      instead of the inlined body, just like a normal opaque function.
//      The body is still verified against the spec independently.
module 0x42::TestInlineFunSpec {

    spec module {
        pragma verify = true;
    }

    // -- (1) non-opaque inline ------------------------------------------------

    public inline fun add_one(x: u64): u64 {
        x + 1
    }
    spec add_one {
        aborts_if x == 0xFFFFFFFFFFFFFFFF;
        ensures result == x + 1;
    }

    public fun caller_inline(): u64 {
        add_one(41)
    }
    spec caller_inline {
        ensures result == 42;
    }

    // -- (2) opaque inline ----------------------------------------------------

    // Body returns `x + 1`. The spec is the canonical contract used at call
    // sites; the body is verified against this spec.
    public inline fun add_one_opaque(x: u64): u64 {
        x + 1
    }
    spec add_one_opaque {
        pragma opaque = true;
        aborts_if x == 0xFFFFFFFFFFFFFFFF;
        ensures result == x + 1;
    }

    // The caller's post-condition is discharged by the callee's spec, not by
    // looking at the inlined body — `pragma opaque` hides the body at the
    // call site.
    public fun caller_opaque(): u64 {
        add_one_opaque(41)
    }
    spec caller_opaque {
        ensures result == 42;
    }
}
