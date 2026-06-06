// A parameterized function cannot use an empty #[test] row.
address 0x1 {
module M {
    #[test]
    fun empty_parameterized_row(addr: signer) {
        let _ = addr;
    }
}
}
