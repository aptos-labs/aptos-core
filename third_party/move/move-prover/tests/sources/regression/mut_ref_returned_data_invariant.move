// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// A `&mut` parameter's data invariant is owed at the function's exit whether
// or not the reference is moved into the result. The slot the reference left
// is empty, so no other obligation covers it, and the caller reads the
// returned reference as one whose current satisfies its invariants; a
// function that returns a reference over a broken value is rejected here.

module 0x42::mut_ref_returned_data_invariant {

    struct S { v: u64 }
    spec S {
        invariant v == 0;
    }

    struct Outer { inner: S, n: u64 }

    /// Rejected: the invariant is broken where the parameter's loan ends.
    public fun break_and_keep(s: &mut S) {
        s.v = 1;
    }

    /// Rejected for the same reason: moving the reference into the result
    /// does not discharge the obligation, it only moves where it is held.
    public fun break_and_return(s: &mut S): &mut S {
        s.v = 1;
        s
    }

    /// Must verify: the invariant holds of the current handed to the caller.
    public fun keep_and_return(s: &mut S): &mut S {
        s.v = 0;
        s
    }

    /// Rejected here, accepted by the Boogie backend, whose `PackRefDeep` at
    /// the exit covers the parameter and not the reborrow it handed on: the
    /// reborrow's current reaches the caller as one whose invariants hold, so
    /// it is owed where it leaves. The two baselines differ for this reason.
    public fun return_reborrow_broken(o: &mut Outer): &mut S {
        o.inner.v = 1;
        &mut o.inner
    }

    /// Must verify.
    public fun return_reborrow_ok(o: &mut Outer): &mut S {
        o.inner.v = 0;
        &mut o.inner
    }

    /// Rejected: the returned referent is certificate-free, but the
    /// parameter's own referent is broken and its loan still ends here.
    public fun return_free_reborrow_broken(o: &mut Outer): &mut u64 {
        o.inner.v = 1;
        &mut o.n
    }

    /// Must verify.
    public fun return_free_reborrow_ok(o: &mut Outer): &mut u64 {
        o.inner.v = 0;
        &mut o.n
    }

    /// Must verify: a caller of a function returning a `&mut` is not
    /// vacuous, and reads the invariant of the reference it received.
    public fun caller(s: &mut S): bool {
        let r = keep_and_return(s);
        r.v == 0
    }

    spec caller {
        ensures result;
    }

    // The independent review's reproducer: an explicit early return. The
    // wrapper certified only the fall-through outcome, and `Flow.bind` passes
    // `return` through, so a reference whose referent violates its invariant
    // escaped. Nested and loop forms are kept because the frontend normalizes
    // a function-tail return into a value expression, which masks the defect.

    /// Rejected: the early return hands out a broken referent.
    public fun early_return_broken(b: bool, s: &mut S): &mut S {
        if (b) {
            s.v = 1;
            return s
        };
        s
    }

    /// Must verify: the same shape, invariant preserved.
    public fun early_return_ok(b: bool, s: &mut S): &mut S {
        if (b) {
            s.v = 0;
            return s
        };
        s
    }

    /// Rejected: an early return nested two branches deep.
    public fun nested_return_broken(b: bool, c: bool, s: &mut S): &mut S {
        if (b) {
            if (c) {
                s.v = 1;
                return s
            };
            s
        } else {
            s
        }
    }

    /// Rejected: an early return out of a loop.
    public fun loop_return_broken(n: u64, s: &mut S): &mut S {
        let i = 0;
        while (i < n) {
            s.v = 1;
            return s
        };
        s
    }

    /// Must verify: an early return out of a loop, invariant preserved.
    public fun loop_return_ok(n: u64, s: &mut S): &mut S {
        let i = 0;
        while (i < n) {
            s.v = 0;
            return s
        };
        s
    }
}
