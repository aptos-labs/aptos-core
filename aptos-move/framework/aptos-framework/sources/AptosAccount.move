/// The `AptosAccount` module manages Aptos accounts.
/// It also defines the prolog and epilog that run before and after every
/// transaction in addition to the core prologue and epilogue.

module AptosFramework::AptosAccount {
    use Std::Errors;
    use Std::Signer;
    use CoreFramework::Account;
    use CoreFramework::Timestamp;
    use CoreFramework::SystemAddresses;
    use CoreFramework::TransactionPublishingOption;
    use AptosFramework::Marker;
    use AptosFramework::AptosValidatorConfig;
    use AptosFramework::AptosValidatorOperatorConfig;
    use AptosFramework::TestCoin;
    use AptosFramework::TransactionFee;

    friend AptosFramework::Genesis;

    const MAX_U64: u128 = 18446744073709551615;

    const ECANNOT_CREATE_AT_VM_RESERVED: u64 = 0;
    const EGAS: u64 = 1;
    const ECANNOT_CREATE_AT_CORE_CODE: u64 = 2;
    const EADDR_NOT_MATCH_PREIMAGE: u64 = 3;
    const EWRITESET_NOT_ALLOWED: u64 = 6;
    const EMULTI_AGENT_NOT_SUPPORTED: u64 = 7;
    const EMODULE_NOT_ALLOWED: u64 = 8;
    const ESCRIPT_NOT_ALLOWED: u64 = 9;

    /// Prologue errors. These are separated out from the other errors in this
    /// module since they are mapped separately to major VM statuses, and are
    /// important to the semantics of the system.
    const PROLOGUE_ECANT_PAY_GAS_DEPOSIT: u64 = 1005;
    const PROLOGUE_ETRANSACTION_EXPIRED: u64 = 1006;
    const PROLOGUE_ESCRIPT_NOT_ALLOWED: u64 = 1008;
    const PROLOGUE_EMODULE_NOT_ALLOWED: u64 = 1009;
    const PROLOGUE_EINVALID_WRITESET_SENDER: u64 = 1010;

    public(friend) fun create_account_internal(auth_key: address): (signer, vector<u8>) {
        assert!(
            auth_key != @VMReserved,
            Errors::invalid_argument(ECANNOT_CREATE_AT_VM_RESERVED)
        );
        assert!(
            auth_key != @CoreFramework,
            Errors::invalid_argument(ECANNOT_CREATE_AT_CORE_CODE)
        );
        Account::create_account(auth_key, &Marker::get())
    }

    /// Create the account for @CoreFramework to help module upgrades on testnet.
    public(friend) fun create_core_framework_account(): signer {
        Timestamp::assert_genesis();
        let (signer, _) = Account::create_account(@CoreFramework, &Marker::get());
        signer
    }

