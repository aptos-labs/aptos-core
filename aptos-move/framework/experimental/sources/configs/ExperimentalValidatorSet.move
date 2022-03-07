module ExperimentalFramework::ExperimentalValidatorSet {
    use Std::Capability;
    use CoreFramework::DiemSystem;

    struct ExperimentalValidatorSet has drop {}

    public fun initialize_validator_set(
        account: &signer,
    ) {
        DiemSystem::initialize_validator_set<ExperimentalValidatorSet>(account);
        Capability::create(account, &ExperimentalValidatorSet {});
    }

    public fun add_validator(
        account: &signer,
        validator_addr: address,
    ) {
        DiemSystem::add_validator(
            validator_addr,
            Capability::acquire(account, &ExperimentalValidatorSet {})
        );
    }

    public fun remove_validator(
        account: &signer,
        validator_addr: address,
    ) {
        DiemSystem::remove_validator(
            validator_addr,
            Capability::acquire(account, &ExperimentalValidatorSet {})
        );
    }
}
