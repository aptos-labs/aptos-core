module AptosFramework::Genesis {
    use Std::Signer;
    use Std::Event;
    use Std::Vector;
    use AptosFramework::Account;
    use AptosFramework::ConsensusConfig;
    use AptosFramework::TransactionPublishingOption;
    use AptosFramework::Version;
    use AptosFramework::Block;
    use AptosFramework::ChainId;
    use AptosFramework::Reconfiguration;
    use AptosFramework::Stake;
    use AptosFramework::TestCoin;
    use AptosFramework::Timestamp;
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
        epoch_interval: u64,
        minimum_stake: u64,
        maximum_stake: u64,
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
            epoch_interval,
            minimum_stake,
            maximum_stake,
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
        epoch_interval: u64,
        minimum_stake: u64,
        maximum_stake: u64,
    ) {
        // initialize the core resource account
        Account::initialize(
            core_resource_account,
            @AptosFramework,
            b"Account",
            b"script_prologue",
            b"module_prologue",
            b"writeset_prologue",
            b"multi_agent_script_prologue",
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
        Version::initialize(core_resource_account, initial_version);
        Stake::initialize_validator_set(core_resource_account, minimum_stake, maximum_stake);

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
        Block::initialize_block_metadata(core_resource_account, epoch_interval);
        Timestamp::set_time_has_started(core_resource_account);
    }

    /// Sets up the initial validator set for the network.
    /// The validator "owner" accounts, and their authentication
    /// keys are encoded in the `owners` and `owner_auth_key` vectors.
    /// Each validator signs consensus messages with the private key corresponding to the Ed25519
    /// public key in `consensus_pubkeys`.
    /// Finally, each validator must specify the network address
    /// (see types/src/network_address/mod.rs) for itself and its full nodes.
    public(script) fun create_initialize_validators(
        core_resource_account: signer,
        owners: vector<address>,
        owner_auth_keys: vector<vector<u8>>,
        consensus_pubkeys: vector<vector<u8>>,
        validator_network_addresses: vector<vector<u8>>,
        full_node_network_addresses: vector<vector<u8>>,
        staking_distribution: vector<u64>,
    ) {
        let num_owners = Vector::length(&owners);
        let num_owner_keys = Vector::length(&owner_auth_keys);
        assert!(num_owners == num_owner_keys, 0);
        let num_validator_network_addresses = Vector::length(&validator_network_addresses);
        assert!(num_owner_keys == num_validator_network_addresses, 0);
        let num_full_node_network_addresses = Vector::length(&full_node_network_addresses);
        assert!(num_validator_network_addresses == num_full_node_network_addresses, 0);
        let num_staking = Vector::length(&staking_distribution);
        assert!(num_full_node_network_addresses == num_staking, 0);

        let i = 0;
        while (i < num_owners) {
            let owner = Vector::borrow(&owners, i);
            // create each validator account and rotate its auth key to the correct value
            let (owner_account, _) = Account::create_account_internal(*owner);

            let owner_auth_key = *Vector::borrow(&owner_auth_keys, i);
            Account::rotate_authentication_key_internal(&owner_account, owner_auth_key);

            // use the operator account set up the validator config
            let validator_network_address = *Vector::borrow(&validator_network_addresses, i);
            let full_node_network_address = *Vector::borrow(&full_node_network_addresses, i);
            let consensus_pubkey = *Vector::borrow(&consensus_pubkeys, i);
            Stake::register_validator_candidate(
                &owner_account,
                consensus_pubkey,
                validator_network_address,
                full_node_network_address
            );
            let amount = *Vector::borrow(&staking_distribution, i);
            Stake::delegate_stake(&core_resource_account, *owner, amount, 100000);
            Stake::join_validator_set(&owner_account);

            i = i + 1;
        };
        Stake::on_new_epoch();
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
            0,
            0,
            0
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
