module module_owner::dice {
    use std::signer::address_of;
    use aptos_framework::randomness;

    struct DiceRollResult has drop, key {
        roll: u64,
    }

    #[randomness]
    entry fun roll(account: signer) {
        let _addr = address_of(&account);
        let _roll = randomness::u64_range(0, 6);
    }
}