    /// Initialize this module. This is only callable from genesis.
    public fun initialize(core_resource: &signer) {
        Timestamp::assert_genesis();
        // Operational constraint, not a privilege constraint.
        SystemAddresses::assert_core_resource(core_resource);
        Account::initialize<Marker::ChainMarker>(
            core_resource,
            @CoreFramework,
            b"AptosAccount",
            b"script_prologue",
            b"module_prologue",
            b"writeset_prologue",
            b"script_prologue",
            b"epilogue",
            b"writeset_epilogue",
            false,
        );
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Basic account creation method.
    ///////////////////////////////////////////////////////////////////////////

    public(script) fun create_account(auth_key: address) {
        let (signer, _) = create_account_internal(auth_key);
        TestCoin::register(&signer);
    }

    public fun exists_at(addr: address): bool {
        Account::exists_at(addr)
    }

    /// Create a Validator account
    public(script) fun create_validator_account(
        core_resource: signer,
        new_account_address: address,
        human_name: vector<u8>,
    ) {
        create_validator_account_internal(&core_resource, new_account_address, human_name);
    }

    public fun create_validator_account_internal(
        core_resource: &signer,
        new_account_address: address,
        human_name: vector<u8>,
    ) {
        let (new_account, _) = create_account_internal(new_account_address);
        AptosValidatorConfig::publish(core_resource, &new_account, human_name);
    }

    /// Create a Validator Operator account
    public(script) fun create_validator_operator_account(
        core_resource: signer,
        new_account_address: address,
        human_name: vector<u8>,
    ) {
        create_validator_operator_account_internal(&core_resource, new_account_address, human_name)
    }

    public fun create_validator_operator_account_internal(
        core_resource: &signer,
        new_account_address: address,
        human_name: vector<u8>,
    ) {
        let (new_account, _) = create_account_internal(new_account_address);
        AptosValidatorOperatorConfig::publish(core_resource, &new_account, human_name);
    }

    /// Rotate the authentication key for the account under cap.account_address
    public(script) fun rotate_authentication_key(
        account: signer,
        new_authentication_key: vector<u8>,
    ) {
      rotate_authentication_key_internal(&account, new_authentication_key);
    }

    public fun rotate_authentication_key_internal(
        account: &signer,
        new_authentication_key: vector<u8>,
    ) {
        Account::rotate_authentication_key(account, new_authentication_key)
    }

    ///////////////////////////////////////////////////////////////////////////
    // Prologues and epilogues
    ///////////////////////////////////////////////////////////////////////////
    fun module_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) {
        assert!(TransactionPublishingOption::is_module_allowed(), Errors::invalid_state(PROLOGUE_EMODULE_NOT_ALLOWED));
        prologue_common(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)
    }

    fun script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        script_hash: vector<u8>,
    ) {
        assert!(TransactionPublishingOption::is_script_allowed(&script_hash), Errors::invalid_state(PROLOGUE_ESCRIPT_NOT_ALLOWED));
        prologue_common(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)
    }

    fun writeset_prologue(
        _sender: signer,
        _txn_sequence_number: u64,
        _txn_public_key: vector<u8>,
        _txn_expiration_time: u64,
        _chain_id: u8,
    ) {
        assert!(false, Errors::invalid_argument(PROLOGUE_EINVALID_WRITESET_SENDER));
    }

    fun multi_agent_script_prologue(
        _sender: signer,
        _txn_sequence_number: u64,
        _txn_sender_public_key: vector<u8>,
        _secondary_signer_addresses: vector<address>,
        _secondary_signer_public_key_hashes: vector<vector<u8>>,
        _txn_gas_price: u64,
        _txn_max_gas_units: u64,
        _txn_expiration_time: u64,
        _chain_id: u8,
    ) {
        assert!(false, Errors::invalid_argument(EMULTI_AGENT_NOT_SUPPORTED));
    }

    fun epilogue(
        account: signer,
        _txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) {
        assert!(txn_max_gas_units >= gas_units_remaining, Errors::invalid_argument(EGAS));
        let gas_used = txn_max_gas_units - gas_units_remaining;

        assert!(
            (txn_gas_price as u128) * (gas_used as u128) <= MAX_U64,
            Errors::limit_exceeded(EGAS)
        );
        let transaction_fee_amount = txn_gas_price * gas_used;
        let coin = TestCoin::withdraw(&account, transaction_fee_amount);
        TransactionFee::burn_fee(coin);

        Account::epilogue(&account, &Marker::get());
    }

    fun writeset_epilogue(
        _core_resource: signer,
        _txn_sequence_number: u64,
        _should_trigger_reconfiguration: bool,
    ) {
        assert!(false, Errors::invalid_argument(EWRITESET_NOT_ALLOWED));
    }

    fun prologue_common(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) {
        assert!(
            Timestamp::now_seconds() < txn_expiration_time,
            Errors::invalid_argument(PROLOGUE_ETRANSACTION_EXPIRED),
        );
        Account::prologue(&sender, txn_sequence_number, txn_public_key, chain_id);
        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        let addr = Signer::address_of(&sender);
        assert!(TestCoin::exists_at(addr), Errors::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
        let balance = TestCoin::balance_of(addr);
        assert!(balance >= max_transaction_fee, Errors::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
    }
}
