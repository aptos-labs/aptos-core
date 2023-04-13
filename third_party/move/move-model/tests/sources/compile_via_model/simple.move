module 0x1::simple {
    fun works_fine(x: u64): u64 { x + 1 }
    fun showing_sequential_not_supported_yet(x: u64): u64 { let x = x; x = x + 1; x }
}
