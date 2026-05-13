module publisher_address::dice_roll {
    use std::signer::address_of;
    use aptos_framework::randomness;

    struct LastRoll has key {
        value: u64,
    }

    #[randomness]
    entry fun roll(account: signer) acquires LastRoll {
        let addr = address_of(&account);
        let new_roll = randomness::u64_range(0, 6);
        if (exists<LastRoll>(addr)) {
            borrow_global_mut<LastRoll>(addr).value = new_roll;
        } else {
            move_to(&account, LastRoll { value: new_roll });
        }
    }
}
