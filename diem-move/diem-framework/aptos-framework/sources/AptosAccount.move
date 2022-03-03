/// The `AptosAccount` module manages experimental accounts.
/// It also defines the prolog and epilog that run before and after every
/// transaction in addition to the core prologue and epilogue.

module AptosFramework::AptosAccount {
    use Std::Errors;
    use CoreFramework::Account;
    use CoreFramework::DiemTimestamp;
    use CoreFramework::SystemAddresses;
    use AptosFramework::Marker;
    use AptosFramework::AptosValidatorConfig;
    use AptosFramework::AptosValidatorOperatorConfig;
    use AptosFramework::TestCoin;
    use AptosFramework::TransactionFee;

    const MAX_U64: u128 = 18446744073709551615;

    const ECANNOT_CREATE_AT_VM_RESERVED: u64 = 0;
    const EGAS: u64 = 1;
    const ECANNOT_CREATE_AT_CORE_CODE: u64 = 2;

    fun create_account_internal(account_address: address, auth_key_prefix: vector<u8>): (signer, vector<u8>) {
        assert!(
            account_address != @VMReserved,
            Errors::invalid_argument(ECANNOT_CREATE_AT_VM_RESERVED)
        );
        assert!(
            account_address != @CoreFramework,
            Errors::invalid_argument(ECANNOT_CREATE_AT_CORE_CODE)
        );
        Account::create_account(account_address, auth_key_prefix, &Marker::get())
    }

    /// Initialize this module. This is only callable from genesis.
    public fun initialize(core_resource: &signer) {
        DiemTimestamp::assert_genesis();
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
    /// Basic account creation method: no roles attached, no conditions checked.
    ///////////////////////////////////////////////////////////////////////////

    public fun create_account(
        new_account_address: address,
        auth_key_prefix: vector<u8>,
    ): signer {
        let (signer, _) = create_account_internal(new_account_address, auth_key_prefix);
        signer
    }

    public fun exists_at(addr: address): bool {
        Account::exists_at(addr)
    }

    /// Create a Validator account
    public fun create_validator_account(
        core_resource: &signer,
        new_account_address: address,
        auth_key_prefix: vector<u8>,
        human_name: vector<u8>,
    ) {
        let (new_account, _) = create_account_internal(new_account_address, auth_key_prefix);
        AptosValidatorConfig::publish(core_resource, &new_account, human_name);
    }

    /// Create a Validator Operator account
    public fun create_validator_operator_account(
        core_resource: &signer,
        new_account_address: address,
        auth_key_prefix: vector<u8>,
        human_name: vector<u8>,
    ) {
        let (new_account, _) = create_account_internal(new_account_address, auth_key_prefix);
        AptosValidatorOperatorConfig::publish(core_resource, &new_account, human_name);
    }

    /// Rotate the authentication key for the account under cap.account_address
    public fun rotate_authentication_key(
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
        _txn_gas_price: u64,
        _txn_max_gas_units: u64,
        _txn_expiration_time: u64,
        chain_id: u8,
    ) {
        Account::prologue(&sender, txn_sequence_number, txn_public_key, chain_id)
    }

    fun script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        _txn_gas_price: u64,
        _txn_max_gas_units: u64,
        _txn_expiration_time: u64,
        chain_id: u8,
        _script_hash: vector<u8>,
    ) {
        Account::prologue(&sender, txn_sequence_number, txn_public_key, chain_id)
    }

    fun writeset_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        _txn_expiration_time: u64,
        chain_id: u8,
    ) {
        Account::prologue(&sender, txn_sequence_number, txn_public_key, chain_id)
    }

    // Might be able to combine this
    fun multi_agent_script_prologue(
        sender: signer,
        txn_sequence_number: u64,
        txn_sender_public_key: vector<u8>,
        _secondary_signer_addresses: vector<address>,
        _secondary_signer_public_key_hashes: vector<vector<u8>>,
        _txn_gas_price: u64,
        _txn_max_gas_units: u64,
        _txn_expiration_time: u64,
        chain_id: u8,
    ) {
         Account::prologue(&sender, txn_sequence_number, txn_sender_public_key, chain_id)
    }

    fun epilogue(
        account: signer,
        _txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) {
        // [EA1; Invariant]: Make sure that the transaction's `max_gas_units` is greater
        // than the number of gas units remaining after execution.
        assert!(txn_max_gas_units >= gas_units_remaining, Errors::invalid_argument(EGAS));
        let gas_used = txn_max_gas_units - gas_units_remaining;

        // [EA2; Invariant]: Make sure that the transaction fee would not overflow maximum
        // number representable in a u64. Already checked in [PCA5].
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
        core_resource: signer,
        _txn_sequence_number: u64,
        should_trigger_reconfiguration: bool,
    ) {
        Account::writeset_epilogue(&core_resource, should_trigger_reconfiguration, &Marker::get());
    }
}
