module publisher::test {
    use aptos_framework::coin::{Self, Coin};
    use aptos_framework::aptos_coin::AptosCoin;

    struct State has key {
        important_value: u64,
        coins: Coin<AptosCoin>,
    }

    fun init_module(s: &signer) {
        move_to(s, State {
            important_value: get_value(),
            coins: coin::zero<AptosCoin>(),
        })
    }

    fun get_value(): u64 {
        2
    }
}
