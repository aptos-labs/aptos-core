/// Stores a freshly generated key, so every run writes different bytes.
module fixtures::random_keys {
    use aptos_std::ed25519;

    struct Key has key {
        bytes: vector<u8>,
    }

    #[test(account = @fixtures)]
    fun stores_a_random_key(account: &signer) {
        let (_, public_key) = ed25519::generate_keys();
        move_to(account, Key { bytes: ed25519::validated_public_key_to_bytes(&public_key) });
    }
}
