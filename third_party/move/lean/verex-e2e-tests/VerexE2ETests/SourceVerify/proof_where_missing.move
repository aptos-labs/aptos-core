/// A proof file's items follow `verex proofs where`.
module 0x42::proof_where_missing {
    fun double(a: u64): u64 {
        a + a
    }
    spec double {
        aborts_if a + a > MAX_U64;
        ensures result == 2 * a;
    }
}
