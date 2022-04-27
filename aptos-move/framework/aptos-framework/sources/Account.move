module AptosFramework::Account {
    use Std::BCS;
    use Std::Errors;
    use Std::Event::{Self, EventHandle};
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
    struct Account has key {
        authentication_key: vector<u8>,
        sequence_number: u64,
        self_address: address,
        balance: TestCoin::Coin,
        transfer_events: TransferEvents,
    }

    /// Events handles.
    struct TransferEvents has store {
        sent_events: EventHandle<SentEvent>,
        received_events: EventHandle<ReceivedEvent>,
    }

    struct SentEvent has drop, store {
        amount: u64,
        to: address,
    }

    struct ReceivedEvent has drop, store {
        amount: u64,
        from: address,
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

    /// Create the account for @AptosFramework to help module upgrades on testnet.
    public(friend) fun create_core_framework_account(): signer {
        Timestamp::assert_genesis();
        let (signer, _) = create_account_unchecked(@AptosFramework);
        signer
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
                balance: TestCoin::zero(),
                transfer_events: TransferEvents {
                    sent_events: Event::new_event_handle<SentEvent>(&new_account),
                    received_events: Event::new_event_handle<ReceivedEvent>(&new_account),
                }
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

    public fun get_balance(addr: address): u64 acquires Account {
        TestCoin::value(&borrow_global<Account>(addr).balance)
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
        let balance = get_balance(transaction_sender);
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
        let coin = withdraw_from(&account, transaction_fee_amount);
        TransactionFee::burn_fee(coin);

        let addr = Signer::address_of(&account);
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
    /// Coin related functions.
    ///////////////////////////////////////////////////////////////////////////

    public fun withdraw_from(from: &signer, amount: u64): TestCoin::Coin acquires Account {
        let account = borrow_global_mut<Account>(Signer::address_of(from));
        TestCoin::split(&mut account.balance, amount)
    }

    public fun deposit_to(to: address, coins: TestCoin::Coin) acquires Account {
        let account = borrow_global_mut<Account>(to);
        TestCoin::merge(&mut account.balance, coins);
    }

    /// Mint coins if the account has MintCapability.
    public fun mint_internal(account: &signer, mint_addr: address, amount: u64) acquires Account {
        let coins = TestCoin::mint(account, amount);
        deposit_to(mint_addr, coins);
    }

    ///////////////////////////////////////////////////////////////////////////
    /// Script functions.
    ///////////////////////////////////////////////////////////////////////////

    public(script) fun create_account(auth_key: address) {
        create_account_internal(auth_key);
    }

    public(script) fun rotate_authentication_key(account: signer, new_auth_key: vector<u8>) acquires Account {
        rotate_authentication_key_internal(&account, new_auth_key);
    }

    /// Transfers `amount` of coins from `from` to `to`.
    public(script) fun transfer(from: &signer, to: address, amount: u64) acquires Account {
        let check = withdraw_from(from, amount);
        deposit_to(to, check);
        // emit events
        let sender_handle = &mut borrow_global_mut<Account>(Signer::address_of(from)).transfer_events;
        Event::emit_event<SentEvent>(
            &mut sender_handle.sent_events,
            SentEvent { amount, to },
        );
        let receiver_handle = &mut borrow_global_mut<Account>(to).transfer_events;
        Event::emit_event<ReceivedEvent>(
            &mut receiver_handle.received_events,
            ReceivedEvent { amount, from: Signer::address_of(from) },
        );
    }

    /// Mint coins if the account has MintCapability.
    public(script) fun mint(account: signer, mint_addr: address, amount: u64) acquires Account {
        mint_internal(&account, mint_addr, amount);
    }

    #[test(account = @0x123)]
    fun zero_balance(account: signer) acquires Account {
        let addr = Signer::address_of(&account);
        create_account_internal(addr);
        assert!(get_balance(addr) == 0, 0);
    }

    #[test(account = @0x123)]
    #[expected_failure(abort_code = 6)] // Can specify an abort code
    fun double_creation(account: signer) {
        let addr = Signer::address_of(&account);
        create_account_internal(addr);
        create_account_internal(addr);
    }

    #[test]
    #[expected_failure]
    fun balance_of_dne() acquires Account {
        get_balance(@0x1);
    }

    #[test(account = @CoreResources, receiver = @0x123)]
    public(script) fun test_transfer(
        account: signer,
        receiver: signer,
    ) acquires Account {
        TestCoin::initialize(&account, 1000000);
        let amount = 1000;
        let addr = Signer::address_of(&account);
        let addr1 = Signer::address_of(&receiver);
        create_account_internal(addr);
        create_account_internal(addr1);
        mint_internal(&account, addr, amount);

        transfer(&account, addr1, 400);
        assert!(get_balance(addr) == 600, 0);
        assert!(get_balance(addr1) == 400, 0);
    }

    #[test_only]
    public fun create_and_mint_for_test(account: &signer, amount: u64) acquires Account {
        let addr = Signer::address_of(account);
        create_account_internal(addr);

        deposit_to(addr, TestCoin::mint_for_test(amount));
    }
}
