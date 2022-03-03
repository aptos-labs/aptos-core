module AptosFramework::AptosValidatorSet {
    use Std::Capability;
    use CoreFramework::DiemSystem;
    use AptosFramework::Marker;

    public fun initialize_validator_set(
        account: &signer,
    ) {
        DiemSystem::initialize_validator_set<Marker::ChainMarker>(account);
    }

    public fun add_validator(
        account: &signer,
        validator_addr: address,
    ) {
        DiemSystem::add_validator(
            validator_addr,
            Capability::acquire(account, &Marker::get())
        );
    }

    public fun remove_validator(
        account: &signer,
        validator_addr: address,
    ) {
        DiemSystem::remove_validator(
            validator_addr,
            Capability::acquire(account, &Marker::get())
        );
    }
}
