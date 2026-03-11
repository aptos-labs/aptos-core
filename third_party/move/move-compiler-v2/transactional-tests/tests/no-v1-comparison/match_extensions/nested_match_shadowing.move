//# publish
module 0xc0ffee::m {
    // Outer match binds y = x.  Inner match (in body) also binds y.
    // When the inner guard fails, inner catch-all must see the *outer* arm's y (= x).
    // When the outer guard fails, outer catch-all must see the *parameter* y.
    public fun nested_body(x: u64, y: u64): u64 {
        match (x) {
            y if (y > 10) => match (y) {
                y if (y > 100) => y + 2000,
                _ => y + 100,
            },
            _ => y,
        }
    }

    // The outer guard itself contains a nested match that binds y.
    // The inner match's y binding must not leak into the outer catch-all.
    public fun nested_guard(x: u64, y: u64): u64 {
        match (x) {
            y if (match (y) {
                y if (y > 10) => true,
                _ => false,
            }) => y + 1000,
            _ => y,
        }
    }

    // Triple nesting: same variable y rebound at every level.
    // Each level's catch-all must see the y from the enclosing arm,
    // and the outermost catch-all must see the parameter y.
    public fun triple_nested(x: u64, y: u64): u64 {
        match (x) {
            y if (y > 5) => match (y) {
                y if (y > 50) => match (y) {
                    y if (y > 500) => y + 3000,
                    _ => y + 2000,
                },
                _ => y + 1000,
            },
            _ => y,
        }
    }
}

// --- nested_body tests ---
// x=50, y=7: outer guard passes (50>10), inner guard fails (50<=100) => 50+100 = 150
//# run 0xc0ffee::m::nested_body --args 50u64 7u64

// x=200, y=7: both guards pass (200>10, 200>100) => 200+2000 = 2200
//# run 0xc0ffee::m::nested_body --args 200u64 7u64

// x=5, y=7: outer guard fails (5<=10), catch-all => parameter y = 7
//# run 0xc0ffee::m::nested_body --args 5u64 7u64

// --- nested_guard tests ---
// x=50, y=7: inner match in guard: 50>10 => true, outer body: 50+1000 = 1050
//# run 0xc0ffee::m::nested_guard --args 50u64 7u64

// x=5, y=7: inner match in guard: 5<=10 => false, outer catch-all => parameter y = 7
//# run 0xc0ffee::m::nested_guard --args 5u64 7u64

// --- triple_nested tests ---
// x=3, y=42: outermost guard fails => parameter y = 42
//# run 0xc0ffee::m::triple_nested --args 3u64 42u64

// x=10, y=42: level1 passes, level2 fails => 10+1000 = 1010
//# run 0xc0ffee::m::triple_nested --args 10u64 42u64

// x=100, y=42: level1+2 pass, level3 fails => 100+2000 = 2100
//# run 0xc0ffee::m::triple_nested --args 100u64 42u64

// x=1000, y=42: all pass => 1000+3000 = 4000
//# run 0xc0ffee::m::triple_nested --args 1000u64 42u64
