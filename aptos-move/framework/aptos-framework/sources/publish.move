/// Module for publishing packages to resource accounts.
module aptos_framework::publish {
    use std::signer;
    use aptos_framework::account::{Self, SignerCapability};

    /// Holds the SignerCapability.
    /// This can only ever be held by the deployer.
    struct SignerCapabilityStore has key, drop {
        /// The SignerCapability of the resource.
        resource_signer_cap: SignerCapability
    }

    public entry fun publish_package_txn(
        deployer: &signer,
        seed: vector<u8>,
        metadata_serialized: vector<u8>,
        code: vector<vector<u8>>,
    ) {
        let (resource, resource_signer_cap) = account::create_resource_account(deployer, seed);
        aptos_framework::code::publish_package_txn(&resource, metadata_serialized, code);
        move_to(deployer, SignerCapabilityStore { resource_signer_cap });
    }

    /// Retrieves the resource account signer capability once, allowing the package to be able
    /// to sign as itself.
    public fun retrieve_resource_account_cap(
        deployer: &signer
    ): SignerCapability acquires SignerCapabilityStore {
        let SignerCapabilityStore { resource_signer_cap } = move_from<SignerCapabilityStore>(signer::address_of(deployer));
        resource_signer_cap
    }
}
