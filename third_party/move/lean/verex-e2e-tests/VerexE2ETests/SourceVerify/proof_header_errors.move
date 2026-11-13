/// A proof file's header names its own Move file.
module 0x42::proof_header_errors {
    fun double(a: u64): u64 {
        a + a
    }
    spec double {
        aborts_if a + a > MAX_U64;
        ensures result == 2 * a;
    }
}
