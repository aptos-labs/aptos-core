/// The chain id distinguishes between different chains (e.g., testnet and the main network).
/// One important role is to prevent transactions intended for one chain from being executed on another.
/// This code provides a container for storing a chain id and functions to initialize and get it.
module AptosFramework::ChainId {
    use AptosFramework::SystemAddresses;
    use AptosFramework::Timestamp;
    use Std::Errors;
    use Std::Signer;

    struct ChainId has key {
        id: u8
    }

    /// The `ChainId` resource was not in the required state
    const ECHAIN_ID: u64 = 0;

    /// Publish the chain ID `id` of this instance under the SystemAddresses address
    public fun initialize(account: &signer, id: u8) {
        Timestamp::assert_genesis();
        SystemAddresses::assert_core_resource(account);
        assert!(!exists<ChainId>(Signer::address_of(account)), Errors::already_published(ECHAIN_ID));
        move_to(account, ChainId { id })
    }

    /// Return the chain ID of this instance
    public fun get(): u8 acquires ChainId {
        Timestamp::assert_operating();
        borrow_global<ChainId>(@CoreResources).id
    }
}
