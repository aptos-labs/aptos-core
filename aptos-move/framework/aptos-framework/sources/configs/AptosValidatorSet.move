module AptosFramework::AptosValidatorSet {
    use Std::Capability;
    use AptosFramework::ValidatorSystem;
    use AptosFramework::Marker;

    public fun initialize_validator_set(
        account: &signer,
    ) {
        ValidatorSystem::initialize_validator_set<Marker::ChainMarker>(account);
    }

    public(script) fun add_validator(
        account: signer,
        validator_addr: address,
    ) {
        add_validator_internal(&account, validator_addr);
    }

    public(script) fun remove_validator(
        account: signer,
        validator_addr: address,
    ) {
        ValidatorSystem::remove_validator(
            validator_addr,
            Capability::acquire(&account, &Marker::get())
        );
    }

    public fun add_validator_internal(
        account: &signer,
        validator_addr: address,
    ) {
        ValidatorSystem::add_validator(
            validator_addr,
            Capability::acquire(account, &Marker::get())
        );
    }
}
