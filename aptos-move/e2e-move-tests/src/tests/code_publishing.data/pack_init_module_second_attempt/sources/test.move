module 0xcafe::test {
    use supra_framework::coin::{Self, Coin};
    use supra_framework::supra_coin::SupraCoin;

    struct State has key {
        important_value: u64,
        coins: Coin<SupraCoin>,
    }

    fun init_module(s: &signer) {
        move_to(s, State {
            important_value: get_value(),
            coins: coin::zero<SupraCoin>(),
        })
    }

    fun get_value(): u64 {
        2
    }
}
