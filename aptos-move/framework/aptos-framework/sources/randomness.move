/// On-chain randomness utils.
module aptos_std::randomness {
    use aptos_framework::system_addresses;

    friend aptos_framework::block;

    /// The block-level randomness seed.
    /// It's updated at the beginning of every block.
    struct BlockRandomness has drop, key {
        block_randomness: vector<u8>,
    }

    /// Invoked in `block_prologue_ext()` to update the block-level seed randomness.
    public(friend) fun on_new_block(account: &signer, randomness_available: bool, block_randomness: vector<u8>) acquires BlockRandomness {
        system_addresses::assert_aptos_framework(account);
        if (exists<BlockRandomness>(@aptos_framework)) {
            move_from<BlockRandomness>(@aptos_framework);
        };
        if (randomness_available) {
            move_to(account, BlockRandomness { block_randomness })
        };
    }
}
