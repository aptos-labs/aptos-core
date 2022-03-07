module AptosFramework::AptosVersion {
    use Std::Capability;
    use CoreFramework::DiemVersion;
    use AptosFramework::Marker;

    /// Publishes the Version config.
    public fun initialize(account: &signer, initial_version: u64) {
        DiemVersion::initialize<Marker::ChainMarker>(account, initial_version);
    }

    /// Updates the major version to a larger version.
    public fun set(account: &signer, major: u64) {
        DiemVersion::set<Marker::ChainMarker>(
            major,
            &Capability::acquire(account, &Marker::get()),
        );
    }
}
