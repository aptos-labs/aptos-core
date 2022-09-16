spec aptos_framework::coin {
    spec mint {
        pragma opaque;
        let addr = spec_coin_address<CoinType>();
        modifies global<CoinInfo<CoinType>>(addr);
        aborts_if [abstract] false;
        ensures [abstract] result.value == amount;
    }

    spec coin_address {
        pragma opaque;
        ensures [abstract] result == spec_coin_address<CoinType>();
    }

    spec fun spec_coin_address<CoinType>(): address {
        // TODO: abstracted due to the lack of support for `type_info` in Prover.
        @0x0
    }
}
