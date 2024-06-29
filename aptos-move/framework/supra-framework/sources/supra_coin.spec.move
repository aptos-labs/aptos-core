spec supra_framework::supra_coin {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec initialize(supra_framework: &signer): (BurnCapability<SupraCoin>, MintCapability<SupraCoin>) {
        use supra_framework::aggregator_factory;

        let addr = signer::address_of(supra_framework);
        aborts_if addr != @supra_framework;
        aborts_if !string::spec_internal_check_utf8(b"Supra Coin");
        aborts_if !string::spec_internal_check_utf8(b"SUP");
        aborts_if exists<MintCapStore>(addr);
        aborts_if exists<coin::CoinInfo<SupraCoin>>(addr);
        aborts_if !exists<aggregator_factory::AggregatorFactory>(addr);
        ensures exists<MintCapStore>(addr);
        ensures exists<coin::CoinInfo<SupraCoin>>(addr);
        ensures result_1 == BurnCapability<SupraCoin> {};
        ensures result_2 == MintCapability<SupraCoin> {};
    }

    spec destroy_mint_cap {
        let addr = signer::address_of(supra_framework);
        aborts_if addr != @supra_framework;
        aborts_if !exists<MintCapStore>(@supra_framework);
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

    spec schema ExistsSupraCoin {
        requires exists<coin::CoinInfo<SupraCoin>>(@supra_framework);
    }

}
