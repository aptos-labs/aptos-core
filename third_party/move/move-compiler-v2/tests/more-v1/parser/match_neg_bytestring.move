module 0xc0ffee::m {
    fun neg_bytes(x: vector<u8>): vector<u8> {
        match (x) {
            -b"hello" => b"bye",
            _ => b"",
        }
    }
}
