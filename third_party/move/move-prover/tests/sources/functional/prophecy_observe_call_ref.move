// separate_baseline: path
// Copyright © Aptos Foundation
// A `&mut` materialized as a function-call result has no statically known
// structure, so no sync site exists for it: observing one of its lenders while
// it is live is rejected under the prophecy model (the observation would read
// the installed prophecy). The path model verifies this file.
module 0x42::prophecy_observe_call_ref {
    struct S has drop { a: u64, b: u64 }

    fun get_a(s: &mut S): &mut u64 {
        &mut s.a
    }

    fun observe_lender(): u64 {
        let s = S { a: 1, b: 2 };
        let r = get_a(&mut s);
        *r = 5;
        spec {
            assert s.a == 5; // error: cannot observe under a call-derived borrow
        };
        *r = 6;
        s.a
    }
}
