// Copyright © Aptos Foundation
// Transitive spec-function wrappers around behavioral predicates, including a
// wrapper mixing a behavioral predicate with a direct global read of another
// resource: the folded predicate memory and the directly used memory both
// thread through a labeled call.
module 0x42::labeled_spec_fun_bp_transitive {
    struct Counter has key { value: u64 }
    struct Cap has key { limit: u64 }

    fun bump(a: address) acquires Counter, Cap {
        assert!(Counter[a].value < Cap[a].limit, 1);
        Counter[a].value = Counter[a].value + 1;
    }
    spec bump {
        aborts_if !exists<Counter>(a);
        aborts_if !exists<Cap>(a);
        aborts_if Counter[a].value >= Cap[a].limit;
        aborts_if Counter[a].value + 1 > MAX_U64;
    }

    spec fun wont_abort(a: address): bool {
        !aborts_of<bump>(a)
    }

    // Mixes the transitive predicate with a direct read of `Cap`.
    spec fun under_cap(a: address): bool {
        wont_abort(a) && Cap[a].limit > 0
    }

    fun touch(a: address) acquires Counter, Cap {
        bump(a);
    }
    spec touch {
        pragma aborts_if_is_partial;
        // Note: quantified states are unconstrained, so the hypothesis must
        // pin down everything the wrapper needs (including `limit > 0`, which
        // does not follow from well-typedness in a symbolic state).
        ensures forall S in *:
            (S |~ (exists<Counter>(a) && exists<Cap>(a)
                && Counter[a].value < Cap[a].limit
                && Cap[a].limit > 0 && Cap[a].limit < 1000))
                ==> (S |~ under_cap(a));
        ensures forall S in *:
            (S |~ (exists<Counter>(a) && Counter[a].value < 1000))
                ==> (S |~ under_cap(a)); // error: says nothing about the cap
    }
}
