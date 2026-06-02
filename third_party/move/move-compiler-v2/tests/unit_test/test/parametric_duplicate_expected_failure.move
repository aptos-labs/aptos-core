// Two #[expected_failure] attributes in the same bracket group are ambiguous.
address 0x1 {
module M {
    #[test(addr = @0x1), expected_failure, expected_failure(abort_code = 5)]
    #[test(addr = @0x2)]
    fun duplicate_expected_failure(addr: signer) {
        let _ = addr;
    }
}
}
