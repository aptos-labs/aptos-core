// RUN: publish --print(stackless,micro-ops)
module 0x42::closure_ret_transfer {
    public fun g(): (u64, u64) { (1, 2) }

    public fun h(v: u64): u64 { v + 40 }

    public fun h2(x: u64, y: u64): u64 { x * 100 + y }

    public fun live_ret_then_call(): (u64, u64) {
        let (a, b) = (g)();
        (a, h(b))
    }

    public fun ret_slot_overwrite(): (u64, u64) {
        let (a, b) = (g)();
        (a, h2(b, 7))
    }

    public fun pass_through(): u64 {
        let (a, b) = (g)();
        h2(a, b)
    }

    public fun consumed_before_next_call(): u64 {
        let (a, b) = (g)();
        a + b
    }
}

// RUN: execute 0x42::closure_ret_transfer::live_ret_then_call
// CHECK: results: 1, 42

// RUN: execute 0x42::closure_ret_transfer::ret_slot_overwrite
// CHECK: results: 1, 207

// RUN: execute 0x42::closure_ret_transfer::pass_through
// CHECK: results: 102

// RUN: execute 0x42::closure_ret_transfer::consumed_before_next_call
// CHECK: results: 3
