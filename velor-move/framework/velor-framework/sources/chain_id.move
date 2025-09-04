/// The chain id distinguishes between different chains (e.g., testnet and the main network).
/// One important role is to prevent transactions intended for one chain from being executed on another.
/// This code provides a container for storing a chain id and functions to initialize and get it.
module velor_framework::chain_id {
    use velor_framework::system_addresses;

    friend velor_framework::genesis;

    struct ChainId has key {
        id: u8
    }

    /// Only called during genesis.
    /// Publish the chain ID `id` of this instance under the SystemAddresses address
    public(friend) fun initialize(velor_framework: &signer, id: u8) {
        system_addresses::assert_velor_framework(velor_framework);
        move_to(velor_framework, ChainId { id })
    }

    #[view]
    /// Return the chain ID of this instance.
    public fun get(): u8 acquires ChainId {
        borrow_global<ChainId>(@velor_framework).id
    }

    #[test_only]
    use std::signer;

    #[test_only]
    public fun initialize_for_test(velor_framework: &signer, id: u8) {
        if (!exists<ChainId>(signer::address_of(velor_framework))) {
            initialize(velor_framework, id);
        }
    }

    #[test(velor_framework = @0x1)]
    fun test_get(velor_framework: &signer) acquires ChainId {
        initialize_for_test(velor_framework, 1u8);
        assert!(get() == 1u8, 1);
    }
}
