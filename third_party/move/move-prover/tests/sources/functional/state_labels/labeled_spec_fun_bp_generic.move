// Copyright © Aptos Foundation
// Generic behavioral predicate targets inside generic spec functions: the
// folded predicate memory is instantiated per call of the spec function.
module 0x42::labeled_spec_fun_bp_generic {
    struct Box<T: store> has key { v: T }

    fun put<T: store + drop>(a: address, x: T) acquires Box {
        Box<T>[a].v = x;
    }
    spec put {
        aborts_if !exists<Box<T>>(a);
    }

    spec fun can_put<T: store + drop>(a: address, x: T): bool {
        !aborts_of<put>(a, x)
    }

    fun test(a: address) acquires Box {
        put(a, 1u64);
        put(a, true);
    }
    spec test {
        pragma aborts_if_is_partial;
        ensures forall S in *:
            (S |~ exists<Box<u64>>(a)) ==> (S |~ can_put(a, 0u64));
        ensures forall S in *:
            (S |~ exists<Box<bool>>(a)) ==> (S |~ can_put(a, false));
        ensures forall S in *:
            (S |~ exists<Box<u64>>(a)) ==> (S |~ can_put(a, false)); // error: wrong instantiation
    }
}
