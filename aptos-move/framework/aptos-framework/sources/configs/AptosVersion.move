module AptosFramework::AptosVersion {
    use Std::Capability;
    use CoreFramework::Version;
    use AptosFramework::Marker;

    /// Publishes the Version config.
    public fun initialize(account: &signer, initial_version: u64) {
        Version::initialize<Marker::ChainMarker>(account, initial_version);
    }

    /// Updates the major version to a larger version.
    public fun set(account: &signer, major: u64) {
        Version::set<Marker::ChainMarker>(
            major,
            &Capability::acquire(account, &Marker::get()),
        );
    }
}
