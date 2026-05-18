// Cannot mix `=` with `!=` / `in` on the same parameter.
module 0x1::M {
    #[test(_a = @0x1, _a != @0x2)]
    public fun mix_eq_then_ne(_a: signer) { }

    #[test(_a != @0x2, _a = @0x1)]
    public fun mix_ne_then_eq(_a: signer) { }
}
