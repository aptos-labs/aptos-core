/// Maintains the version number for the blockchain.
module CoreFramework::DiemVersion {
    use Std::Capability::Cap;
    use Std::Errors;
    use Std::Signer;
    use CoreFramework::DiemTimestamp;

    /// Marker to be stored under 0x1 during genesis
    struct VersionChainMarker<phantom T> has key {}

    struct DiemVersion has key, copy, drop, store {
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
        DiemTimestamp::assert_genesis();

        assert!(Signer::address_of(account) == @CoreResources, Errors::requires_address(ECONFIG));

        assert!(
            !exists<VersionChainMarker<T>>(@CoreResources),
            Errors::already_published(ECHAIN_MARKER)
        );

        assert!(
            !exists<DiemVersion>(@CoreResources),
            Errors::already_published(ECONFIG)
        );

        move_to(
            account,
            VersionChainMarker<T> {},
        );
        move_to(
            account,
            DiemVersion { major: initial_version },
        );
    }

    spec initialize {
        include DiemTimestamp::AbortsIfNotGenesis;
        aborts_if Signer::address_of(account) != @CoreResources with Errors::REQUIRES_ADDRESS;
        aborts_if exists<VersionChainMarker<T>>(@CoreResources) with Errors::ALREADY_PUBLISHED;
        aborts_if exists<DiemVersion>(@CoreResources) with Errors::ALREADY_PUBLISHED;
        ensures exists<VersionChainMarker<T>>(@CoreResources);
        ensures exists<DiemVersion>(@CoreResources);
        ensures global<DiemVersion>(@CoreResources).major == initial_version;
    }

    /// Updates the major version to a larger version.
    public fun set<T>(major: u64, _cap: &Cap<T>) acquires DiemVersion {
        assert!(exists<VersionChainMarker<T>>(@CoreResources), Errors::not_published(ECHAIN_MARKER));
        assert!(exists<DiemVersion>(@CoreResources), Errors::not_published(ECONFIG));
        let old_major = *&borrow_global<DiemVersion>(@CoreResources).major;

        assert!(
            old_major < major,
            Errors::invalid_argument(EINVALID_MAJOR_VERSION_NUMBER)
        );

        let config = borrow_global_mut<DiemVersion>(@CoreResources);
        config.major = major;
    }

    spec set {
        aborts_if !exists<VersionChainMarker<T>>(@CoreResources) with Errors::NOT_PUBLISHED;
        aborts_if !exists<DiemVersion>(@CoreResources) with Errors::NOT_PUBLISHED;
        aborts_if global<DiemVersion>(@CoreResources).major >= major with Errors::INVALID_ARGUMENT;
        ensures global<DiemVersion>(@CoreResources).major == major;
    }
}
