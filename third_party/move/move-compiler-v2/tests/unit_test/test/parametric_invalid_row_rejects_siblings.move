// One invalid row rejects the entire test function, including valid sibling rows.
address 0x1 {
module M {
    #[test(addr = @0x1)]
    #[test(typo = @0x2)]
    fun invalid_row_rejects_siblings(addr: signer) {
        let _ = addr;
    }
}
}
