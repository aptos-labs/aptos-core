// Multiple #[test] attributes are accepted as parametric rows.
// Two parametric rows produce two TestCases named `<function>@rowN`.
address 0x1 {
module M {
    #[test(_a=@0x1, _b=@0x2)]
    #[test(_a=@0x3, _b=@0x4)]
    public fun a(_a: signer, _b: signer) { }

    #[test(_a=@0x1, _b=@0x2)]
    #[test(_a=@0x3, _b=@0x4)]
    public fun b(_a: signer, _b: signer) { }
}
}
