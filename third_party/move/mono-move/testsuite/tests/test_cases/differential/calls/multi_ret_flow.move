// RUN: publish --print(bytecode,stackless,micro-ops)
module 0xc0ffee::multi_ret_flow {
    fun two_rets(): (u64, u64) {
        (10, 20)
    }

    fun two_args(a: u64, b: u64): u64 {
        a + b
    }

    public fun trigger(): u64 {
        let (a, b) = two_rets();
        two_args(a, b)
    }

    public fun trigger_swap(): u64 {
        let (a, b) = two_rets();
        two_args(b, a)
    }
}

// RUN: execute 0xc0ffee::multi_ret_flow::trigger
// CHECK: results: 30

// RUN: execute 0xc0ffee::multi_ret_flow::trigger_swap
// CHECK: results: 30
