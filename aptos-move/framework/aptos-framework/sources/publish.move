/// Module for publishing packages to resource accounts.
module aptos_framework::publish {
    use std::signer;
    use aptos_framework::account::{Self, SignerCapability};

    public entry fun publish_package_txn(
        deployer: &signer,
        seed: vector<u8>,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        let (resource, resource_signer_cap) = account::create_resource_account(deployer, seed);
        aptos_framework::code::publish_package_txn(&resource, metadata_serialized, code);
        store_signer_cap(resource_signer_cap);
    }

    /// Holds the SignerCapability.
    /// This can only ever be held by the resource account itself.
    struct SignerCapabilityStore has key, drop {
        /// The SignerCapability of the resource.
        resource_signer_cap: SignerCapability
    }

    /// Stores the [SignerCapability].
    public fun store_signer_cap(
        resource_signer_cap: SignerCapability,
    ) {
        let resource = account::create_signer_with_capability(&resource_signer_cap);
        move_to(&resource, SignerCapabilityStore { resource_signer_cap });
    }

    /// When called by the resource account, it will retrieve the capability associated with that
    /// account and rotate the account's auth key to 0x0 making the account inaccessible without
    /// the SignerCapability.
    public fun retrieve_resource_account_cap(
        resource: &signer
    ): SignerCapability acquires SignerCapabilityStore {
        let SignerCapabilityStore { resource_signer_cap } = move_from<SignerCapabilityStore>(signer::address_of(resource));
        resource_signer_cap
    }
}
