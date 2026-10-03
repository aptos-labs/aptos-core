// A struct field and the branches of a conditional are unified without
// integer variance below the top level, so a value built from spec arithmetic
// (`Option<num>`) where the code's type (`Option<u64>`) is expected is a type
// error there, unlike in an equality or a spec function argument.
module 0x42::spec_int_instantiation_err {
    use std::option::{Self, Option};

    struct Pair<T> has copy, drop {
        first: T,
        rest: Option<T>,
    }

    fun pack_field(x: u64): Pair<u64> {
        Pair { first: x, rest: option::some(x + 1) }
    }
    spec pack_field {
        aborts_if x + 1 > MAX_U64;
        ensures result == Pair { first: x, rest: option::spec_some(x + 1) }; // error: expected `Option<u64>`
    }

    fun either(c: bool, x: u64): Option<u64> {
        if (c) option::some(x + 1) else option::none()
    }
    spec either {
        aborts_if c && x + 1 > MAX_U64;
        ensures result == (if (c) option::spec_some(x + 1) else option::spec_none<u64>()); // error: expected `Option<u64>`
    }
}
