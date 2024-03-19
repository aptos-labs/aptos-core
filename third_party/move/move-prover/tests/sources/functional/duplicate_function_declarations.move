module 0x42::DuplicateFunction {
    struct R0 { x: u8 }

    spec R0 {
        fun double(x: u8): u8 {
            x * 2
        }
        invariant x > 0;
    }

    fun double(x: u8): u8 {
        x
    }

    spec double (x: u8) : u8 {}
}
