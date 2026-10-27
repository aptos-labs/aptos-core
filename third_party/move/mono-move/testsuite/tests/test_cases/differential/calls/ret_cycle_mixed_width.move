// RUN: publish --print(stackless,micro-ops)
module 0x42::m {
    struct S has copy, drop { x: u16, y: bool }

    public fun make(_p: u16, _q: i8): S { S { x: 40358, y: false } }

    public fun observe(v: S): (u8, i8, u16, u16, bool) { (2, 0, 0, v.x, v.y) }

    public fun runner(): (u8, i8, u16, u16, bool) { (observe)((make)(10, 12)) }
}

// RUN: execute 0x42::m::runner
// CHECK: results: 2, 0, 0, 40358, false
