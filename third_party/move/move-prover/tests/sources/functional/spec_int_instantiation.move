// Spec arithmetic is typed `num`, so a struct built from it in a spec is
// instantiated at `num` while the value it is compared with has the integer
// type of the code. Both describe the same mathematical values.
module 0x42::spec_int_instantiation {
    use std::option::{Self, Option};

    struct Pair<T> has copy, drop {
        first: T,
        rest: Option<T>,
    }

    enum Choice<T> has copy, drop {
        Some { value: T },
        Many { values: vector<T>, last: T },
    }

    fun inc(x: u64): Option<u64> {
        option::some(x + 1)
    }
    spec inc {
        aborts_if x + 1 > MAX_U64;
        ensures result == option::spec_some(x + 1);
    }

    fun inc_ne(x: u64): Option<u64> {
        option::some(x + 1)
    }
    spec inc_ne {
        aborts_if x + 1 > MAX_U64;
        ensures result != option::spec_some(x);
    }

    fun pair(x: u64): Pair<u64> {
        Pair { first: x, rest: option::some(x * 2) }
    }
    spec pair {
        aborts_if x * 2 > MAX_U64;
        ensures result == Pair { first: x + 0, rest: option::spec_some(x * 2) };
    }

    fun choice(x: u64): Choice<u64> {
        Choice::Some { value: x + 1 }
    }
    spec choice {
        aborts_if x + 1 > MAX_U64;
        ensures result == Choice::Some { value: x + 1 };
    }

    spec fun same(a: Option<u64>, b: Option<u64>): bool { a == b }

    fun via_spec_fun(x: u64): Option<u64> {
        option::some(x + 1)
    }
    spec via_spec_fun {
        aborts_if x + 1 > MAX_U64;
        ensures same(result, option::spec_some(x + 1));
    }

    fun inc_wrong(x: u64): Option<u64> {
        option::some(x + 1)
    }
    spec inc_wrong {
        aborts_if x + 1 > MAX_U64;
        ensures result == option::spec_some(x + 2); // error: post-condition does not hold
    }
}
