/// Maintains the version number for the blockchain.
module aptos_framework::version {
    use std::error;
    use std::signer;

    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

    struct Version has key, copy, drop, store {
        major: u64,
    }

    /// Tried to set an invalid major version for the VM. Major versions must be strictly increasing
    const EINVALID_MAJOR_VERSION_NUMBER: u64 = 1;
    const ENOT_AUTHORIZED: u64 = 2;

    /// Only called during genesis.
    /// Publishes the Version config.
    public(friend) fun initialize(account: &signer, initial_version: u64) {
        system_addresses::assert_aptos_framework(account);

        move_to(
            account,
            Version { major: initial_version },
        );
    }

    /// Updates the major version to a larger version.
    /// This is only used in test environments and outside of them, the core resources account shouldn't exist.
    public entry fun set_version(account: &signer, major: u64) acquires Version {
        let address = signer::address_of(account);
        // Core resources account only exists in tests or testnets. In mainnet set up, only on chain governance can call
        // with the aptos framework account to update the version.
        assert!(
            system_addresses::is_aptos_framework_address(address) || system_addresses::is_core_resource_address(address),
            error::permission_denied(ENOT_AUTHORIZED));

        let old_major = *&borrow_global<Version>(@aptos_framework).major;
        assert!(
            old_major < major,
            error::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER)
        );

        let config = borrow_global_mut<Version>(@aptos_framework);
        config.major = major;

        // Need to trigger reconfiguration so validator nodes can sync on the updated version.
        reconfiguration::reconfigure();
    }
}
