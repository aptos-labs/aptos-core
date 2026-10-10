// Tests `result_of` for a nullary, effect-free function value. The generated
// equivalence axiom has no quantifier bindings and must not use `forall`.
module 0x42::result_of_nullary {
    fun apply(f: ||u64): u64 {
        f()
    }
    spec apply {
        ensures result == result_of<f>();
    }

    fun known(): u64 {
        apply(|| 7)
    }
    spec known {
        ensures result == 7;
    }
}
