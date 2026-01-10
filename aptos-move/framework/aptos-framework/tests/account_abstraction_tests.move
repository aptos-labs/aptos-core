#[test_only]
module aptos_framework::account_abstraction_tests {
    use std::signer;
    use aptos_framework::auth_data::AbstractionAuthData;
    use aptos_framework::object;

    public fun invalid_authenticate(
        account: signer,
        _signing_data: AbstractionAuthData,
    ): signer {
        let addr = signer::address_of(&account);
        let cref = object::create_object(addr);
        cref.generate_signer()
    }

    public fun test_auth(account: signer, _data: AbstractionAuthData): signer { account }
}
