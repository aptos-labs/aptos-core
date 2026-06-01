// An orphan #[expected_failure] on a multi-row function is rejected.
address 0x1 {
module M {
    #[test(addr = @0x1)]
    #[test(addr = @0x2)]
    #[expected_failure]
    fun multi_row_with_orphan(addr: signer) {
        let _ = addr;
    }
}
}
