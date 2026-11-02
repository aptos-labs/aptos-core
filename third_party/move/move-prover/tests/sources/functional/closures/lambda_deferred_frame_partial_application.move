// A closure that partially applies a callee which itself leaves its frame to the call site
// (`|y| call_g(h, y)`, lowered to a closure over `call_g` capturing `h`) has no footprint:
// `call_g`'s summary records nothing about the `h` it runs. Handing it to another deferring
// callee must be rejected; deferring it would havoc nothing and let the claim that R is
// unchanged verify.
module 0x42::lambda_deferred_frame_partial_application {
    struct R has key { v: u64 }

    fun call_g(g: |u64| u64 has copy + drop, y: u64): u64 {
        g(y)
    }
    spec call_g {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun apply_k(k: |u64| u64 has copy + drop, y: u64): u64 {
        k(y)
    }
    spec apply_k {
        pragma opaque;
        pragma verify = false;
        aborts_if false;
    }

    fun writer(y: u64): u64 acquires R {
        borrow_global_mut<R>(@0x42).v = y;
        y
    }
    spec writer {
        pragma opaque;
        modifies global<R>(@0x42);
        aborts_if !exists<R>(@0x42);
        ensures global<R>(@0x42).v == y;
        ensures result == y;
    }

    fun partially_applied(): u64 acquires R {
        let before = borrow_global<R>(@0x42).v;
        let h: |u64| u64 has copy + drop = |z| writer(z);
        let k: |u64| u64 has copy + drop = |y| call_g(h, y);
        apply_k(k, before + 1);
        borrow_global<R>(@0x42).v
    }
    spec partially_applied {
        pragma aborts_if_is_partial;
        ensures result == old(global<R>(@0x42).v);
    }
}
