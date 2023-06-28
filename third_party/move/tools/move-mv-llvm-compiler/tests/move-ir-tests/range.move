module 0x42::M {
    spec module {
    fun some_range(upper: u64): range {
        0..upper
    }
    }
}
