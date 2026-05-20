// RUN: publish --print(bytecode,stackless,micro-ops)
module 0xc0ffee::refs {
    fun add_refs(x: &u64, y: &u64): u64 {
        *x + *y
    }

    public fun add(x: u64, y: u64): u64 {
        add_refs(&x, &y)
    }
}

// RUN: execute 0xc0ffee::refs::add --args 0, 0
// CHECK: results: 0

// RUN: execute 0xc0ffee::refs::add --args 1, 2
// CHECK: results: 3

// RUN: execute 0xc0ffee::refs::add --args 100, 200
// CHECK: results: 300

// RUN: execute 0xc0ffee::refs::add --args 18446744073709551614, 1
// CHECK: results: 18446744073709551615
