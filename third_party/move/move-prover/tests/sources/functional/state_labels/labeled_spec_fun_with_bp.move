// Copyright © Aptos Foundation
// State-labeled calls of spec-function wrappers whose bodies (transitively)
// contain behavioral predicates over concrete functions. The predicate
// target's spec memory is folded into the spec function's memory parameters,
// so the labeled state's memory threads through the call.
module 0x42::labeled_spec_fun_with_bp {
    struct Counter has key { value: u64 }

    fun bump(a: address) acquires Counter {
        Counter[a].value = Counter[a].value + 1;
    }
    spec bump {
        aborts_if !exists<Counter>(a);
        aborts_if Counter[a].value + 1 > MAX_U64;
    }

    spec fun wont_abort(a: address): bool {
        !aborts_of<bump>(a)
    }

    spec fun wraps(a: address): bool {
        wont_abort(a)
    }

    fun touch(a: address) acquires Counter {
        bump(a);
    }
    spec touch {
        pragma aborts_if_is_partial;
        // In any state where the counter exists and cannot overflow, `bump`
        // does not abort.
        ensures forall S in *:
            (S |~ (exists<Counter>(a) && Counter[a].value < 1000))
                ==> (S |~ wont_abort(a));
        // The same through a transitive wrapper.
        ensures forall S in *:
            (S |~ (exists<Counter>(a) && Counter[a].value < 1000))
                ==> (S |~ wraps(a));
    }

    fun touch_bad(a: address) acquires Counter {
        bump(a);
    }
    spec touch_bad {
        pragma aborts_if_is_partial;
        ensures forall S in *: S |~ wont_abort(a); // error: `bump` aborts in a state without the counter
    }
}
