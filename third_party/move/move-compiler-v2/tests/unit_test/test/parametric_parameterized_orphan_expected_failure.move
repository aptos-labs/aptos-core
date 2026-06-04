// A multi-row test rejects a parameterized top-level #[expected_failure].
address 0x1 {
module M {
    #[test(addr = @0x1)]
    #[test(addr = @0x2)]
    #[expected_failure(abort_code = 5, location = 0x1::M)]
    fun parameterized_orphan_expected_failure(addr: signer) {
        let _ = addr;
    }
}
}
