module ExperimentalFramework::ExperimentalValidatorConfig {
    use Std::Capability;
    use CoreFramework::ValidatorConfig;

    friend ExperimentalFramework::ExperimentalAccount;

    struct ExperimentalValidatorConfig has drop {}

    public fun initialize(account: &signer) {
        ValidatorConfig::initialize<ExperimentalValidatorConfig>(account);
        Capability::create(account, &ExperimentalValidatorConfig{});
    }

    public(friend) fun publish(
        root_account: &signer,
        validator_account: &signer,
        human_name: vector<u8>,
    ) {
        ValidatorConfig::publish(
            validator_account,
            human_name,
            Capability::acquire(root_account, &ExperimentalValidatorConfig{})
        );
    }
}
