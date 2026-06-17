// Copyright © Aptos Foundation
// A data invariant relating behavioral predicates of two *different* stored
// function fields cannot be lifted into either field's witness axioms —
// rewriting an occurrence to the other field's witness would fabricate an
// assumption that is never checked. The lifting bails on the field mismatch;
// the pack-time assert still enforces the invariant, as the intended failure
// below shows.
module 0x42::data_inv_cross_field {
    struct S has key, drop {
        f: |u64|u64 has copy + store + drop,
        g: |u64|u64 has copy + store + drop,
    }
    spec S {
        invariant forall Q in *, x: u64:
            Q |~ (!aborts_of<f>(x) && !aborts_of<g>(x) ==>
                result_of<g>(x) >= result_of<f>(x));
    }

    #[persistent]
    fun id(x: u64): u64 {
        x
    }
    spec id {
        pragma opaque;
        aborts_if false;
        ensures result == x;
    }

    #[persistent]
    fun inc(x: u64): u64 {
        x + 1
    }
    spec inc {
        pragma opaque;
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }

    fun store_ok(s: &signer) {
        move_to(s, S { f: |x| id(x), g: |x| inc(x) });
    }

    fun store_violates(s: &signer) {
        move_to(s, S { f: |x| inc(x), g: |x| id(x) }); // error: data invariant does not hold
    }
}
