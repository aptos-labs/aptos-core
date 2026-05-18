module 0xc0ffee::m {
    // Non-exhaustive tuple match with byte strings: missing witness should display
    // using b"..." for valid UTF-8 byte arrays.
    fun non_exhaustive_tuple_utf8(x: vector<u8>, y: bool): u64 {
        match ((x, y)) {
            (b"hello", true) => 1,
        }
    }

    // Non-exhaustive tuple match with non-UTF-8 byte strings: missing witness should
    // display using x"..." hex encoding.
    fun non_exhaustive_tuple_hex(x: vector<u8>, y: bool): u64 {
        match ((x, y)) {
            (x"deadbeef", true) => 1,
        }
    }
}
