module module_owner::dice {
    use std::signer::address_of;
    use std::vector;
    use velor_framework::randomness;

    struct DiceRollHistory has key {
        rolls: vector<u64>,
    }

    #[lint::allow_unsafe_randomness]
    public entry fun roll_v0(_account: signer) {
        let _ = randomness::u64_range(0, 6);
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
        vector::push_back(&mut roll_history.rolls, new_roll);
        move_to(&account, roll_history);
    }

    #[randomness(max_gas=56789)]
    entry fun roll_v2(_account: signer) {
        let _ = randomness::u64_range(0, 6);
    }
}
