module 0xcafe::test {
    use velor_framework::coin::{Self, Coin};
    use velor_framework::velor_coin::VelorCoin;
    use std::signer::address_of;

    struct State has key {
        important_value: u64,
        coins: Coin<VelorCoin>,
    }

    fun init_module(s: &signer) {
        // Transfer away all the APT from s so there's nothing left to pay for gas.
        // This makes this init_module function fail for sure.
        let balance = coin::balance<VelorCoin>(address_of(s));
        let coins = coin::withdraw<VelorCoin>(s, balance);

        move_to(s, State {
            important_value: get_value(),
            coins,
        })
    }

    fun get_value(): u64 {
        1
    }
}
