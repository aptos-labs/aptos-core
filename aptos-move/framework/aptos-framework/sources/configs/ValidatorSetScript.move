// placeholder to maintain compatibility
module AptosFramework::ValidatorSetScript {

    public(script) fun register_validator_config(
       _validator_operator_account: signer,
       _validator_address: address,
       _consensus_pubkey: vector<u8>,
       _validator_network_addresses: vector<u8>,
       _fullnode_network_addresses: vector<u8>,
    ) {
    }

    public(script) fun set_validator_operator(
        _account: signer,
        _operator_name: vector<u8>,
        _operator_account: address
    ) {
    }

    public(script) fun set_validator_config_and_reconfigure(
        _validator_operator_account: signer,
        _validator_account: address,
        _consensus_pubkey: vector<u8>,
        _validator_network_addresses: vector<u8>,
        _fullnode_network_addresses: vector<u8>,
    ) {
    }

    public(script) fun create_validator_account(
        _core_resource: signer,
        _new_account_address: address,
        _human_name: vector<u8>,
    ) {
    }

    public(script) fun create_validator_operator_account(
        _core_resource: signer,
        _new_account_address: address,
        _human_name: vector<u8>,
    ) {
    }

    public(script) fun add_validator(
        _account: signer,
        _validator_addr: address
    ) {
    }

    public(script) fun remove_validator(
        _account: signer,
        _validator_addr: address
    ) {
    }
}
