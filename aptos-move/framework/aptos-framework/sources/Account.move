module AptosFramework::Account {
    use Std::BCS;
    use Std::Errors;
    use Std::Hash;
    use Std::Signer;
    use Std::Vector;
    use AptosFramework::ChainId;
    use AptosFramework::TestCoin;
    use AptosFramework::Timestamp;
    use AptosFramework::TransactionFee;
    use AptosFramework::TransactionPublishingOption;

    friend AptosFramework::Genesis;

    /// Resource representing an account.
    struct Account has key, store {
        authentication_key: vector<u8>,
        sequence_number: u64,
        self_address: address,
    }

    /// This holds information that will be picked up by the VM to call the
    /// correct chain-specific prologue and epilogue functions
    struct ChainSpecificAccountInfo has key {
        module_addr: address,
        module_name: vector<u8>,
        script_prologue_name: vector<u8>,
        module_prologue_name: vector<u8>,
        writeset_prologue_name: vector<u8>,
        multi_agent_prologue_name: vector<u8>,
        user_epilogue_name: vector<u8>,
        writeset_epilogue_name: vector<u8>,
        currency_code_required: bool,
    }

    const MAX_U64: u128 = 18446744073709551615;

    /// Account already existed
    const EACCOUNT: u64 = 0;
    /// Sequence number exceeded the maximum value for a u64
    const ESEQUENCE_NUMBER_TOO_BIG: u64 = 1;
    /// The address provided didn't match the `AptosFramework` address.
    const ENOT_APTOS_FRAMEWORK: u64 = 2;
    /// The provided authentication had an invalid length
    const EMALFORMED_AUTHENTICATION_KEY: u64 = 3;

    const ECANNOT_CREATE_AT_VM_RESERVED: u64 = 4;
    const EGAS: u64 = 5;
    const ECANNOT_CREATE_AT_CORE_CODE: u64 = 6;
    const EADDR_NOT_MATCH_PREIMAGE: u64 = 7;
    const EWRITESET_NOT_ALLOWED: u64 = 8;
    const EMULTI_AGENT_NOT_SUPPORTED: u64 = 9;
    const EMODULE_NOT_ALLOWED: u64 = 10;
    const ESCRIPT_NOT_ALLOWED: u64 = 11;

    /// Prologue errors. These are separated out from the other errors in this
    /// module since they are mapped separately to major VM statuses, and are
    /// important to the semantics of the system.
    const PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY: u64 = 1001;
    const PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD: u64 = 1002;
    const PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW: u64 = 1003;
    const PROLOGUE_EACCOUNT_DNE: u64 = 1004;
    const PROLOGUE_ECANT_PAY_GAS_DEPOSIT: u64 = 1005;
    const PROLOGUE_ETRANSACTION_EXPIRED: u64 = 1006;
    const PROLOGUE_EBAD_CHAIN_ID: u64 = 1007;
    const PROLOGUE_ESCRIPT_NOT_ALLOWED: u64 = 1008;
    const PROLOGUE_EMODULE_NOT_ALLOWED: u64 = 1009;
    const PROLOGUE_EINVALID_WRITESET_SENDER: u64 = 1010;
    const PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG: u64 = 1011;
    const PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH: u64 = 1012;

    native fun create_signer(addr: address): signer;

    public fun initialize(account: &signer,
        module_addr: address,
        module_name: vector<u8>,
        script_prologue_name: vector<u8>,
        module_prologue_name: vector<u8>,
        writeset_prologue_name: vector<u8>,
        multi_agent_prologue_name: vector<u8>,
        user_epilogue_name: vector<u8>,
        writeset_epilogue_name: vector<u8>,
        currency_code_required: bool,
    ) {
        assert!(Signer::address_of(account) == @CoreResources, Errors::requires_address(ENOT_APTOS_FRAMEWORK));
        move_to(account, ChainSpecificAccountInfo {
            module_addr,
            module_name,
            script_prologue_name,
            module_prologue_name,
            writeset_prologue_name,
            multi_agent_prologue_name,
            user_epilogue_name,
            writeset_epilogue_name,
            currency_code_required,
        });
    }

    /// Construct an authentication key, aborting if the prefix is not valid.
    fun create_authentication_key(account: &signer, auth_key_prefix: vector<u8>): vector<u8> {
        let authentication_key = auth_key_prefix;
        Vector::append(
            &mut authentication_key, BCS::to_bytes(Signer::borrow_address(account))
        );
        assert!(
            Vector::length(&authentication_key) == 32,
            Errors::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        authentication_key
    }

    /// Publishes a new `Account` resource under `new_address`.
    /// A signer representing `new_address` is returned. This way, the caller of this function
    /// can publish additional resources under `new_address`.
    /// The `_witness` guarantees that owner the registered caller of this function can call it.
    /// authentication key returned is `auth_key_prefix` | `fresh_address`.
    public fun create_account_internal(
        new_address: address,
    ): (signer, vector<u8>) {
        // there cannot be an Account resource under new_addr already.
        assert!(!exists<Account>(new_address), Errors::already_published(EACCOUNT));
        assert!(
            new_address != @VMReserved,
            Errors::invalid_argument(ECANNOT_CREATE_AT_VM_RESERVED)
        );
        assert!(
            new_address != @AptosFramework,
            Errors::invalid_argument(ECANNOT_CREATE_AT_CORE_CODE)
        );

        create_account_unchecked(new_address)
    }

    fun create_account_unchecked(new_address: address): (signer, vector<u8>) {
        let new_account = create_signer(new_address);
        let authentication_key = BCS::to_bytes(&new_address);
        assert!(
            Vector::length(&authentication_key) == 32,
            Errors::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        move_to(
            &new_account,
            Account {
                authentication_key: copy authentication_key,
                sequence_number: 0,
                self_address: new_address,
            }
        );

        (new_account, authentication_key)
    }

    public fun exists_at(addr: address): bool {
        exists<Account>(addr)
    }

    public fun get_sequence_number(addr: address) : u64 acquires Account {
        borrow_global<Account>(addr).sequence_number
    }

    public fun get_authentication_key(addr: address) : vector<u8> acquires Account {
        *&borrow_global<Account>(addr).authentication_key
    }

    public(script) fun rotate_authentication_key(account: signer, new_auth_key: vector<u8>) acquires Account {
        rotate_authentication_key_internal(&account, new_auth_key);
    }

    public fun rotate_authentication_key_internal(
        account: &signer,
        new_auth_key: vector<u8>,
    ) acquires Account {
        let addr = Signer::address_of(account);
        assert!(exists_at(addr), Errors::not_published(EACCOUNT));
        assert!(
            Vector::length(&new_auth_key) == 32,
            Errors::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.authentication_key = new_auth_key;
    }

    fun prologue_common(
        sender: signer,
        txn_sequence_number: u64,
        txn_public_key: vector<u8>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) acquires Account {
        assert!(
            Timestamp::now_seconds() < txn_expiration_time,
            Errors::invalid_argument(PROLOGUE_ETRANSACTION_EXPIRED),
        );
        let transaction_sender = Signer::address_of(&sender);
        assert!(ChainId::get() == chain_id, Errors::invalid_argument(PROLOGUE_EBAD_CHAIN_ID));
        assert!(exists<Account>(transaction_sender), Errors::invalid_argument(PROLOGUE_EACCOUNT_DNE));
        let sender_account = borrow_global<Account>(transaction_sender);
        assert!(
            Hash::sha3_256(txn_public_key) == *&sender_account.authentication_key,
            Errors::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
        );
        assert!(
            (txn_sequence_number as u128) < MAX_U64,
            Errors::limit_exceeded(PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG)
        );

        assert!(
            txn_sequence_number >= sender_account.sequence_number,
            Errors::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD)
        );

        // [PCA12]: Check that the transaction's sequence number matches the
        // current sequence number. Otherwise sequence number is too new by [PCA11].
        assert!(
            txn_sequence_number == sender_account.sequence_number,
            Errors::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)
        );
        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        assert!(TestCoin::exists_at(transaction_sender), Errors::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
        let balance = TestCoin::balance_of(transaction_sender);
        assert!(balance >= max_transaction_fee, Errors::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
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
    ) acquires Account {
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
    ) acquires Account {
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
        sender: signer,
        txn_sequence_number: u64,
        txn_sender_public_key: vector<u8>,
        secondary_signer_addresses: vector<address>,
        secondary_signer_public_key_hashes: vector<vector<u8>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
    ) acquires Account {
        prologue_common(sender, txn_sequence_number, txn_sender_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id);

        let num_secondary_signers = Vector::length(&secondary_signer_addresses);

        assert!(
            Vector::length(&secondary_signer_public_key_hashes) == num_secondary_signers,
            Errors::invalid_argument(PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH),
        );

        let i = 0;
        while (i < num_secondary_signers) {
            let secondary_address = *Vector::borrow(&secondary_signer_addresses, i);
            assert!(exists_at(secondary_address), Errors::invalid_argument(PROLOGUE_EACCOUNT_DNE));

            let signer_account = borrow_global<Account>(secondary_address);
            let signer_public_key_hash = *Vector::borrow(&secondary_signer_public_key_hashes, i);
            assert!(
                signer_public_key_hash == *&signer_account.authentication_key,
                Errors::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
            );
            i = i + 1;
        }
    }

    fun writeset_epilogue(
        _core_resource: signer,
        _txn_sequence_number: u64,
        _should_trigger_reconfiguration: bool,
    ) {
        assert!(false, Errors::invalid_argument(EWRITESET_NOT_ALLOWED));
    }

    /// Epilogue function is run after a transaction is successfully executed.
    /// Called by the Adapter
    fun epilogue(
        account: signer,
        _txn_sequence_number: u64,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64
    ) acquires Account {
        assert!(txn_max_gas_units >= gas_units_remaining, Errors::invalid_argument(EGAS));
        let gas_used = txn_max_gas_units - gas_units_remaining;

        assert!(
            (txn_gas_price as u128) * (gas_used as u128) <= MAX_U64,
            Errors::limit_exceeded(EGAS)
        );
        let transaction_fee_amount = txn_gas_price * gas_used;
        let addr = Signer::address_of(&account);
        // it's important to maintain the error code consistent with vm
        // to do failed transaction cleanup.
        assert!(TestCoin::balance_of(addr) >= transaction_fee_amount, Errors::limit_exceeded(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
        let coin = TestCoin::withdraw(&account, transaction_fee_amount);
        TransactionFee::burn_fee(coin);

        let old_sequence_number = get_sequence_number(addr);

        assert!(
            (old_sequence_number as u128) < MAX_U64,
            Errors::limit_exceeded(ESEQUENCE_NUMBER_TOO_BIG)
        );

        // Increment sequence number
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.sequence_number = old_sequence_number + 1;
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Basic account creation method.
    ///////////////////////////////////////////////////////////////////////////

    public(script) fun create_account(auth_key: address) {
        let (signer, _) = create_account_internal(auth_key);
        TestCoin::register(&signer);
    }

    /// Create the account for @AptosFramework to help module upgrades on testnet.
    public(friend) fun create_core_framework_account(): signer {
        Timestamp::assert_genesis();
        let (signer, _) = create_account_unchecked(@AptosFramework);
        signer
    }
}
