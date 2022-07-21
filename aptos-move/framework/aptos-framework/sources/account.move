module aptos_framework::account {
    use std::bcs;
    use std::error;
    use std::hash;
    use std::signer;
    use std::vector;
    use aptos_framework::chain_id;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::timestamp;
    use aptos_framework::transaction_fee;
    use aptos_framework::transaction_publishing_option;
    use aptos_framework::system_addresses;

    friend aptos_framework::genesis;

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

    struct SignerCapability has drop, store { account: address }

    const MAX_U64: u128 = 18446744073709551615;

    /// Account already existed
    const EACCOUNT: u64 = 0;
    /// Sequence number exceeded the maximum value for a u64
    const ESEQUENCE_NUMBER_TOO_BIG: u64 = 1;
    /// The address provided didn't match the `aptos_framework` address.
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

    #[test_only]
    public fun create_address_for_test(bytes: vector<u8>): address {
        create_address(bytes)
    }

    native fun create_address(bytes: vector<u8>): address;
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
        system_addresses::assert_aptos_framework(account);

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
        vector::append(
            &mut authentication_key, bcs::to_bytes(signer::borrow_address(account))
        );
        assert!(
            vector::length(&authentication_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        authentication_key
    }

    /// Publishes a new `Account` resource under `new_address`. A signer representing `new_address`
    /// is returned. This way, the caller of this function can publish additional resources under
    /// `new_address`.
    public(friend) fun create_account_internal(new_address: address): signer {
        // there cannot be an Account resource under new_addr already.
        assert!(!exists<Account>(new_address), error::already_exists(EACCOUNT));
        assert!(
            new_address != @vm_reserved,
            error::invalid_argument(ECANNOT_CREATE_AT_VM_RESERVED)
        );
        assert!(
            new_address != @aptos_framework,
            error::invalid_argument(ECANNOT_CREATE_AT_CORE_CODE)
        );

        create_account_unchecked(new_address)
    }

    fun create_account_unchecked(new_address: address): signer {
        let new_account = create_signer(new_address);
        let authentication_key = bcs::to_bytes(&new_address);
        assert!(
            vector::length(&authentication_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
        );
        move_to(
            &new_account,
            Account {
                authentication_key,
                sequence_number: 0,
                self_address: new_address,
            }
        );

        new_account
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

    public entry fun rotate_authentication_key(account: signer, new_auth_key: vector<u8>) acquires Account {
        rotate_authentication_key_internal(&account, new_auth_key);
    }

    public fun rotate_authentication_key_internal(
        account: &signer,
        new_auth_key: vector<u8>,
    ) acquires Account {
        let addr = signer::address_of(account);
        assert!(exists_at(addr), error::not_found(EACCOUNT));
        assert!(
            vector::length(&new_auth_key) == 32,
            error::invalid_argument(EMALFORMED_AUTHENTICATION_KEY)
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
            timestamp::now_seconds() < txn_expiration_time,
            error::invalid_argument(PROLOGUE_ETRANSACTION_EXPIRED),
        );
        let transaction_sender = signer::address_of(&sender);
        assert!(chain_id::get() == chain_id, error::invalid_argument(PROLOGUE_EBAD_CHAIN_ID));
        assert!(exists<Account>(transaction_sender), error::invalid_argument(PROLOGUE_EACCOUNT_DNE));
        let sender_account = borrow_global<Account>(transaction_sender);
        assert!(
            hash::sha3_256(txn_public_key) == *&sender_account.authentication_key,
            error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
        );
        assert!(
            (txn_sequence_number as u128) < MAX_U64,
            error::out_of_range(PROLOGUE_ESEQUENCE_NUMBER_TOO_BIG)
        );

        assert!(
            txn_sequence_number >= sender_account.sequence_number,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_OLD)
        );

        // [PCA12]: Check that the transaction's sequence number matches the
        // current sequence number. Otherwise sequence number is too new by [PCA11].
        assert!(
            txn_sequence_number == sender_account.sequence_number,
            error::invalid_argument(PROLOGUE_ESEQUENCE_NUMBER_TOO_NEW)
        );
        let max_transaction_fee = txn_gas_price * txn_max_gas_units;
        assert!(
            coin::is_account_registered<AptosCoin>(transaction_sender),
            error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),
        );
        let balance = coin::balance<AptosCoin>(transaction_sender);
        assert!(balance >= max_transaction_fee, error::invalid_argument(PROLOGUE_ECANT_PAY_GAS_DEPOSIT));
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
        assert!(transaction_publishing_option::is_module_allowed(), error::invalid_state(PROLOGUE_EMODULE_NOT_ALLOWED));
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
        assert!(transaction_publishing_option::is_script_allowed(&script_hash), error::invalid_state(PROLOGUE_ESCRIPT_NOT_ALLOWED));
        prologue_common(sender, txn_sequence_number, txn_public_key, txn_gas_price, txn_max_gas_units, txn_expiration_time, chain_id)
    }

    fun writeset_prologue(
        _sender: signer,
        _txn_sequence_number: u64,
        _txn_public_key: vector<u8>,
        _txn_expiration_time: u64,
        _chain_id: u8,
    ) {
        assert!(false, error::invalid_argument(PROLOGUE_EINVALID_WRITESET_SENDER));
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

        let num_secondary_signers = vector::length(&secondary_signer_addresses);

        assert!(
            vector::length(&secondary_signer_public_key_hashes) == num_secondary_signers,
            error::invalid_argument(PROLOGUE_ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH),
        );

        let i = 0;
        while (i < num_secondary_signers) {
            let secondary_address = *vector::borrow(&secondary_signer_addresses, i);
            assert!(exists_at(secondary_address), error::invalid_argument(PROLOGUE_EACCOUNT_DNE));

            let signer_account = borrow_global<Account>(secondary_address);
            let signer_public_key_hash = *vector::borrow(&secondary_signer_public_key_hashes, i);
            assert!(
                signer_public_key_hash == *&signer_account.authentication_key,
                error::invalid_argument(PROLOGUE_EINVALID_ACCOUNT_AUTH_KEY),
            );
            i = i + 1;
        }
    }

    fun writeset_epilogue(
        _core_resource: signer,
        _txn_sequence_number: u64,
        _should_trigger_reconfiguration: bool,
    ) {
        assert!(false, error::invalid_argument(EWRITESET_NOT_ALLOWED));
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
        assert!(txn_max_gas_units >= gas_units_remaining, error::invalid_argument(EGAS));
        let gas_used = txn_max_gas_units - gas_units_remaining;

        assert!(
            (txn_gas_price as u128) * (gas_used as u128) <= MAX_U64,
            error::out_of_range(EGAS)
        );
        let transaction_fee_amount = txn_gas_price * gas_used;
        let addr = signer::address_of(&account);
        // it's important to maintain the error code consistent with vm
        // to do failed transaction cleanup.
        assert!(
            coin::balance<AptosCoin>(addr) >= transaction_fee_amount,
            error::out_of_range(PROLOGUE_ECANT_PAY_GAS_DEPOSIT),
        );
        transaction_fee::burn_fee(addr, transaction_fee_amount);

        let old_sequence_number = get_sequence_number(addr);

        assert!(
            (old_sequence_number as u128) < MAX_U64,
            error::out_of_range(ESEQUENCE_NUMBER_TOO_BIG)
        );

        // Increment sequence number
        let account_resource = borrow_global_mut<Account>(addr);
        account_resource.sequence_number = old_sequence_number + 1;
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Basic account creation methods.
    ///////////////////////////////////////////////////////////////////////////

    public entry fun create_account(auth_key: address) {
        let signer = create_account_internal(auth_key);
        coin::register<AptosCoin>(&signer);
    }

    /// A resource account is used to manage resources independent of an account managed by a user.
    public fun create_resource_account(
        source: &signer,
        seed: vector<u8>,
    ): (signer, SignerCapability) {
        let bytes = bcs::to_bytes(&signer::address_of(source));
        vector::append(&mut bytes, seed);
        let addr = create_address(hash::sha3_256(bytes));

        let signer = create_account_internal(copy addr);
        let signer_cap = SignerCapability { account: addr };
        (signer, signer_cap)
    }

    /// Create the account for @aptos_framework to help module upgrades on testnet.
    public(friend) fun create_core_framework_account(): (signer, SignerCapability) {
        timestamp::assert_genesis();
        let signer = create_account_unchecked(@aptos_framework);
        let signer_cap = SignerCapability { account: @aptos_framework };
        (signer, signer_cap)
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Capability based functions for efficient use.
    ///////////////////////////////////////////////////////////////////////////

    public fun create_signer_with_capability(capability: &SignerCapability): signer {
        let addr = &capability.account;
        create_signer(*addr)
    }

    #[test(user = @0x1)]
    public entry fun test_create_resource_account(user: signer) {
        let (resource_account, _) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);
        coin::register<AptosCoin>(&resource_account);
    }

    #[test_only]
    struct DummyResource has key { }

    #[test(user = @0x1)]
    public entry fun test_module_capability(user: signer) acquires DummyResource {
        let (resource_account, signer_cap) = create_resource_account(&user, x"01");
        assert!(signer::address_of(&resource_account) != signer::address_of(&user), 0);

        let resource_account_from_cap = create_signer_with_capability(&signer_cap);
        assert!(&resource_account == &resource_account_from_cap, 1);
        coin::register<AptosCoin>(&resource_account_from_cap);

        move_to(&resource_account_from_cap, DummyResource { });
        borrow_global<DummyResource>(signer::address_of(&resource_account));
    }

    ///////////////////////////////////////////////////////////////////////////
    // Test-only sequence number mocking for extant Account resource
    ///////////////////////////////////////////////////////////////////////////

    #[test_only]
    /// Increment sequence number of account at address `addr`
    public fun increment_sequence_number(
        addr: address,
    ) acquires Account {
        let acct = borrow_global_mut<Account>(addr);
        acct.sequence_number = acct.sequence_number + 1;
    }

    #[test_only]
    /// Update address `addr` to have `s` as its sequence number
    public fun set_sequence_number(
        addr: address,
        s: u64
    ) acquires Account {
        borrow_global_mut<Account>(addr).sequence_number = s;
    }

    #[test]
    /// Verify test-only sequence number mocking
    public entry fun mock_sequence_numbers()
    acquires Account {
        let addr: address = @0x1234; // Define test address
        create_account(addr); // Initialize account resource
        // Assert sequence number intializes to 0
        assert!(borrow_global<Account>(addr).sequence_number == 0, 0);
        increment_sequence_number(addr); // Increment sequence number
        // Assert correct mock value post-increment
        assert!(borrow_global<Account>(addr).sequence_number == 1, 1);
        set_sequence_number(addr, 10); // Set mock sequence number
        // Assert correct mock value post-modification
        assert!(borrow_global<Account>(addr).sequence_number == 10, 2);
    }

}
