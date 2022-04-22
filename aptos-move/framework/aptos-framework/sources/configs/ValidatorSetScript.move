module AptosFramework::ValidatorSetScript {
    use AptosFramework::ValidatorConfig;
    use AptosFramework::ValidatorOperatorConfig;
    use AptosFramework::ValidatorSet;

    public(script) fun register_validator_config(
        validator_operator_account: signer,
        validator_address: address,
        consensus_pubkey: vector<u8>,
        validator_network_addresses: vector<u8>,
        fullnode_network_addresses: vector<u8>,
    ) {
        ValidatorConfig::set_config(
            &validator_operator_account,
            validator_address,
            consensus_pubkey,
            validator_network_addresses,
            fullnode_network_addresses
        );
    }

    public(script) fun set_validator_operator(
        account: signer,
        operator_name: vector<u8>,
        operator_account: address
    ) {
        assert!(ValidatorOperatorConfig::get_human_name(operator_account) == operator_name, 0);
        ValidatorConfig::set_operator(&account, operator_account);
    }

    public(script) fun set_validator_config_and_reconfigure(
        validator_operator_account: signer,
        validator_account: address,
        consensus_pubkey: vector<u8>,
        validator_network_addresses: vector<u8>,
        fullnode_network_addresses: vector<u8>,
    ) {
        ValidatorConfig::set_config(
            &validator_operator_account,
            validator_account,
            consensus_pubkey,
            validator_network_addresses,
            fullnode_network_addresses
        );
        ValidatorSet::update_config_and_reconfigure(&validator_operator_account, validator_account);
    }
}
