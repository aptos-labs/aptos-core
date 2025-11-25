module 0x99::FiledAccess {
    struct Inner has copy, drop {
        value: u64,
    }

    struct Outer has copy, drop {
        inner: Inner,
    }

    // `arg1.inner` can be reused
    // perf_gain: removed 1 `borrow_loc` + 2 `borrow_field`
    // new_cost:
    // - `st_loc` to flush `arg1.inner`
    // - `copy_loc` of `arg1.inner` twice respectively for
    //    its original use and the new use
    fun test_field_access(arg1: Outer, arg2: u64, x: u64, y: u64): u64 {
        arg1.inner.value + arg1.inner.value
    }
}
