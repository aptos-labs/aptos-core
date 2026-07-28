// separate_baseline: path
// Copyright © Aptos Foundation
// A data-invariant violation written through a conditional borrow that may
// target either a global resource or a local is reported: the dying reference
// holds the stored-back value on every path, so its invariant is asserted at
// the death site regardless of which lender was borrowed.
module 0x42::data_inv_cond_global {
    struct S has store { x: u64 }
    spec S { invariant x > 0; }
    struct T has key { s: S }

    fun violates(a: address, c: bool) acquires T {
        let tl = T { s: S { x: 1 } };
        let r = if (c) &mut T[a] else &mut tl;
        r.s.x = 0; // error: data invariant does not hold
        let T { s: S { x: _ } } = tl;
    }

    fun preserves(a: address, c: bool) acquires T {
        let tl = T { s: S { x: 1 } };
        let r = if (c) &mut T[a] else &mut tl;
        r.s.x = 7;
        let T { s: S { x: _ } } = tl;
    }
}
