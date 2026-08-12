// exclude_for: cvc5
// Coverage, not a regression test: enum-variant field selection in a spec
// whose function is called from a lambda, exercising behavioral wrappers
// together with $Arbitrary_value_of declarations. This shape verifies cleanly
// with or without the temporary-SpecTranslator declaration-sharing fix; that
// fix's reproducer needs a larger configuration than this suite can express,
// so this test only pins the simple combination as working.
module 0x42::bp_enum_select_wrapper {

    enum E has drop { V { x: u64 } }

    fun bump(d: u64, e: &mut E) {
        e.x += d;
    }

    spec bump {
        aborts_if e.x + d > MAX_U64;
        ensures e.x == old(e).x + d;
    }

    fun apply(f: |&mut E| has drop, e: &mut E) {
        f(e)
    }

    public fun caller(e: &mut E, d: u64) {
        apply(|s| bump(d, s), e)
    }
}
