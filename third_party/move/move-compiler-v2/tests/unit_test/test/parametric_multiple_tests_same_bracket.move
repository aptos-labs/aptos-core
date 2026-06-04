// A row bracket contains exactly one #[test].
address 0x1 {
module M {
    #[test(addr = @0x1), test(addr = @0x2)]
    fun multiple_tests_same_bracket(addr: signer) {
        let _ = addr;
    }
}
}
