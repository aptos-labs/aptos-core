spec aptos_framework::aptos_coin {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec initialize(aptos_framework: &signer): (BurnCapability<AptosCoin>, MintCapability<AptosCoin>) {
        use aptos_framework::aggregator_factory;

        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if !string::spec_internal_check_utf8(b"Aptos Coin");
        aborts_if !string::spec_internal_check_utf8(b"APT");
        aborts_if exists<MintCapStore>(addr);
        aborts_if exists<coin::CoinInfo<AptosCoin>>(addr);
        aborts_if !exists<aggregator_factory::AggregatorFactory>(addr);
        ensures exists<MintCapStore>(addr);
        ensures exists<coin::CoinInfo<AptosCoin>>(addr);
        ensures result_1 == BurnCapability<AptosCoin> {};
        ensures result_2 == MintCapability<AptosCoin> {};
    }

    spec destroy_mint_cap {
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if !exists<MintCapStore>(@aptos_framework);
    }

    // Test function, not needed verify.
    spec configure_accounts_for_test {
        pragma verify = false;
    }

    // Only callable in tests and testnets. not needed verify.
    spec mint(
        account: &signer,
        dst_addr: address,
        amount: u64,
    ) {
        pragma verify = false;
    }

    // Only callable in tests and testnets. not needed verify.
    spec delegate_mint_capability {
        pragma verify = false;
    }

    // Only callable in tests and testnets. not needed verify.
    spec claim_mint_capability(account: &signer) {
        pragma verify = false;
    }

    spec find_delegation(addr: address): Option<u64> {
        aborts_if !exists<Delegations>(@core_resources);
    }

    spec schema ExistsAptosCoin {
        requires exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
    }

}
