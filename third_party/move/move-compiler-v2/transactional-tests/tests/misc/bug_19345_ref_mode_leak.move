//# publish
module 0xc0ffee::m {
    struct Outer has copy, drop { inner: Inner }
    struct Inner has copy, drop { x: u64 }

    fun test_if_else_select(cond: bool, o1: Outer, o2: Outer): u64 {
        (if (cond) { o1.inner } else { o2.inner }).x
    }

    fun test_block_select(o: Outer): u64 {
        ({ o.inner }).x
    }

    fun test_block_with_side_effect(o: Outer): u64 {
        ({ let _y = 1; o.inner }).x
    }

    fun test_mutate_if_else(cond: bool, o1: Outer, o2: Outer): Outer {
        let result = if (cond) { o1 } else { o2 };
        result.inner.x = 42;
        result
    }

    public fun main() {
        let o1 = Outer { inner: Inner { x: 10 } };
        let o2 = Outer { inner: Inner { x: 20 } };

        assert!(test_if_else_select(true, o1, o2) == 10, 0);
        assert!(test_if_else_select(false, o1, o2) == 20, 1);
        assert!(test_block_select(o1) == 10, 2);
        assert!(test_block_with_side_effect(o1) == 10, 3);

        let result = test_mutate_if_else(true, o1, o2);
        assert!(result.inner.x == 42, 4);
    }
}

//# run 0xc0ffee::m::main
