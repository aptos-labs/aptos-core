// Function types normalize (ability-stripped, tuple-flattened) at every
// nesting depth, matching the erasure Boogie type names apply: two wrapper
// instantiations whose stored function fields differ only in the abilities
// of a nested function type must produce a single canonical function type
// (one closure datatype, one field constructor), not two that mangle to the
// same Boogie name.
module 0x42::fun_field_nested_ability_variants {
    use std::option::{Self, Option};

    struct A has drop {
        f: Option<|(|u64| has drop)| has drop + store>,
    }
    struct B has drop {
        f: Option<|(|u64| has drop + copy)| has drop + store>,
    }

    public fun run_a(g: |u64| has drop) {
        g(1)
    }
    public fun run_b(g: |u64| has drop + copy) {
        g(2)
    }

    fun mk_a(): A {
        A { f: option::some(run_a) }
    }
    spec mk_a {
        aborts_if false;
    }

    fun mk_b(): B {
        B { f: option::some(run_b) }
    }
    spec mk_b {
        aborts_if false;
    }

    // The identity-constraint lookup must use the same canonical keys as
    // the registration: an ability-carrying function type argument
    // (normalization strips abilities) previously missed the lookup,
    // silently dropping the `is $struct_field` well-formedness assumption.
    fun probe_identity(o: Option<|u64| has drop + store>): bool {
        option::is_some(&o)
    }
    spec probe_identity {
        aborts_if false;
    }
}
