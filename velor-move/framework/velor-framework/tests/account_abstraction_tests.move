#[test_only]
module velor_framework::account_abstraction_tests {
    use std::signer;
    use velor_framework::auth_data::AbstractionAuthData;
    use velor_framework::object;

    public fun invalid_authenticate(
        account: signer,
        _signing_data: AbstractionAuthData,
    ): signer {
        let addr = signer::address_of(&account);
        let cref = object::create_object(addr);
        object::generate_signer(&cref)
    }

    public fun test_auth(account: signer, _data: AbstractionAuthData): signer { account }
}
