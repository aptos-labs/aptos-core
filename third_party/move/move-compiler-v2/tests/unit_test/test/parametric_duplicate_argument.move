// Every function parameter is assigned exactly once in a row.
address 0x1 {
module M {
    #[test(addr = @0x1, addr = @0x2)]
    fun duplicate_argument(addr: signer) {
        let _ = addr;
    }
}
}
