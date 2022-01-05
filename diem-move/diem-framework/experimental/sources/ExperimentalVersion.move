/// Maintains the version number for the blockchain.
module ExperimentalFramework::ExperimentalVersion {
    use Std::Capability;
    use CoreFramework::DiemVersion;

    struct ExperimentalVersion has drop {}

    /// Publishes the Version config.
    public fun initialize(account: &signer, initial_version: u64) {
        DiemVersion::initialize<ExperimentalVersion>(account, initial_version);
        Capability::create<ExperimentalVersion>(account, &ExperimentalVersion {});
    }

    /// Updates the major version to a larger version.
    public fun set(account: &signer, major: u64) {
        DiemVersion::set<ExperimentalVersion>(
            major,
            &Capability::acquire(account, &ExperimentalVersion {}),
        );
    }
}
