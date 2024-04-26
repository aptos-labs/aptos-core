module module_owner::dice {
    use std::signer::address_of;
    use std::string::utf8;
    use std::vector;
    use aptos_std::debug;
    use aptos_framework::randomness;

    struct DiceRollHistory has key {
        rolls: vector<u64>,
    }

    #[randomness]
    entry fun roll(account: signer) acquires DiceRollHistory {
        debug::print(&utf8(b"0425 - a"));
        let addr = address_of(&account);
        let roll_history = if (exists<DiceRollHistory>(addr)) {
            move_from<DiceRollHistory>(addr)
        } else {
            DiceRollHistory { rolls: vector[] }
        };
        debug::print(&utf8(b"0425 - b"));
        let new_roll = randomness::u64_range(0, 6);
        vector::push_back(&mut roll_history.rolls, new_roll);
        move_to(&account, roll_history);
        debug::print(&utf8(b"0425 - c"));
    }
}
