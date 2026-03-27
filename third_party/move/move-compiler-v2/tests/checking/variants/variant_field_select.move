// Copyright © Aptos Foundation

module 0x42::test {

    enum State has copy, drop {
        Empty,
        Value(u64),
        Continuation(|u64|u64),
    }

    enum Uniform has copy, drop {
        A(u64),
        B(u64),
    }

    // Variant-qualified field access on enum with heterogeneous field types
    fun get_value(s: &State): u64 {
        if (s is Value) {
            s.Value.0
        } else {
            0
        }
    }

    // Variant-qualified in spec expression
    spec get_value(s: &State): u64 {
        ensures (s is Value) ==> result == s.Value.0;
    }

    // Uniform field types: no variant qualification needed (plain .0 works)
    fun get_uniform(u: &Uniform): u64 {
        u.0
    }
}
