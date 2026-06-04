// Every row assignment names a function parameter.
address 0x1 {
module M {
    #[test(real = @0x1, typo = @0x2)]
    fun unknown_argument(real: signer) {
        let _ = real;
    }
}
}
