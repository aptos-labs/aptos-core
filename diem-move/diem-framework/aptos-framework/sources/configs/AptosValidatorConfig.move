module AptosFramework::AptosValidatorConfig {
    use Std::Capability;
    use CoreFramework::ValidatorConfig;
    use AptosFramework::Marker;

    friend AptosFramework::AptosAccount;

    public fun initialize(account: &signer) {
        ValidatorConfig::initialize<Marker::ChainMarker>(account);
    }

    public(friend) fun publish(
        root_account: &signer,
        validator_account: &signer,
        human_name: vector<u8>,
    ) {
        ValidatorConfig::publish(
            validator_account,
            human_name,
            Capability::acquire(root_account, &Marker::get())
        );
    }
}
