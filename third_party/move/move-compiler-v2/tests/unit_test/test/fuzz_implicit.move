// Bare #[test] on a function with parameters now treats the parameters as
// implicit fuzz inputs. With no FuzzValueSource registered the compiler reports
// a clear diagnostic instead of the old "Missing test parameter assignment".
module 0x1::M {
    #[test]
    public fun bare_with_signer(_a: signer) { }

    #[test]
    public fun bare_with_two(_a: signer, _b: address) { }

    // No parameters: no fuzz, no error.
    #[test]
    public fun bare_zero_args() { }
}
