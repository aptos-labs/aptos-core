// Single-row tests must keep their un-suffixed name (no #0).
address 0x1 {
module M {
    #[test(addr = @0x1)]
    fun single_row_test(addr: signer) {
        let _ = addr;
    }
}
}
