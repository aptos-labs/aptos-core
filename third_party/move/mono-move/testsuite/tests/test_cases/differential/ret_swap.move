// RUN: publish
module 0xc0ffee::ret_swap {
    fun swap_back(a: u64, b: u64): (u64, u64) {
        (b, a)
    }

    public fun first(): u64 {
        let (x, _y) = swap_back(11, 22);
        x // expects 22
    }

    public fun second(): u64 {
        let (_x, y) = swap_back(11, 22);
        y // expects 11
    }

    public fun second_offset(): u64 {
        // y - 5 isolates the second ret slot. With cycle handling
        // working, y = 11 → 6. Without it, y might equal x = 22 → 17.
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
