// A single-row test accepts a top-level legacy #[expected_failure] in a separate bracket.
address 0x1 {
module M {
    #[test(addr = @0x1)]
    #[expected_failure]
    fun single_row_legacy_expected_failure(addr: signer) {
        let _ = addr;
        abort 1
    }
}
}
