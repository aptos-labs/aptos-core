module 0xc0ffee::m {
    // Non-exhaustive: byte string match without wildcard should error
    fun non_exhaustive_bytestring(x: vector<u8>): u64 {
        match (x) {
            b"hello" => 1,
            b"world" => 2,
        }
    }

    // Duplicate byte string pattern should be unreachable
    fun duplicate_bytestring(x: vector<u8>): u64 {
        match (x) {
            b"hello" => 1,
            b"hello" => 2,
            _ => 3,
        }
    }

    // Exhaustive match (byte strings + wildcard) should have no error
    fun exhaustive_bytestring(x: vector<u8>): u64 {
        match (x) {
            b"hello" => 1,
            x"deadbeef" => 2,
            _ => 3,
        }
    }
}
