module 0xcafe::test_assert {
    use std::error;

    /// Some error that happens to have code 0.
    const E_SOME_ERROR: u64 = 0;

    public entry fun entry_assert(_s: &signer) {
        assert!(false);
    }

    public entry fun entry_error(_s: &signer) {
        abort E_SOME_ERROR
    }

    public entry fun entry_canonical_error(_s: &signer) {
        abort error::not_found(E_SOME_ERROR)
    }
}
