module 0x1::some_randapp {
    use aptos_std::randomness;

    entry fun safe_private_entry_call() {
        let _ = randomness::u64_integer();
    }

    public entry fun unsafe_public_entry_call() {
        let _ = randomness::u64_integer();
    }

    public(friend) entry fun safe_friend_entry_call() {
        let _ = randomness::u64_integer();
    }

    public fun unsafe_public_call() {
        let _ = randomness::u64_integer();
    }
}
