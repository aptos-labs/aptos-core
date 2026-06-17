// also_include_for: prophecy
module 0x42::prophecy_basic {
    struct S has drop { f: u64, g: u64 }

    // Mutate a whole local through a reference.
    fun mutate_local(): u64 {
        let x = 5;
        let r = &mut x;
        *r = 7;
        x
    }
    spec mutate_local {
        ensures result == 7;
    }

    // Mutate a field of a local struct through a reference; the other field is
    // untouched. Exercises a local-root borrow followed by a field-on-reference borrow.
    fun mutate_field(): (u64, u64) {
        let s = S { f: 5, g: 10 };
        let r = &mut s.f;
        *r = 7;
        (s.f, s.g)
    }
    spec mutate_field {
        ensures result_1 == 7;
        ensures result_2 == 10;
    }
}
