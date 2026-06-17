// Copyright © Aptos Foundation
// An in-code spec assertion reading a mutably borrowed global observes the
// borrow's current value even when the local that supplied the borrow address
// is reassigned while the borrow is live.
module 0x42::spec_read_reassigned_addr {
    struct R has key { v: u64 }

    fun write_then_assert(c: bool, a: address, b: address): u64 acquires R {
        let addr = if (c) a else b;
        let r = &mut R[addr];
        addr = if (c) b else a;
        r.v = 5;
        spec {
            assert c ==> R[a].v == 5;
            assert !c ==> R[b].v == 5;
        };
        let v = r.v;
        assert!(addr == a || addr == b, 0);
        v
    }
    spec write_then_assert {
        requires exists<R>(a) && exists<R>(b);
        aborts_if false;
        ensures result == 5;
        ensures c ==> R[a].v == 5;
        ensures !c ==> R[b].v == 5;
    }
}
