module 0xcafe::test {
    use velor_framework::coin::{Self, Coin};
    use velor_framework::velor_coin::VelorCoin;

    struct State has key {
        important_value: u64,
        coins: Coin<VelorCoin>,
    }

    fun init_module(s: &signer) {
        move_to(s, State {
            important_value: get_value(),
            coins: coin::zero<VelorCoin>(),
        })
    }

    fun get_value(): u64 {
        2
    }
}
