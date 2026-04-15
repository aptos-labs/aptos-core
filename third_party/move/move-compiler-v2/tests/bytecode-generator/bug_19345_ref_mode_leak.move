module 0xc0ffee::m {
    struct Outer has copy, drop { inner: Inner }
    struct Inner has copy, drop { x: u64 }

    // IfElse wrapping Select - the main #19345 case
    fun test_if_else_select(cond: bool, o1: Outer, o2: Outer): u64 {
        (if (cond) { o1.inner } else { o2.inner }).x
    }

    // Block wrapping Select
    fun test_block_select(o: Outer): u64 {
        ({ o.inner }).x
    }

    // Block with side-effect wrapping Select
    fun test_block_with_side_effect(o: Outer): u64 {
        ({ let _y = 1; o.inner }).x
    }

    // Mutation through IfElse operand
    fun test_mutate_if_else(cond: bool, o1: Outer, o2: Outer): Outer {
        let result = if (cond) { o1 } else { o2 };
        result.inner.x = 42;
        result
    }
}
