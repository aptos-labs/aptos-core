// Every function parameter must be assigned in a row.
address 0x1 {
module M {
    #[test(a = @0x1)]
    fun missing_argument(a: signer, b: address) {
        let _ = a;
        let _ = b;
    }
}
}
