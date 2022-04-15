/// Maintains the version number for the blockchain.
module AptosFramework::Version {
    use Std::Errors;
    use AptosFramework::Reconfiguration;
    use AptosFramework::Timestamp;
    use AptosFramework::SystemAddresses;

    struct Version has key, copy, drop, store {
        major: u64,
    }

    /// Error with config
    const ECONFIG: u64 = 0;
    /// Tried to set an invalid major version for the VM. Major versions must be strictly increasing
    const EINVALID_MAJOR_VERSION_NUMBER: u64 = 1;

    /// Publishes the Version config.
    public fun initialize(account: &signer, initial_version: u64) {
        Timestamp::assert_genesis();

        SystemAddresses::assert_core_resource(account);

        assert!(
            !exists<Version>(@CoreResources),
            Errors::already_published(ECONFIG)
        );

        move_to(
            account,
            Version { major: initial_version },
        );
    }

    /// Updates the major version to a larger version.
    public(script) fun set_version(account: signer, major: u64) acquires Version {
        SystemAddresses::assert_core_resource(&account);
        assert!(exists<Version>(@CoreResources), Errors::not_published(ECONFIG));
        let old_major = *&borrow_global<Version>(@CoreResources).major;

        assert!(
            old_major < major,
            Errors::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER)
        );

        let config = borrow_global_mut<Version>(@CoreResources);
        config.major = major;

        Reconfiguration::reconfigure();
    }
}
