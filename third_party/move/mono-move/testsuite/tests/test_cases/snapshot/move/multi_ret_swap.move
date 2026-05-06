// Multi-ret destructuring fed back into a multi-arg call, once in
// order (`two_args(a, b)`) and once swapped (`two_args(b, a)`).
// Snapshot pins the lowered micro-ops for both shapes.
module 0xc0ffee::multi_ret_swap {
    fun two_rets(): (u64, u64) {
        (10, 20)
    }

    fun two_args(_a: u64, _b: u64): u64 {
        0
    }

    fun in_order(): u64 {
        let (a, b) = two_rets();
        two_args(a, b)
    }

    fun swapped(): u64 {
        let (a, b) = two_rets();
        two_args(b, a)
    }
}
