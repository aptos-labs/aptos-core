module ExperimentalFramework::ExperimentalValidatorOperatorConfig {
    use Std::Capability;
    use CoreFramework::ValidatorOperatorConfig;

    friend ExperimentalFramework::ExperimentalAccount;

    struct ExperimentalValidatorOperatorConfig has drop {}

    public fun initialize(account: &signer) {
        ValidatorOperatorConfig::initialize<ExperimentalValidatorOperatorConfig>(account);
        Capability::create(account, &ExperimentalValidatorOperatorConfig{});
    }

    public(friend) fun publish(
        root_account: &signer,
        validator_operator_account: &signer,
        human_name: vector<u8>,
    ) {
        ValidatorOperatorConfig::publish(
            validator_operator_account,
            human_name,
            Capability::acquire(root_account, &ExperimentalValidatorOperatorConfig{})
        );
    }
}
