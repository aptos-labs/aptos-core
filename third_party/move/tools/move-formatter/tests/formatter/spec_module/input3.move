/// test_point: has {'requires'}
spec aptos_framework::aptos_coin {
    spec module {
        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    spec initialize {
        pragma aborts_if_is_partial;
        let addr = signer::address_of(aptos_framework);
        ensures exists<MintCapStore>(addr);
        ensures exists<coin::CoinInfo<AptosCoin>>(addr);
    }

    spec destroy_mint_cap {
        let addr = signer::address_of(aptos_framework);
        aborts_if addr != @aptos_framework;
        aborts_if !exists<MintCapStore>(@aptos_framework);
    }

spec schema ExistsAptosCoin {
    requires exists<coin::CoinInfo<AptosCoin>>(@aptos_framework);
}

}
