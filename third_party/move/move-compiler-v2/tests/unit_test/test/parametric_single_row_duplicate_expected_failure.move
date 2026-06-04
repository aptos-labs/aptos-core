// A single-row test accepts at most one #[expected_failure] anywhere.
address 0x1 {
module M {
    #[test(addr = @0x1), expected_failure]
    #[expected_failure]
    fun single_row_duplicate_expected_failure(addr: signer) {
        let _ = addr;
    }
}
}
