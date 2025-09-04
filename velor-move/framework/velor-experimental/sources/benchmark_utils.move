module velor_experimental::benchmark_utils {
    use velor_framework::account;
    use velor_framework::velor_account;

    /// Entry function that creates account resource, and funds the account.
    /// This makes sure that transactions later don't need to create an account,
    /// and so actual costs of entry functions can be more precisely measured.
    entry fun transfer_and_create_account(
        source: &signer, to: address, amount: u64
    ) {
        account::create_account_if_does_not_exist(to);
        velor_account::transfer(source, to, amount);
    }
}
