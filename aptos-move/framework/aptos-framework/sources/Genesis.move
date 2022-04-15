module AptosFramework::Genesis {
    use Std::Signer;
    use Std::Event;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::ConsensusConfig;
    use AptosFramework::TransactionPublishingOption;
    use AptosFramework::ValidatorSet;
    use AptosFramework::Version;
    use AptosFramework::Block;
    use AptosFramework::ChainId;
    use AptosFramework::Reconfiguration;
    use AptosFramework::TestCoin;
    use AptosFramework::Timestamp;
    use AptosFramework::ValidatorConfig;
    use AptosFramework::ValidatorOperatorConfig;
    use AptosFramework::VMConfig;

    fun initialize(
        core_resource_account: signer,
        core_resource_account_auth_key: vector<u8>,
        initial_script_allow_list: vector<vector<u8>>,
        is_open_module: bool,
        instruction_schedule: vector<u8>,
        native_schedule: vector<u8>,
        chain_id: u8,
        initial_version: u64,
        consensus_config: vector<u8>,
        min_price_per_gas_unit: u64,
    ) {
        initialize_internal(
            &core_resource_account,
            core_resource_account_auth_key,
            initial_script_allow_list,
            is_open_module,
            instruction_schedule,
            native_schedule,
            chain_id,
            initial_version,
            consensus_config,
            min_price_per_gas_unit,
        )
    }

    fun initialize_internal(
        core_resource_account: &signer,
        core_resource_account_auth_key: vector<u8>,
        initial_script_allow_list: vector<vector<u8>>,
        is_open_module: bool,
        instruction_schedule: vector<u8>,
        native_schedule: vector<u8>,
        chain_id: u8,
        initial_version: u64,
        consensus_config: vector<u8>,
        min_price_per_gas_unit: u64,
    ) {
        // initialize the core resource account
        Account::initialize(
            core_resource_account,
            @AptosFramework,
            b"Account",
            b"script_prologue",
            b"module_prologue",
            b"writeset_prologue",
            b"script_prologue",
            b"epilogue",
            b"writeset_epilogue",
            false,
        );
        Account::create_account_internal(Signer::address_of(core_resource_account));
        Account::rotate_authentication_key_internal(core_resource_account, copy core_resource_account_auth_key);
        // initialize the core framework account
        let core_framework_account = Account::create_core_framework_account();
        Account::rotate_authentication_key_internal(&core_framework_account, core_resource_account_auth_key);

        // Consensus config setup
        ConsensusConfig::initialize(core_resource_account);
        ValidatorSet::initialize_validator_set(core_resource_account);
        Version::initialize(core_resource_account, initial_version);

        VMConfig::initialize(
            core_resource_account,
            instruction_schedule,
            native_schedule,
            min_price_per_gas_unit,
        );

        ConsensusConfig::set(core_resource_account, consensus_config);

        TransactionPublishingOption::initialize(core_resource_account, initial_script_allow_list, is_open_module);

        TestCoin::initialize(core_resource_account, 1000000);
        TestCoin::mint_internal(core_resource_account, Signer::address_of(core_resource_account), 18446744073709551615);

        // Pad the event counter for the Root account to match DPN. This
        // _MUST_ match the new epoch event counter otherwise all manner of
        // things start to break.
        Event::destroy_handle(Event::new_event_handle<u64>(core_resource_account));
        Event::destroy_handle(Event::new_event_handle<u64>(core_resource_account));

        // this needs to be called at the very end
        ChainId::initialize(core_resource_account, chain_id);
        Reconfiguration::initialize(core_resource_account);
        Block::initialize_block_metadata(core_resource_account);
        Timestamp::set_time_has_started(core_resource_account);
    }

    /// Sets up the initial validator set for the network.
    /// The validator "owner" accounts, their UTF-8 names, and their authentication
    /// keys are encoded in the `owners`, `owner_names`, and `owner_auth_key` vectors.
    /// Each validator signs consensus messages with the private key corresponding to the Ed25519
    /// public key in `consensus_pubkeys`.
    /// Each validator owner has its operation delegated to an "operator" (which may be
    /// the owner). The operators, their names, and their authentication keys are encoded
    /// in the `operators`, `operator_names`, and `operator_auth_keys` vectors.
    /// Finally, each validator must specify the network address
    /// (see types/src/network_address/mod.rs) for itself and its full nodes.
    fun create_initialize_owners_operators(
        core_resource_account: signer,
        owners: vector<signer>,
        owner_names: vector<vector<u8>>,
        owner_auth_keys: vector<vector<u8>>,
        consensus_pubkeys: vector<vector<u8>>,
        operators: vector<signer>,
        operator_names: vector<vector<u8>>,
        operator_auth_keys: vector<vector<u8>>,
        validator_network_addresses: vector<vector<u8>>,
        full_node_network_addresses: vector<vector<u8>>,
    ) {
        let num_owners = Vector::length(&owners);
        let num_owner_names = Vector::length(&owner_names);
        assert!(num_owners == num_owner_names, 0);
        let num_owner_keys = Vector::length(&owner_auth_keys);
        assert!(num_owner_names == num_owner_keys, 0);
        let num_operators = Vector::length(&operators);
        assert!(num_owner_keys == num_operators, 0);
        let num_operator_names = Vector::length(&operator_names);
        assert!(num_operators == num_operator_names, 0);
        let num_operator_keys = Vector::length(&operator_auth_keys);
        assert!(num_operator_names == num_operator_keys, 0);
        let num_validator_network_addresses = Vector::length(&validator_network_addresses);
        assert!(num_operator_keys == num_validator_network_addresses, 0);
        let num_full_node_network_addresses = Vector::length(&full_node_network_addresses);
        assert!(num_validator_network_addresses == num_full_node_network_addresses, 0);

        let i = 0;
        while (i < num_owners) {
            let owner = Vector::borrow(&owners, i);
            let owner_address = Signer::address_of(owner);
            let owner_name = *Vector::borrow(&owner_names, i);
            // create each validator account and rotate its auth key to the correct value
            Account::create_validator_account_internal(
                &core_resource_account, owner_address, owner_name
            );

            let owner_auth_key = *Vector::borrow(&owner_auth_keys, i);
            Account::rotate_authentication_key_internal(owner, owner_auth_key);

            let operator = Vector::borrow(&operators, i);
            let operator_address = Signer::address_of(operator);
            let operator_name = *Vector::borrow(&operator_names, i);
            // create the operator account + rotate its auth key if it does not already exist
            if (!Account::exists_at(operator_address)) {
                Account::create_validator_operator_account_internal(
                    &core_resource_account, operator_address, copy operator_name
                );
                let operator_auth_key = *Vector::borrow(&operator_auth_keys, i);
                Account::rotate_authentication_key_internal(operator, operator_auth_key);
            };
            // assign the operator to its validator
            assert!(ValidatorOperatorConfig::get_human_name(operator_address) == operator_name, 0);
            ValidatorConfig::set_operator(owner, operator_address);

            // use the operator account set up the validator config
            let validator_network_address = *Vector::borrow(&validator_network_addresses, i);
            let full_node_network_address = *Vector::borrow(&full_node_network_addresses, i);
            let consensus_pubkey = *Vector::borrow(&consensus_pubkeys, i);
            ValidatorConfig::set_config(
                operator,
                owner_address,
                consensus_pubkey,
                validator_network_address,
                full_node_network_address
            );

            // finally, add this validator to the validator set
            ValidatorSet::add_validator_internal(&core_resource_account, owner_address);

            i = i + 1;
        }
    }

    #[test_only]
    public fun setup(core_resource_account: &signer) {
        initialize_internal(
            core_resource_account,
            x"0000000000000000000000000000000000000000000000000000000000000000",
            Vector::empty(),
            true,
            x"", // instruction_schedule not needed for unit tests
            x"", // native schedule not needed for unit tests
            4u8, // TESTING chain ID
            0,
            x"",
            1,
        )
    }

    #[test(account = @CoreResources)]
    fun test_setup(account: signer) {
        use AptosFramework::Account;

        setup(&account);
        assert!(Account::exists_at(@AptosFramework), 0);
        assert!(Account::exists_at(@CoreResources), 0);
    }
}
