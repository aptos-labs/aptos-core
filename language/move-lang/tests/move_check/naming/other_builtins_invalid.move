module 0x8675309::M {
    fun foo(x: &mut u64) {
        freeze<u64, bool>(x);
        freeze<>(x);
    }
}
