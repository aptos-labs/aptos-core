module aa::test_functions {
    use velor_framework::velor_account;

    /// test function for multi-agent aa.
    public entry fun transfer_to_the_last(a: &signer, b: &signer, c: &signer, d: address) {
        velor_account::transfer(a, d, 1);
        velor_account::transfer(b, d, 1);
        velor_account::transfer(c, d, 1);
    }
}
