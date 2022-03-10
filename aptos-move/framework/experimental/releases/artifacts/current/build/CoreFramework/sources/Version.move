/// Maintains the version number for the blockchain.
module CoreFramework::Version {
    use Std::Capability::Cap;
    use Std::Errors;
    use CoreFramework::Reconfiguration;
    use CoreFramework::Timestamp;
    use CoreFramework::SystemAddresses;

    /// Marker to be stored under 0x1 during genesis
    struct VersionChainMarker<phantom T> has key {}

    struct Version has key, copy, drop, store {
        major: u64,
    }

    /// Error with chain marker
    const ECHAIN_MARKER: u64 = 0;
    /// Error with config
    const ECONFIG: u64 = 1;
    /// Tried to set an invalid major version for the VM. Major versions must be strictly increasing
    const EINVALID_MAJOR_VERSION_NUMBER: u64 = 2;

    /// Publishes the Version config.
    public fun initialize<T>(account: &signer, initial_version: u64) {
        Timestamp::assert_genesis();

        SystemAddresses::assert_core_resource(account);

        assert!(
            !exists<VersionChainMarker<T>>(@CoreResources),
            Errors::already_published(ECHAIN_MARKER)
        );

        assert!(
            !exists<Version>(@CoreResources),
            Errors::already_published(ECONFIG)
        );

        move_to(
            account,
            VersionChainMarker<T> {},
        );
        move_to(
            account,
            Version { major: initial_version },
        );
    }

    /// Updates the major version to a larger version.
    public fun set<T>(major: u64, _cap: &Cap<T>) acquires Version {
        assert!(exists<VersionChainMarker<T>>(@CoreResources), Errors::not_published(ECHAIN_MARKER));
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
