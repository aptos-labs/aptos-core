module module_owner::dice {
    use std::signer::address_of;
    use aptos_framework::randomness;

    struct DiceRollResult has key {
        roll: u64,
    }

    #[randomness]
    entry fun roll(account: signer) acquires DiceRollResult {
        let addr = address_of(&account);
        let roll = randomness::u64_range(0, 6);
        if (exists<DiceRollResult>(addr)) {
            move_from<DiceRollResult>(addr);
        };
        move_to(&account, DiceRollResult { roll });
    }
}
