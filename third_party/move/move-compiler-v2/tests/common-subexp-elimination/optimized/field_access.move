module 0x99::FiledAccess {
    struct Inner has copy, drop {
        value: u64,
    }

    struct Outer has copy, drop {
        inner: Inner,
    }

    // `arg1.inner.value` can be reused
    // perf_gain: 2 field accesses + 1 readref eliminated
    // new_cost: `u64` flushed and copied twice
    fun test_field_access(arg1: Outer, arg2: u64): u64 {
        arg1.inner.value + arg2 + arg1.inner.value
    }
}
