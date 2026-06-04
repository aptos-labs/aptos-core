// #[test_only] is not allowed inside a row bracket.
address 0x1 {
module M {
    #[test(addr = @0x1), test_only]
    fun test_only_row_sibling(addr: signer) {
        let _ = addr;
    }
}
}
