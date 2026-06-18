// RUN: publish --print(stackless,micro-ops)
module 0x42::generic_closure_pack {
    fun identity<T>(v: T): T {
        v
    }

    fun pick<T: drop>(captured: T, replacement: T, take_captured: bool): T {
        if (take_captured) { captured } else { replacement }
    }

    // Non-capturing closure over a generic target, packed in a non-generic
    // function.
    fun call_identity(v: u64): u64 {
        let f: |u64|u64 has drop = |x| identity(x);
        f(v)
    }

    // Capturing closure over a T-capturing generic target, packed inside a
    // generic function.
    fun make_pick<T: drop>(captured: T): |T, bool|T has drop {
        |replacement, take_captured| pick(captured, replacement, take_captured)
    }

    fun call_pick(v: u64): u64 {
        let take = make_pick(v);
        let leave = make_pick(v);
        take(v + 1, true) * 1000 + leave(v + 1, false)
    }
}

// RUN: execute 0x42::generic_closure_pack::call_identity --args 314
// CHECK: results: 314

// RUN: execute 0x42::generic_closure_pack::call_pick --args 5
// CHECK: results: 5006
