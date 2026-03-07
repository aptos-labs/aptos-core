#[test_only]
/// Shared test utilities for sigma protocol proof tests.
module aptos_experimental::sigma_protocol_test_utils {
    use std::signer;
    use aptos_framework::account;
    use aptos_framework::chain_id;
    use aptos_framework::fungible_asset::{Self, Metadata};
    use aptos_framework::object::Object;

    /// Creates a test signer, initializes chain_id, and creates a fungible asset.
    /// WARNING: Can only be called once per test because it calls `create_fungible_asset`!
    public fun setup_test_environment(): (signer, Object<Metadata>) {
        let sender = account::create_signer_for_test(@0x1);
        chain_id::initialize_for_test(&sender, 4);
        let (_, _, _, _, asset_type) = fungible_asset::create_fungible_asset(&sender);
        (sender, asset_type)
    }

    /// Returns the address of a signer.
    public fun addr(s: &signer): address {
        signer::address_of(s)
    }
}
