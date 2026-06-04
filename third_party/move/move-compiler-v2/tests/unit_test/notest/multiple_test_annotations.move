// Test-planner row validation is ignored when test code is disabled.
address 0x1 {
module M {
    #[test(_a=@0x1)]
    #[test(_b=@0x2)]
    public fun a(_a: signer, _b: signer) { }

    #[test]
    #[test(_a=@0x1, _b=@0x2)]
    public fun b(_a: signer, _b: signer) { }
}
}
