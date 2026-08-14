// separate_baseline: path
// Copyright © Aptos Foundation
// A struct declaring its own data invariant that is mutated through a
// conditional borrow of a global resource sits *above* the borrow fork: there
// is no path-independent reference to assert the invariant on, so the commit
// is rejected under the prophecy model. The path model verifies this file and
// reports the actual violation.
module 0x42::data_inv_cond_container {
    struct Inner has store { x: u64 }

    struct R1 has key { i: Inner }
    spec R1 {
        invariant i.x > 0;
    }

    struct R2 has key { i: Inner }
    spec R2 {
        invariant i.x > 0;
    }

    fun violates(a: address, c: bool) acquires R1, R2 {
        let r = if (c) &mut R1[a].i else &mut R2[a].i;
        r.x = 0; // error: rejected — invariant above the borrow fork
    }
}
