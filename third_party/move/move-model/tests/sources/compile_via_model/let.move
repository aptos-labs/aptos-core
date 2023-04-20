module 0x1::let_ {
    fun works_fine(x: u64): u64 {
        let x = x + 1;
        x = x + 1;
        x
    }
}
