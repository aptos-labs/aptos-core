/// Maintains the version number for the blockchain.
module aptos_framework::version {
    use std::error;
    use aptos_framework::reconfiguration;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;

    struct Version has key, copy, drop, store {
        major: u64,
    }

    /// Error with config
    const ECONFIG: u64 = 0;
    /// Tried to set an invalid major version for the VM. Major versions must be strictly increasing
    const EINVALID_MAJOR_VERSION_NUMBER: u64 = 1;

    /// Publishes the Version config.
    public fun initialize(account: &signer, initial_version: u64) {
        timestamp::assert_genesis();
        system_addresses::assert_aptos_framework(account);

        assert!(
            !exists<Version>(@aptos_framework),
            error::already_exists(ECONFIG)
        );

        move_to(
            account,
            Version { major: initial_version },
        );
    }

    /// Updates the major version to a larger version.
    /// This is only used in test environments and outside of them, the core resources account shouldn't exist.
    public entry fun set_version(account: signer, major: u64) acquires Version {
        system_addresses::assert_core_resource(&account);
        assert!(exists<Version>(@aptos_framework), error::not_found(ECONFIG));
        let old_major = *&borrow_global<Version>(@aptos_framework).major;

        assert!(
            old_major < major,
            error::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER)
        );

        let config = borrow_global_mut<Version>(@aptos_framework);
        config.major = major;

        reconfiguration::reconfigure();
    }
}
