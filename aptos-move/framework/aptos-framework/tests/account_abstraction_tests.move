#[test_only]
module aptos_framework::account_abstraction_tests {
    use aptos_framework::auth_data::AbstractionAuthData;

    public fun test_auth(account: signer, _data: AbstractionAuthData): signer { account }
}
