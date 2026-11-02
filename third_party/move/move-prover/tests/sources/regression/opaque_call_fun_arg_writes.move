// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A function-typed parameter may modify the memory its `modifies_of` frame declares, directly or
// through an opaque callee.

module 0x42::opaque_call_fun_arg_writes {
    struct R has key { value: u64 }

    fun forward_at(fv: |address| has drop, a: address) {
        fv(a);
    }
    spec forward_at {
        pragma opaque;
        modifies_of<fv>(x: address) R[x];
        ensures ensures_of<fv>(a);
        aborts_if aborts_of<fv>(a);
    }

    /// Must fail: the opaque `forward_at` passes the frame on.
    public fun forwarded_at(fv: |address| has drop, a: address) {
        forward_at(fv, a);
    }
    spec forwarded_at {
        modifies_of<fv>(x: address) R[x];
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }

    /// Must verify: the effect of `fv` is described by its behavioral predicates.
    public fun forwarded_at_behavior(fv: |address| has drop, a: address) {
        forward_at(fv, a);
    }
    spec forwarded_at_behavior {
        modifies_of<fv>(x: address) R[x];
        ensures ensures_of<fv>(a);
        aborts_if aborts_of<fv>(a);
    }

    /// Must fail: the frame allows `R[a]` to change.
    public fun addressed(fv: |address| has drop, a: address) {
        fv(a);
    }
    spec addressed {
        modifies_of<fv>(x: address) R[x];
        requires exists<R>(a);
        ensures R[a].value == old(R[a].value);
    }
}
