#[test_only]
module 0x123::slow_coin {
    struct SlowCoin has key { }

    public fun initialize(account: &signer) {
        move_to(account, SlowCoin { });
    }
}

#[test_only]
module aptos_framework::fast_coin {
    struct FastCoin has key { }

    public fun initialize(account: &signer) {
        move_to(account, FastCoin { });
    }
}

#[test_only]
module aptos_framework::coin_supply_tests {
    use aptos_framework::aggregator_factory;
    use aptos_framework::fast_coin::{Self, FastCoin};
    use aptos_framework::coin_supply;
    use 0x123::slow_coin::{Self, SlowCoin};

    #[test(aptos_account = @aptos_framework, other_account = @0x123)]
    fun test_supply(aptos_account: signer, other_account: signer) {
        // Intitialize 2 coins, on aptos account and somewhere else.
        slow_coin::initialize(&other_account);
        fast_coin::initialize(&aptos_account);
        aggregator_factory::initialize_aggregator_factory(&aptos_account);

        // Coins from Aptos account should be parallelizable.
        let fast_supply = coin_supply::new<FastCoin>();
        // TODO: uncomment once excution is supported.
        // assert!(supply::is_parallelizable(&fast_supply), 0);

        coin_supply::add(&mut fast_supply, 100);
        coin_supply::sub(&mut fast_supply, 50);
        coin_supply::add(&mut fast_supply, 950);
        assert!(coin_supply::read(&fast_supply) == 1000, 0);

        // Coins from all other accounts shouldn't be parallelizable.
        let slow_supply = coin_supply::new<SlowCoin>();
        assert!(!coin_supply::is_parallelizable(&slow_supply), 0);

        coin_supply::add(&mut slow_supply, 100);
        coin_supply::sub(&mut slow_supply, 50);
        coin_supply::add(&mut slow_supply, 950);
        assert!(coin_supply::read(&slow_supply) == 1000, 0);

        // But if we upgrade, we should be able to get the parallelism.
        coin_supply::upgrade(&mut slow_supply);
        // TODO: uncomment once excution is supported.
        // assert!(coin_supply::is_parallelizable(&slow_supply), 0);

        coin_supply::add(&mut slow_supply, 100);
        coin_supply::sub(&mut slow_supply, 50);
        coin_supply::add(&mut slow_supply, 950);
        assert!(coin_supply::read(&slow_supply) == 2000, 0);

        coin_supply::drop_unchecked(fast_supply);
        coin_supply::drop_unchecked(slow_supply);
    }
}
