// Copyright © Aptos Foundation

module 0x42::test {

    enum State has copy, drop {
        Empty,
        Value(u64),
        Continuation(|u64|u64),
    }

    // Error: field does not exist in the specified variant
    fun bad_field(s: &State): u64 {
        s.Empty.0
    }

    // Error: not a variant name (no variant called "Foo")
    fun bad_variant(s: &State): u64 {
        s.Foo.0
    }
}
