/// A proof file holds proofs: an item of the module there is rejected.
module 0x42::proof_items_errors {
    fun double(a: u64): u64 {
        a + a
    }
    spec double {
        aborts_if a + a > MAX_U64;
        ensures result == 2 * a;
    }
}
