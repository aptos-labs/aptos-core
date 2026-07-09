// Tests for retained inline-opaque functions called across module boundaries.
module 0x42::opaque_inline_cross_module_dep {
    public inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }
}
module 0x42::opaque_inline_cross_module {
    use 0x42::opaque_inline_cross_module_dep;

    fun test_cross_module(x: u64, c: u64): u64 {
        opaque_inline_cross_module_dep::apply(
            |y| y + c spec { ensures result == y + c; },
            x
        )
    }
    spec test_cross_module {
        ensures result == x + c;
    }
}
