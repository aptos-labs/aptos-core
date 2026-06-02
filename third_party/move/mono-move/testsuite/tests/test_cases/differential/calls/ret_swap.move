// RUN: publish --print(bytecode,stackless,micro-ops)
module 0xc0ffee::ret_swap {
    fun swap_back(a: u64, b: u64): (u64, u64) {
        (b, a)
    }

    public fun first(): u64 {
        let (x, _y) = swap_back(11, 22);
        x
    }

    public fun second(): u64 {
        let (_x, y) = swap_back(11, 22);
        y
    }

    public fun second_offset(): u64 {
        let (_x, y) = swap_back(11, 22);
        y - 5
    }
}

// RUN: execute 0xc0ffee::ret_swap::first
// CHECK: results: 22

// RUN: execute 0xc0ffee::ret_swap::second
// CHECK: results: 11

// RUN: execute 0xc0ffee::ret_swap::second_offset
// CHECK: results: 6
