// separate_baseline: path
// Copyright © Aptos Foundation
// In-code spec blocks observe the *current* value of a mutably borrowed lender
// while the borrow is live: at each observation the lender is temporarily synced
// to the borrow's current value and restored afterwards, guarded by per-site path
// flags. The tests pin: observation between two writes (with a later write, so
// the observed value is not the final one), observation after joins where either
// branch's borrow may reach, and non-vacuity of observations combined with
// assumes (the trailing must-fail asserts).
module 0x42::prophecy_observe_sync {
    struct P has drop { f: u64 }

    // The observation between the writes sees 5, not the final 6; the assume
    // does not corrupt the prophecy (which resolves to 6 at the borrow's death).
    fun observe_between_writes() {
        let s = P { f: 1 };
        let c = &mut s.f;
        *c = 5;
        spec {
            assert s.f == 5;
            assume s.f == 5;
        };
        *c = 6;
        let x = s.f;
        spec {
            assert x == 6;
            assert x == 100; // error: intended failure guards against vacuity
        };
    }

    // Both branches borrow the same local; the join's reaching borrow is
    // tracked by path flags, so the observation sees the branch's write and
    // the final write flows back after the observation.
    fun observe_after_join(c: bool) {
        let x = 1;
        let r;
        if (c) {
            r = &mut x;
            *r = 2;
        } else {
            r = &mut x;
            *r = 3;
        };
        spec {
            assert c ==> x == 2;
            assert !c ==> x == 3;
        };
        *r = 4;
        spec {
            assert x == 4;
            assert x == 5; // error: intended failure guards against vacuity
        };
    }
}
