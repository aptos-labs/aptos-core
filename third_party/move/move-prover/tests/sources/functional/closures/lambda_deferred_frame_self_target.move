// A closure over the function being analysed has no footprint while that function's own
// summary is being computed, so a callee that leaves its frame to the call site cannot be
// handed one: the call must be rejected, not deferred with nothing havoced. Without the
// rejection, the claim that the loop leaves R unchanged would verify.
module 0x42::lambda_deferred_frame_self_target {
    struct R has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun f(flag: bool, z: u64): u64 acquires R {
        if (flag) {
            borrow_global_mut<R>(@0x42).v = z;
            0
        } else {
            let before = borrow_global<R>(@0x42).v;
            let t = true;
            let h: |u64| u64 has copy + drop = |y| f(t, y);
            let i = 0;
            while (i < 3) {
                call_g(h, before + 1);
                i = i + 1;
            };
            borrow_global<R>(@0x42).v - before
        }
    }
    spec f {
        pragma aborts_if_is_partial;
        ensures !flag ==> result == 0;
    }
}
