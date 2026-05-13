// Copyright © Aptos Foundation
// (no language-version flag needed: reads/writes in spec blocks are allowed at 2.4)
module 0x42::transitions {

    struct R has key { val: u64 }
    struct S has key { val: u64 }

    // An uninterpreted spec fun with modifies/reads declaring resource access.
    spec fun uninterpreted_incr_R(a: address): bool modifies R[a] reads R;

    // A spec fun with body and matching modifies/reads.
    spec fun incr_R(a: address): bool modifies R[a] reads R {
        R[a].val == old(R[a].val) + 1
    }

    // A function that increments R's val field.
    fun do_incr(a: address) {
        R[a].val = R[a].val + 1;
    }
    spec do_incr {
        ensures incr_R(a);
    }

    // A spec fun that describes setting R's val to a specific value.
    // Only reads R since it doesn't use old().
    spec fun set_R(a: address, v: u64): bool reads R {
        R[a].val == v
    }

    // A function that sets R's val field.
    fun do_set(a: address, v: u64) {
        R[a].val = v;
    }
    spec do_set {
        ensures set_R(a, v);
    }

    // Negative test: spec fun body that doesn't hold.
    spec fun wrong_incr_R(a: address): bool modifies R[a] reads R {
        R[a].val == old(R[a].val) + 2  // Wrong: increments by 2 instead of 1
    }

    fun do_incr_wrong(a: address) {
        R[a].val = R[a].val + 1;
    }
    spec do_incr_wrong {
        ensures wrong_incr_R(a);
    }

    // Test modifies_of: associates access specifiers with a function-typed parameter.
    fun apply(a: address, f: |address|) {
        f(a)
    }
    spec apply {
        modifies_of<f>(a: address) R[a];
    }
}
