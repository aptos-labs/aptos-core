module 0x0::test_utils {

    // Intrinsic compiled to instruction that unconditionally triggers GC. No-op for old VM.
    public fun forge_gc() {}
}
