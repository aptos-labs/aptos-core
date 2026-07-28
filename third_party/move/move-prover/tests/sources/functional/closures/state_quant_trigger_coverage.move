// Copyright © Aptos Foundation
// State-quantified behavioral predicates whose implicit trigger cannot cover all
// bound variables: a quantified value variable not occurring in the predicate,
// and a closure-literal predicate whose function reads only part of its type's
// memory union. Both verify (the quantifier falls back to solver-chosen
// instantiation instead of an invalid trigger).
module 0x42::state_quant_trigger_coverage {
    use std::signer;

    struct Counter has key { v: u64 }
    struct Other has key { v: u64 }

    struct Mod has key, drop {
        f: |&signer| has copy+store+drop,
    }
    spec Mod {
        modifies_of<f>(s: signer) Counter[signer::address_of(s)];
        // `i` occurs only outside the behavioral predicate.
        invariant forall S in *, s: signer, i: u64: i > 0 ==> (S |~ !aborts_of<f>(s));
    }

    #[persistent]
    fun safe_inc(s: &signer) {
        let addr = signer::address_of(s);
        if (exists<Counter>(addr) && Counter[addr].v < 18446744073709551615) {
            Counter[addr].v = Counter[addr].v + 1;
        }
    }
    spec safe_inc {
        pragma opaque;
        modifies Counter[signer::address_of(s)];
        aborts_if false;
    }

    public fun make(owner: &signer) {
        move_to(owner, Mod { f: safe_inc });
    }

    // Two same-type functions with different memory footprints: the evaluator's
    // type union exceeds each function's own memory.
    fun rd_counter(a: address): u64 acquires Counter {
        Counter[a].v
    }
    spec rd_counter {
        pragma opaque;
        aborts_if !exists<Counter>(a);
        ensures result == Counter[a].v;
    }

    fun rd_other(a: address): u64 acquires Other {
        Other[a].v
    }
    spec rd_other {
        pragma opaque;
        aborts_if !exists<Other>(a);
        ensures result == Other[a].v;
    }

    fun call_rd_counter(a: address): u64 acquires Counter {
        rd_counter(a)
    }
    spec call_rd_counter {
        aborts_if !exists<Counter>(a);
        ensures forall S in *: (S |~ exists<Counter>(a)) ==> (S |~ !aborts_of<rd_counter>(a));
    }

    fun touch_other(a: address): u64 acquires Other {
        rd_other(a)
    }
    spec touch_other {
        aborts_if !exists<Other>(a);
    }
}
