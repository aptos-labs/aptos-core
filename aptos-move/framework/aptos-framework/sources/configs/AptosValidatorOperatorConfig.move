module AptosFramework::AptosValidatorOperatorConfig {
    use Std::Capability;
    use AptosFramework::ValidatorOperatorConfig;
    use AptosFramework::Marker;

    friend AptosFramework::AptosAccount;

    public fun initialize(account: &signer) {
        ValidatorOperatorConfig::initialize<Marker::ChainMarker>(account);
    }

    public(friend) fun publish(
        root_account: &signer,
        validator_operator_account: &signer,
        human_name: vector<u8>,
    ) {
        ValidatorOperatorConfig::publish(
            validator_operator_account,
            human_name,
            Capability::acquire(root_account, &Marker::get())
        );
    }
}
