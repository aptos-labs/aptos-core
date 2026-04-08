module publisher_address::dice_roll {
    use std::signer::address_of;
    use aptos_framework::randomness;

    struct DiceRollHistory has key {
        rolls: vector<u64>,
    }

    #[randomness]
    entry fun roll(account: signer) acquires DiceRollHistory {
        let addr = address_of(&account);
        let roll_history = if (exists<DiceRollHistory>(addr)) {
            move_from<DiceRollHistory>(addr)
        } else {
            DiceRollHistory { rolls: vector[] }
        };
        let new_roll = randomness::u64_range(0, 6);
        roll_history.rolls.push_back(new_roll);
        move_to(&account, roll_history);
    }
}
