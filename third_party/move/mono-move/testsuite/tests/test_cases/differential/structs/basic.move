// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x42::basic {
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    public fun add(a: u64, b: u64): u64 {
        a + b
    }

    fun make_point(x: u64, y: u64): Point {
        Point { x, y }
    }

    fun get_x(p: &Point): u64 {
        p.x
    }

    public fun max(a: u64, b: u64): u64 {
        if (a > b) { a } else { b }
    }

    public fun get_x_of(x: u64, y: u64): u64 {
        get_x(&make_point(x, y))
    }
}

// RUN: execute 0x42::basic::add --args 3, 4
// CHECK: results: 7

// RUN: execute 0x42::basic::add --args 0, 0
// CHECK: results: 0

// RUN: execute 0x42::basic::max --args 5, 9
// CHECK: results: 9

// RUN: execute 0x42::basic::max --args 9, 5
// CHECK: results: 9

// RUN: execute 0x42::basic::max --args 7, 7
// CHECK: results: 7

// RUN: execute 0x42::basic::get_x_of --args 11, 22
// CHECK: results: 11

// RUN: execute 0x42::basic::get_x_of --args 0, 0
// CHECK: results: 0
