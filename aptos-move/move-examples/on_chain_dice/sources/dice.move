module module_owner::dice {
    use std::signer::address_of;
    // use aptos_framework::randomness;

    struct DiceRollHistory has drop, key {
        last_roll: u64,
    }

    entry fun roll(account: signer) acquires DiceRollHistory {
        let addr = address_of(&account);
        if (exists<DiceRollHistory>(addr)) {
            move_from<DiceRollHistory>(addr);
        };
        // let new_roll = randomness::u64_range(0, 6);
        let new_roll = 6;
        move_to(&account, DiceRollHistory { last_roll: new_roll } );
    }
}
