//# publish
module 0xc0ffee::m {
    // Path 2: guarded catch-all on a scalar.
    // The first arm binds `y = x`, but the guard is `false`, so we fall
    // through.  The catch-all `_ => y` must see the *parameter* y, not
    // the pattern-bound y from the failed arm.
    public fun scalar_catch_all(x: u8, y: u8): u8 {
        match (x) {
            y if false => y,
            _ => y,
        }
    }

    // Path 3: guarded non-catch-all tuple with vars.
    // `(y, 1)` binds y = tuple[0] = x.  Guard is always false, so we
    // fall through to `_ => y` which must use the parameter y.
    public fun tuple_with_vars(x: u64, y: u64): u64 {
        match ((x, y)) {
            (y, 1) if false => y,
            _ => y,
        }
    }

    // Multiple guarded arms in sequence: each arm's bindings must be
    // independent and not leak into later arms.
    public fun chained_guards(x: u64, y: u64): u64 {
        match (x) {
            y if (y > 1000) => y,       // binds y = x; guard fails for small x
            y if (y > 500) => y + 1,    // binds y = x; guard fails for small x
            _ => y,                      // must see parameter y
        }
    }
}

//# run 0xc0ffee::m::scalar_catch_all --args 1u8 2u8

//# run 0xc0ffee::m::scalar_catch_all --args 5u8 10u8

//# run 0xc0ffee::m::tuple_with_vars --args 10u64 20u64

//# run 0xc0ffee::m::tuple_with_vars --args 99u64 1u64

//# run 0xc0ffee::m::chained_guards --args 10u64 42u64

//# run 0xc0ffee::m::chained_guards --args 2000u64 42u64
