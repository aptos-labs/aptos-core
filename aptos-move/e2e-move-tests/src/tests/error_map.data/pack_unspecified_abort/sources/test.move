module 0xcafe::test_unspecified_abort {
    use std::error;

    /// User-defined error that happens to use code 0.
    const E_ZERO: u64 = 0;

    /// User-defined error with a non-zero code.
    const E_NONZERO: u64 = 7;

    /// Triggers `UNSPECIFIED_ABORT_CODE` via single-argument `assert!`.
    public entry fun unspecified_abort(_s: &signer) {
        assert!(false);
    }

    /// Aborts directly with the user-defined error code zero.
    public entry fun abort_with_user_zero(_s: &signer) {
        abort E_ZERO
    }

    /// Aborts with a canonical `std::error` code whose reason is 0.
    public entry fun abort_with_canonical_zero(_s: &signer) {
        abort error::not_found(E_ZERO)
    }

    /// Aborts directly with a non-zero user-defined error code.
    public entry fun abort_with_user_nonzero(_s: &signer) {
        abort E_NONZERO
    }

    /// Aborts with a canonical `std::error` code whose reason is non-zero.
    public entry fun abort_with_canonical_nonzero(_s: &signer) {
        abort error::not_found(E_NONZERO)
    }
}
