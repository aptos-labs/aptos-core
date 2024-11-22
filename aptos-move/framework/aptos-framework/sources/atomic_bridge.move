module aptos_framework::ethereum {
    use std::vector;
    use aptos_std::aptos_hash::keccak256;

    /// Constants for ASCII character codes
    const ASCII_A: u8 = 0x41;
    const ASCII_Z: u8 = 0x5A;
    const ASCII_A_LOWERCASE: u8 = 0x61;
    const ASCII_F_LOWERCASE: u8 = 0x66;

    /// Represents an Ethereum address within Aptos smart contracts.
    /// Provides structured handling, storage, and validation of Ethereum addresses.
    struct EthereumAddress has store, copy, drop {
        inner: vector<u8>,
    }

    /// Validates an Ethereum address against EIP-55 checksum rules and returns a new `EthereumAddress`.
    ///
    /// @param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
    /// @return A validated `EthereumAddress` struct.
    /// @abort If the address does not conform to EIP-55 standards.
    public fun ethereum_address(ethereum_address: vector<u8>): EthereumAddress {
        assert_eip55(&ethereum_address);
        EthereumAddress { inner: ethereum_address }
    }

    /// Returns a new `EthereumAddress` without EIP-55 validation.
    ///
    /// @param ethereum_address A 40-byte vector of unsigned 8-bit integers (hexadecimal format).
    /// @return A validated `EthereumAddress` struct.
    /// @abort If the address does not conform to EIP-55 standards.
    public fun ethereum_address_no_eip55(ethereum_address: vector<u8>): EthereumAddress {
        assert_40_char_hex(&ethereum_address);
        EthereumAddress { inner: ethereum_address }
    }

    /// Converts uppercase ASCII characters in a vector to their lowercase equivalents.
    ///
    /// @param input A reference to a vector of ASCII characters.
    /// @return A new vector with lowercase equivalents of the input characters.
    /// @note Only affects ASCII letters; non-alphabetic characters are unchanged.
    public fun to_lowercase(input: &vector<u8>): vector<u8> {
        let lowercase_bytes = vector::empty();
        vector::enumerate_ref(input, |_i, element| {
            let lower_byte = if (*element >= ASCII_A && *element <= ASCII_Z) {
                *element + 32
            } else {
                *element
            };
            vector::push_back<u8>(&mut lowercase_bytes, lower_byte);
        });
        lowercase_bytes
    }

    #[test]
    fun test_to_lowercase() {
        let upper = b"TeST";
        let lower = b"test";
        assert!(to_lowercase(&upper) == lower, 0);
    }

    /// Converts an Ethereum address to EIP-55 checksummed format.
    ///
    /// @param ethereum_address A 40-character vector representing the Ethereum address in hexadecimal format.
    /// @return The EIP-55 checksummed version of the input address.
    /// @abort If the input address does not have exactly 40 characters.
    /// @note Assumes input address is valid and in lowercase hexadecimal format.
    public fun to_eip55_checksumed_address(ethereum_address: &vector<u8>): vector<u8> {
        assert!(vector::length(ethereum_address) == 40, 0);
        let lowercase = to_lowercase(ethereum_address);
        let hash = keccak256(lowercase);
        let output = vector::empty<u8>();

        for (index in 0..40) {
            let item = *vector::borrow(ethereum_address, index);
            if (item >= ASCII_A_LOWERCASE && item <= ASCII_F_LOWERCASE) {
                let hash_item = *vector::borrow(&hash, index / 2);
                if ((hash_item >> ((4 * (1 - (index % 2))) as u8)) & 0xF >= 8) {
                    vector::push_back(&mut output, item - 32);
                } else {
                    vector::push_back(&mut output, item);
                }
            } else {
                vector::push_back(&mut output, item);
            }
        };
        output
    }

    public fun get_inner(eth_address: &EthereumAddress): vector<u8> {
        eth_address.inner
    }

    /// Checks if an Ethereum address conforms to the EIP-55 checksum standard.
    ///
    /// @param ethereum_address A reference to a 40-character vector of an Ethereum address in hexadecimal format.
    /// @abort If the address does not match its EIP-55 checksummed version.
    /// @note Assumes the address is correctly formatted as a 40-character hexadecimal string.
    public fun assert_eip55(ethereum_address: &vector<u8>) {
        let eip55 = to_eip55_checksumed_address(ethereum_address);
        let len = vector::length(&eip55);
        for (index in 0..len) {
            assert!(vector::borrow(&eip55, index) == vector::borrow(ethereum_address, index), 0);
        };
    }

    /// Checks if an Ethereum address is a nonzero 40-character hexadecimal string.
    ///
    /// @param ethereum_address A reference to a vector of bytes representing the Ethereum address as characters.
    /// @abort If the address is not 40 characters long, contains invalid characters, or is all zeros.
    public fun assert_40_char_hex(ethereum_address: &vector<u8>) {
        let len = vector::length(ethereum_address);

        // Ensure the address is exactly 40 characters long
        assert!(len == 40, 1);

        // Ensure the address contains only valid hexadecimal characters
        let is_zero = true;
        for (index in 0..len) {
            let char = *vector::borrow(ethereum_address, index);

            // Check if the character is a valid hexadecimal character (0-9, a-f, A-F)
            assert!(
                (char >= 0x30 && char <= 0x39) || // '0' to '9'
                (char >= 0x41 && char <= 0x46) || // 'A' to 'F'
                (char >= 0x61 && char <= 0x66),  // 'a' to 'f'
                2
            );

            // Check if the address is nonzero
            if (char != 0x30) { // '0'
                is_zero = false;
            };
        };

        // Abort if the address is all zeros
        assert!(!is_zero, 3);
    }

    #[test_only]
    public fun valid_eip55(): vector<u8> {
        b"32Be343B94f860124dC4fEe278FDCBD38C102D88"
    }

    #[test_only]
    public fun invalid_eip55(): vector<u8> {
        b"32be343b94f860124dc4fee278fdcbd38c102d88"
    }

    #[test]
    fun test_valid_eip55_checksum() {
        assert_eip55(&valid_eip55());
    }

    #[test]
    #[expected_failure(abort_code = 0, location = Self)]
    fun test_invalid_eip55_checksum() {
        assert_eip55(&invalid_eip55());
    }

    #[test]
    #[expected_failure(abort_code = 0, location = Self)]
    fun test_simple_invalid_eip55_checksum() {
        assert_eip55(&b"0");
    }
}

module aptos_framework::atomic_bridge_initiator {
    use aptos_framework::account;
    use aptos_framework::atomic_bridge;
    use aptos_framework::atomic_bridge_configuration;
    use aptos_framework::atomic_bridge_configuration::assert_is_caller_operator;
    use aptos_framework::atomic_bridge_store;
    use aptos_framework::atomic_bridge_store::{create_hashlock, bridge_transfer_id};
    use aptos_framework::ethereum;
    use aptos_framework::ethereum::EthereumAddress;
    use aptos_framework::event::{Self, EventHandle}; 
    use aptos_framework::signer;
    #[test_only]
    use std::vector;
    #[test_only]
    use aptos_framework::aptos_account;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_framework::atomic_bridge_store::{valid_hash_lock, assert_valid_bridge_transfer_id, plain_secret};
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::ethereum::valid_eip55;
    #[test_only]
    use aptos_framework::timestamp;

    #[event]
    struct BridgeTransferInitiatedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        initiator: address,
        recipient: vector<u8>,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    #[event]
    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    }

    #[event]
    struct BridgeTransferRefundedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
    }

    /// This struct will store the event handles for bridge events.
    struct BridgeInitiatorEvents has key, store {
        bridge_transfer_initiated_events: EventHandle<BridgeTransferInitiatedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_refunded_events: EventHandle<BridgeTransferRefundedEvent>,
    }

    /// Initializes the module and stores the `EventHandle`s in the resource.
    public fun initialize(aptos_framework: &signer) {
        move_to(aptos_framework, BridgeInitiatorEvents {
            bridge_transfer_initiated_events: account::new_event_handle<BridgeTransferInitiatedEvent>(aptos_framework),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(aptos_framework),
            bridge_transfer_refunded_events: account::new_event_handle<BridgeTransferRefundedEvent>(aptos_framework),
        });
    }

    /// Initiate a bridge transfer of ETH from Movement to the base layer
    /// Anyone can initiate a bridge transfer from the source chain
    /// The amount is burnt from the initiator
    public entry fun initiate_bridge_transfer(
        initiator: &signer,
        recipient: vector<u8>,
        hash_lock: vector<u8>,
        amount: u64
    ) acquires BridgeInitiatorEvents {
        let ethereum_address = ethereum::ethereum_address_no_eip55(recipient);
        let initiator_address = signer::address_of(initiator);
        let time_lock = atomic_bridge_configuration::initiator_timelock_duration();

        let details =
            atomic_bridge_store::create_details(
                initiator_address,
                ethereum_address, amount,
                hash_lock,
                time_lock
            );

        let bridge_transfer_id = bridge_transfer_id(&details);
        atomic_bridge_store::add(bridge_transfer_id, details);
        atomic_bridge::burn(initiator_address, amount);

        let bridge_initiator_events = borrow_global_mut<BridgeInitiatorEvents>(@aptos_framework);
        event::emit_event(
            &mut bridge_initiator_events.bridge_transfer_initiated_events,
            BridgeTransferInitiatedEvent {
                bridge_transfer_id,
                initiator: initiator_address,
                recipient,
                amount,
                hash_lock,
                time_lock
            },
        );
    }

    /// Bridge operator can complete the transfer
    public entry fun complete_bridge_transfer (
        caller: &signer,
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeInitiatorEvents {
        assert_is_caller_operator(caller);
        let (_, _) = atomic_bridge_store::complete_transfer<address, EthereumAddress>(bridge_transfer_id, create_hashlock(pre_image));

        let bridge_initiator_events = borrow_global_mut<BridgeInitiatorEvents>(@aptos_framework);
        event::emit_event(
            &mut bridge_initiator_events.bridge_transfer_completed_events,
            BridgeTransferCompletedEvent {
                bridge_transfer_id,
                pre_image,
            },
        );
    }

    /// Anyone can refund the transfer on the source chain once time lock has passed
    public entry fun refund_bridge_transfer (
        _caller: &signer,
        bridge_transfer_id: vector<u8>,
    ) acquires BridgeInitiatorEvents {
        let (receiver, amount) = atomic_bridge_store::cancel_transfer<address, EthereumAddress>(bridge_transfer_id);
        atomic_bridge::mint(receiver, amount);

        let bridge_initiator_events = borrow_global_mut<BridgeInitiatorEvents>(@aptos_framework);
        event::emit_event(
            &mut bridge_initiator_events.bridge_transfer_refunded_events,
            BridgeTransferRefundedEvent {
                bridge_transfer_id,
            },
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    fun test_initiate_bridge_transfer(
        sender: &signer,
        aptos_framework: &signer,
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);
        initialize(aptos_framework);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let time_lock = atomic_bridge_configuration::initiator_timelock_duration();
        let amount = 1000;

        // Mint some coins
        atomic_bridge::mint(sender_address, amount + 1);

        assert!(coin::balance<AptosCoin>(sender_address) == amount + 1, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        assert!(coin::balance<AptosCoin>(sender_address) == 1, 0);

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        assert_valid_bridge_transfer_id(&bridge_transfer_initiated_event.bridge_transfer_id);
        assert!(bridge_transfer_initiated_event.recipient == recipient, 0);
        assert!(bridge_transfer_initiated_event.amount == amount, 0);
        assert!(bridge_transfer_initiated_event.initiator == sender_address, 0);
        assert!(bridge_transfer_initiated_event.hash_lock == hash_lock, 0);
        assert!(bridge_transfer_initiated_event.time_lock == time_lock, 0);
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 0x10006, location = 0x1::coin)] //EINSUFFICIENT_BALANCE
    fun test_initiate_bridge_transfer_insufficient_balance(
        sender: &signer,
        aptos_framework: &signer,
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let amount = 1000;

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    fun test_complete_bridge_transfer(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let amount = 1000;

        let account_balance = amount + 1;

        // Mint some coins
        atomic_bridge::mint(sender_address, account_balance);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        let bridge_transfer_id = bridge_transfer_initiated_event.bridge_transfer_id;

        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id,
            plain_secret(),
        );

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(signer::address_of(aptos_framework));
        let complete_events = event::emitted_events_by_handle(&bridge_initiator_events.bridge_transfer_completed_events);
        let expected_event = BridgeTransferCompletedEvent {
            bridge_transfer_id,
            pre_image: plain_secret(),
        };
        assert!(std::vector::contains(&complete_events, &expected_event), 0);

    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 0x1, location = 0x1::atomic_bridge_configuration)] // EINVALID_BRIDGE_OPERATOR
    fun test_complete_bridge_transfer_by_sender(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let amount = 1000;
        let account_balance = amount + 1;

        // Mint some coins
        atomic_bridge::mint(sender_address, account_balance);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        let bridge_transfer_id = bridge_transfer_initiated_event.bridge_transfer_id;

        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            plain_secret(),
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 0x1, location = 0x1::atomic_bridge_store)] // EINVALID_PRE_IMAGE
    fun test_complete_bridge_transfer_with_invalid_preimage(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let amount = 1000;
        let account_balance = amount + 1;

        // Mint some coins
        atomic_bridge::mint(sender_address, account_balance);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        let bridge_transfer_id = bridge_transfer_initiated_event.bridge_transfer_id;

        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id,
            b"bad secret",
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 0x10001, location = 0x1::smart_table)] // ENOT_FOUND
    fun test_complete_bridge_with_errorneous_bridge_id_by_operator(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);

        let bridge_transfer_id = b"guessing the id";

        // As operator I send a complete request and it should fail
        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id,
            plain_secret(),
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    fun test_refund_bridge_transfer(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let amount = 1000;

        let account_balance = amount + 1;
        // Mint some coins
        atomic_bridge::mint(sender_address, account_balance);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance - amount, 0);

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        let bridge_transfer_id = bridge_transfer_initiated_event.bridge_transfer_id;

        timestamp::fast_forward_seconds(atomic_bridge_configuration::initiator_timelock_duration() + 1);

        refund_bridge_transfer(sender, bridge_transfer_id);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(signer::address_of(aptos_framework));
        let refund_events = event::emitted_events_by_handle(&bridge_initiator_events.bridge_transfer_refunded_events);
        let expected_event = BridgeTransferRefundedEvent { bridge_transfer_id };
        let was_event_emitted = std::vector::contains(&refund_events, &expected_event);
        
        assert!(was_event_emitted, 0);
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 0x4, location = 0x1::atomic_bridge_store)] //ENOT_EXPIRED
    fun test_refund_bridge_transfer_before_timelock(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeInitiatorEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        atomic_bridge::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let hash_lock = valid_hash_lock();
        let amount = 1000;

        let account_balance = amount + 1;
        // Mint some coins
        atomic_bridge::mint(sender_address, account_balance);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            hash_lock,
            amount
        );

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance - amount, 0);


        let bridge_initiator_events = borrow_global<BridgeInitiatorEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        let bridge_transfer_id = bridge_transfer_initiated_event.bridge_transfer_id;

        refund_bridge_transfer(sender, bridge_transfer_id);
    }
}

module aptos_framework::atomic_bridge_store {
    use std::bcs;
    use std::features;
    use std::vector;
    use aptos_std::aptos_hash::keccak256;
    use aptos_std::smart_table;
    use aptos_std::smart_table::SmartTable;
    use aptos_framework::ethereum::EthereumAddress;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use std::signer;
    use aptos_framework::timestamp::CurrentTimeMicroseconds;

    friend aptos_framework::atomic_bridge_counterparty;
    friend aptos_framework::atomic_bridge_initiator;

    #[test_only]
    use std::hash::sha3_256;
    #[test_only]
    use aptos_framework::ethereum;
    #[test_only]
    use aptos_framework::atomic_bridge_configuration;

    /// Error codes
    const EINVALID_PRE_IMAGE : u64 = 0x1;
    const ENOT_PENDING_TRANSACTION : u64 = 0x2;
    const EEXPIRED : u64 = 0x3;
    const ENOT_EXPIRED : u64 = 0x4;
    const EINVALID_HASH_LOCK : u64 = 0x5;
    const EINVALID_TIME_LOCK : u64 = 0x6;
    const EZERO_AMOUNT : u64 = 0x7;
    const EINVALID_BRIDGE_TRANSFER_ID : u64 = 0x8;
    const EATOMIC_BRIDGE_NOT_ENABLED : u64 = 0x9;

    /// Transaction states
    const PENDING_TRANSACTION: u8 = 0x1;
    const COMPLETED_TRANSACTION: u8 = 0x2;
    const CANCELLED_TRANSACTION: u8 = 0x3;

    /// Minimum time lock of 1 second
    const MIN_TIME_LOCK : u64 = 1;
    const MAX_U64 : u64 = 0xFFFFFFFFFFFFFFFF;

    struct AddressPair<Initiator: store, Recipient: store> has store, copy {
        initiator: Initiator,
        recipient: Recipient,
    }

    /// A smart table wrapper
    struct SmartTableWrapper<K, V> has key, store {
        inner: SmartTable<K, V>,
    }

    /// Details on the transfer
    struct BridgeTransferDetails<Initiator: store, Recipient: store> has store, copy {
        addresses: AddressPair<Initiator, Recipient>,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
        state: u8,
    }

    /// A nonce for the bridge store
    struct Nonce has key {
        inner: u64
    }

    /// Initializes the initiators and counterparties tables and nonce.
    ///
    /// @param aptos_framework The signer for Aptos framework.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, Nonce {
            inner: 0,
        });

        let initiators = SmartTableWrapper<vector<u8>, BridgeTransferDetails<address, EthereumAddress>> {
            inner: smart_table::new(),
        };

        move_to(aptos_framework, initiators);

        let counterparties = SmartTableWrapper<vector<u8>, BridgeTransferDetails<EthereumAddress, address>> {
            inner: smart_table::new(),
        };

        move_to(aptos_framework, counterparties);
    }

    /// Returns the current time in seconds.
    ///
    /// @return Current timestamp in seconds.
    fun now() : u64 {
        timestamp::now_seconds()
    }

    /// Creates a time lock by adding a duration to the current time.
    ///
    /// @param lock The duration to lock.
    /// @return The calculated time lock.
    /// @abort If lock is not above MIN_TIME_LOCK
    public(friend) fun create_time_lock(time_lock: u64) : u64 {
        assert_min_time_lock(time_lock);
        now() + time_lock
    }

    /// Creates bridge transfer details with validation.
    ///
    /// @param initiator The initiating party of the transfer.
    /// @param recipient The receiving party of the transfer.
    /// @param amount The amount to be transferred.
    /// @param hash_lock The hash lock for the transfer.
    /// @param time_lock The time lock for the transfer.
    /// @return A `BridgeTransferDetails` object.
    /// @abort If the amount is zero or locks are invalid.
    public(friend) fun create_details<Initiator: store, Recipient: store>(initiator: Initiator, recipient: Recipient, amount: u64, hash_lock: vector<u8>, time_lock: u64)
        : BridgeTransferDetails<Initiator, Recipient> {
        assert!(amount > 0, EZERO_AMOUNT);
        assert_valid_hash_lock(&hash_lock);
        time_lock = create_time_lock(time_lock);

        BridgeTransferDetails {
            addresses: AddressPair {
                initiator,
                recipient
            },
            amount,
            hash_lock,
            time_lock,
            state: PENDING_TRANSACTION,
        }
    }

    /// Record details of a transfer
    ///
    /// @param bridge_transfer_id Bridge transfer ID.
    /// @param details The bridge transfer details
    public(friend) fun add<Initiator: store, Recipient: store>(bridge_transfer_id: vector<u8>, details: BridgeTransferDetails<Initiator, Recipient>) acquires SmartTableWrapper {
        assert!(features::abort_atomic_bridge_enabled(), EATOMIC_BRIDGE_NOT_ENABLED);

        assert_valid_bridge_transfer_id(&bridge_transfer_id);
        let table = borrow_global_mut<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework);
        smart_table::add(&mut table.inner, bridge_transfer_id, details);
    }

    /// Asserts that the time lock is valid.
    ///
    /// @param time_lock
    /// @abort If the time lock is invalid.
    fun assert_min_time_lock(time_lock: u64) {
        assert!(time_lock >= MIN_TIME_LOCK, EINVALID_TIME_LOCK);
    }

    /// Asserts that the details state is pending.
    ///
    /// @param details The bridge transfer details to check.
    /// @abort If the state is not pending.
    fun assert_pending<Initiator: store, Recipient: store>(details: &BridgeTransferDetails<Initiator, Recipient>) {
        assert!(details.state == PENDING_TRANSACTION, ENOT_PENDING_TRANSACTION)
    }

    /// Asserts that the hash lock is valid.
    ///
    /// @param hash_lock The hash lock to validate.
    /// @abort If the hash lock is invalid.
    fun assert_valid_hash_lock(hash_lock: &vector<u8>) {
        assert!(vector::length(hash_lock) == 32, EINVALID_HASH_LOCK);
    }

    /// Asserts that the bridge transfer ID is valid.
    ///
    /// @param bridge_transfer_id The bridge transfer ID to validate.
    /// @abort If the ID is invalid.
    public(friend) fun assert_valid_bridge_transfer_id(bridge_transfer_id: &vector<u8>) {
        assert!(vector::length(bridge_transfer_id) == 32, EINVALID_BRIDGE_TRANSFER_ID);
    }

    /// Creates a hash lock from a pre-image.
    ///
    /// @param pre_image The pre-image to hash.
    /// @return The generated hash lock.
    public(friend) fun create_hashlock(pre_image: vector<u8>) : vector<u8> {
        assert!(vector::length(&pre_image) > 0, EINVALID_PRE_IMAGE);
        keccak256(pre_image)
    }

    /// Asserts that the hash lock matches the expected value.
    ///
    /// @param details The bridge transfer details.
    /// @param hash_lock The hash lock to compare.
    /// @abort If the hash lock is incorrect.
    fun assert_correct_hash_lock<Initiator: store, Recipient: store>(details: &BridgeTransferDetails<Initiator, Recipient>, hash_lock: vector<u8>) {
        assert!(&hash_lock == &details.hash_lock, EINVALID_PRE_IMAGE);
    }

    /// Asserts that the time lock has expired.
    ///
    /// @param details The bridge transfer details.
    /// @abort If the time lock has not expired.
    fun assert_timed_out_lock<Initiator: store, Recipient: store>(details: &BridgeTransferDetails<Initiator, Recipient>) {
        assert!(now() > details.time_lock, ENOT_EXPIRED);
    }

    /// Asserts we are still within the timelock.
    ///
    /// @param details The bridge transfer details.
    /// @abort If the time lock has expired.
    fun assert_within_timelock<Initiator: store, Recipient: store>(details: &BridgeTransferDetails<Initiator, Recipient>) {
        assert!(!(now() > details.time_lock), EEXPIRED);
    }

    /// Completes the bridge transfer.
    ///
    /// @param details The bridge transfer details to complete.
    fun complete<Initiator: store, Recipient: store>(details: &mut BridgeTransferDetails<Initiator, Recipient>) {
        details.state = COMPLETED_TRANSACTION;
    }

    /// Cancels the bridge transfer.
    ///
    /// @param details The bridge transfer details to cancel.
    fun cancel<Initiator: store, Recipient: store>(details: &mut BridgeTransferDetails<Initiator, Recipient>) {
        details.state = CANCELLED_TRANSACTION;
    }

    /// Validates and completes a bridge transfer by confirming the hash lock and state.
    ///
    /// @param hash_lock The hash lock used to validate the transfer.
    /// @param details The mutable reference to the bridge transfer details to be completed.
    /// @return A tuple containing the recipient and the amount of the transfer.
    /// @abort If the hash lock is invalid, the transfer is not pending, or the hash lock does not match.
    fun complete_details<Initiator: store, Recipient: store + copy>(hash_lock: vector<u8>, details: &mut BridgeTransferDetails<Initiator, Recipient>) : (Recipient, u64) {
        assert_valid_hash_lock(&hash_lock);
        assert_pending(details);
        assert_correct_hash_lock(details, hash_lock);
        assert_within_timelock(details);

        complete(details);

        (details.addresses.recipient, details.amount)
    }

    /// Completes a bridge transfer by validating the hash lock and updating the transfer state.
    ///
    /// @param bridge_transfer_id The ID of the bridge transfer to complete.
    /// @param hash_lock The hash lock used to validate the transfer.
    /// @return A tuple containing the recipient of the transfer and the amount transferred.
    /// @abort If the bridge transfer details are not found or if the completion checks in `complete_details` fail.
    public(friend) fun complete_transfer<Initiator: store, Recipient: copy + store>(bridge_transfer_id: vector<u8>, hash_lock: vector<u8>) : (Recipient, u64) acquires SmartTableWrapper {
        assert!(features::abort_atomic_bridge_enabled(), EATOMIC_BRIDGE_NOT_ENABLED);

        let table = borrow_global_mut<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework);

        let details = smart_table::borrow_mut(
            &mut table.inner,
            bridge_transfer_id);

        complete_details<Initiator, Recipient>(hash_lock, details)
    }

    /// Cancels a pending bridge transfer if the time lock has expired.
    ///
    /// @param details A mutable reference to the bridge transfer details to be canceled.
    /// @return A tuple containing the initiator of the transfer and the amount to be refunded.
    /// @abort If the transfer is not in a pending state or the time lock has not expired.
    fun cancel_details<Initiator: store + copy, Recipient: store>(details: &mut BridgeTransferDetails<Initiator, Recipient>) : (Initiator, u64) {
        assert_pending(details);
        assert_timed_out_lock(details);

        cancel(details);

        (details.addresses.initiator, details.amount)
    }

    /// Cancels a bridge transfer if it is pending and the time lock has expired.
    ///
    /// @param bridge_transfer_id The ID of the bridge transfer to cancel.
    /// @return A tuple containing the initiator of the transfer and the amount to be refunded.
    /// @abort If the bridge transfer details are not found or if the cancellation conditions in `cancel_details` fail.
    public(friend) fun cancel_transfer<Initiator: store + copy, Recipient: store>(bridge_transfer_id: vector<u8>) : (Initiator, u64) acquires SmartTableWrapper {
        assert!(features::abort_atomic_bridge_enabled(), EATOMIC_BRIDGE_NOT_ENABLED);

        let table = borrow_global_mut<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework);

        let details = smart_table::borrow_mut(
            &mut table.inner,
            bridge_transfer_id);

        cancel_details<Initiator, Recipient>(details)
    }

    /// Generates a unique bridge transfer ID based on transfer details and nonce.
    ///
    /// @param details The bridge transfer details.
    /// @return The generated bridge transfer ID.
    public(friend) fun bridge_transfer_id<Initiator: store, Recipient: store>(details: &BridgeTransferDetails<Initiator, Recipient>) : vector<u8> acquires Nonce {
        let nonce = borrow_global_mut<Nonce>(@aptos_framework);
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.addresses.initiator));
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.addresses.recipient));
        vector::append(&mut combined_bytes, details.hash_lock);
        if (nonce.inner == MAX_U64) {
            nonce.inner = 0;  // Wrap around to 0 if at maximum value
        } else {
            nonce.inner = nonce.inner + 1;  // Safe to increment without overflow
        };
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce.inner));

        keccak256(combined_bytes)
    }

    #[view]
    /// Gets initiator bridge transfer details given a bridge transfer ID
    ///
    /// @param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
    /// @return A `BridgeTransferDetails` struct.
    /// @abort If there is no transfer in the atomic bridge store.
    public fun get_bridge_transfer_details_initiator(
        bridge_transfer_id: vector<u8>
    ): BridgeTransferDetails<address, EthereumAddress> acquires SmartTableWrapper {
        get_bridge_transfer_details(bridge_transfer_id)
    }
    
    #[view]
    /// Gets counterparty bridge transfer details given a bridge transfer ID
    ///
    /// @param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
    /// @return A `BridgeTransferDetails` struct.
    /// @abort If there is no transfer in the atomic bridge store.
    public fun get_bridge_transfer_details_counterparty(
        bridge_transfer_id: vector<u8>
    ): BridgeTransferDetails<EthereumAddress, address> acquires SmartTableWrapper {
        get_bridge_transfer_details(bridge_transfer_id)
    }

    fun get_bridge_transfer_details<Initiator: store + copy, Recipient: store + copy>(bridge_transfer_id: vector<u8>
    ): BridgeTransferDetails<Initiator, Recipient> acquires SmartTableWrapper {
        let table = borrow_global<SmartTableWrapper<vector<u8>, BridgeTransferDetails<Initiator, Recipient>>>(@aptos_framework);

        let details_ref = smart_table::borrow(
            &table.inner,
            bridge_transfer_id
        );

        *details_ref
    }

    #[test_only]
    public fun valid_bridge_transfer_id() : vector<u8> {
        sha3_256(b"atomic bridge")
    }

    #[test_only]
    public fun plain_secret() : vector<u8> {
        b"too secret!"
    }

    #[test_only]
    public fun valid_hash_lock() : vector<u8> {
        keccak256(plain_secret())
    }


    #[test(aptos_framework = @aptos_framework)]
    public fun test_get_bridge_transfer_details_initiator(aptos_framework: &signer) acquires SmartTableWrapper {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        features::change_feature_flags_for_testing(
            aptos_framework,
            vector[features::get_atomic_bridge_feature()],
            vector[]
        );
        atomic_bridge_configuration::initialize(aptos_framework);
        initialize(aptos_framework);

        let initiator = signer::address_of(aptos_framework);
        let recipient = ethereum::ethereum_address(ethereum::valid_eip55());
        let amount = 1000;
        let hash_lock = valid_hash_lock();
        let time_lock = create_time_lock(3600);
        let bridge_transfer_id = valid_bridge_transfer_id();

        let details = create_details(
            initiator, 
            recipient, 
            amount, 
            hash_lock, 
            time_lock
        );

        add(bridge_transfer_id, details);

        let retrieved_details = get_bridge_transfer_details_initiator(bridge_transfer_id);

        let BridgeTransferDetails {
            addresses: AddressPair {
                initiator: retrieved_initiator,
                recipient: retrieved_recipient
            },
            amount: retrieved_amount,
            hash_lock: retrieved_hash_lock,
            time_lock: retrieved_time_lock,
            state: retrieved_state
        } = retrieved_details;

        assert!(retrieved_initiator == initiator, 0);
        assert!(retrieved_recipient == recipient, 1);
        assert!(retrieved_amount == amount, 2);
        assert!(retrieved_hash_lock == hash_lock, 3);
        assert!(retrieved_time_lock == time_lock, 4);
        assert!(retrieved_state == PENDING_TRANSACTION, 5);
    }

    #[test(aptos_framework = @aptos_framework)]
    public fun test_get_bridge_transfer_details_counterparty(aptos_framework: &signer) acquires SmartTableWrapper {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        features::change_feature_flags_for_testing(
            aptos_framework,
            vector[features::get_atomic_bridge_feature()],
            vector[]
        );
        initialize(aptos_framework);

        let initiator = ethereum::ethereum_address(ethereum::valid_eip55());
        let recipient = signer::address_of(aptos_framework);
        let amount = 500;
        let hash_lock = valid_hash_lock();
        let time_lock = create_time_lock(3600);
        let bridge_transfer_id = valid_bridge_transfer_id();

        let details = create_details(
            initiator, 
            recipient, 
            amount, 
            hash_lock, 
            time_lock
        );

        add(bridge_transfer_id, details);

        let retrieved_details = get_bridge_transfer_details_counterparty(bridge_transfer_id);

        let BridgeTransferDetails {
            addresses: AddressPair {
                initiator: retrieved_initiator,
                recipient: retrieved_recipient
            },
            amount: retrieved_amount,
            hash_lock: retrieved_hash_lock,
            time_lock: retrieved_time_lock,
            state: retrieved_state
        } = retrieved_details;

        assert!(retrieved_initiator == initiator, 0);
        assert!(retrieved_recipient == recipient, 1);
        assert!(retrieved_amount == amount, 2);
        assert!(retrieved_hash_lock == hash_lock, 3);
        assert!(retrieved_time_lock == time_lock, 4);
        assert!(retrieved_state == PENDING_TRANSACTION, 5);
    }
}

module aptos_framework::atomic_bridge_configuration {
    use std::signer;
    use aptos_framework::event;
    use aptos_framework::system_addresses;

    friend aptos_framework::atomic_bridge_counterparty;
    friend aptos_framework::atomic_bridge_initiator;

    /// Error code for invalid bridge operator
    const EINVALID_BRIDGE_OPERATOR: u64 = 0x1;

    /// Counterparty time lock duration is 24 hours in seconds
    const COUNTERPARTY_TIME_LOCK_DUARTION: u64 = 24 * 60 * 60;
    /// Initiator time lock duration is 48 hours in seconds
    const INITIATOR_TIME_LOCK_DUARTION: u64 = 48 * 60 * 60;

    struct BridgeConfig has key {
        bridge_operator: address,
        initiator_time_lock: u64,
        counterparty_time_lock: u64,
    }

    /// Default minimum sponsored transfer amount is 1_000_000_000
    const DEFAULT_MINIMUM_TRANSFER_AMOUNT: u64 = 1_000_000_000;
    /// Default sponsorship amount is 1_000
    const DEFAULT_SPONSORSHIP_AMOUNT: u64 = 1_000;

    /// Configuration for sponsorship
    struct SponsorshipConfig has key {
        minimum_transfer_amount: u64,
        sponsorship_amount: u64,
    }

    #[event]
    /// Event emitted when the bridge operator is updated.
    struct BridgeConfigOperatorUpdated has store, drop {
        old_operator: address,
        new_operator: address,
    }

    #[event]
    /// Event emitted when the initiator time lock has been updated.
    struct InitiatorTimeLockUpdated has store, drop {
        time_lock: u64,
    }

    #[event]
    /// Event emitted when the initiator time lock has been updated.
    struct CounterpartyTimeLockUpdated has store, drop {
        time_lock: u64,
    }

    /// Initializes the bridge configuration with Aptos framework as the bridge operator.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        let bridge_config = BridgeConfig {
            bridge_operator: signer::address_of(aptos_framework),
            initiator_time_lock: INITIATOR_TIME_LOCK_DUARTION,
            counterparty_time_lock: COUNTERPARTY_TIME_LOCK_DUARTION,
        };
        move_to(aptos_framework, bridge_config);

        let sponsorship_config = SponsorshipConfig {
            minimum_transfer_amount: DEFAULT_MINIMUM_TRANSFER_AMOUNT,
            sponsorship_amount: DEFAULT_SPONSORSHIP_AMOUNT,
        };
        move_to(aptos_framework, sponsorship_config);
    }

    /// Updates the bridge operator, requiring governance validation.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param new_operator The new address to be set as the bridge operator.
    /// @abort If the current operator is the same as the new operator.
    public fun update_bridge_operator(aptos_framework: &signer, new_operator: address
    )   acquires BridgeConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let bridge_config = borrow_global_mut<BridgeConfig>(@aptos_framework);
        let old_operator = bridge_config.bridge_operator;
        assert!(old_operator != new_operator, EINVALID_BRIDGE_OPERATOR);

        bridge_config.bridge_operator = new_operator;

        event::emit(
            BridgeConfigOperatorUpdated {
                old_operator,
                new_operator,
            },
        );
    }

    public fun set_initiator_time_lock_duration(aptos_framework: &signer, time_lock: u64
    ) acquires BridgeConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<BridgeConfig>(@aptos_framework).initiator_time_lock = time_lock;

        event::emit(
            InitiatorTimeLockUpdated {
                time_lock
            },
        );
    }

    public fun set_counterparty_time_lock_duration(aptos_framework: &signer, time_lock: u64
    ) acquires BridgeConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<BridgeConfig>(@aptos_framework).counterparty_time_lock = time_lock;

        event::emit(
            CounterpartyTimeLockUpdated {
                time_lock
            },
        );
    }

    #[view]
    public fun initiator_timelock_duration() : u64 acquires BridgeConfig {
        borrow_global<BridgeConfig>(@aptos_framework).initiator_time_lock
    }

    #[view]
    public fun counterparty_timelock_duration() : u64 acquires BridgeConfig {
        borrow_global<BridgeConfig>(@aptos_framework).counterparty_time_lock
    }

    #[view]
    /// Retrieves the address of the current bridge operator.
    ///
    /// @return The address of the current bridge operator.
    public fun bridge_operator(): address acquires BridgeConfig {
        borrow_global_mut<BridgeConfig>(@aptos_framework).bridge_operator
    }

    /// Asserts that the caller is the current bridge operator.
    ///
    /// @param caller The signer whose authority is being checked.
    /// @abort If the caller is not the current bridge operator.
    public(friend) fun assert_is_caller_operator(caller: &signer
    ) acquires BridgeConfig {
        assert!(borrow_global<BridgeConfig>(@aptos_framework).bridge_operator == signer::address_of(caller), EINVALID_BRIDGE_OPERATOR);
    }

    #[view]
    /// Gets the current minimum transfer amount for sponsorship.
    ///
    /// @return The minimum transfer amount for sponsorship.
    public fun minimum_transfer_amount() : u64 acquires SponsorshipConfig {
        borrow_global<SponsorshipConfig>(@aptos_framework).minimum_transfer_amount
    }

    #[view]
    /// Gets the current sponsorship amount.
    ///
    /// @return The sponsorship amount.
    public fun sponsorship_amount() : u64 acquires SponsorshipConfig {
        borrow_global<SponsorshipConfig>(@aptos_framework).sponsorship_amount
    }

    /// Updates the minimum transfer amount for sponsorship.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param minimum_transfer_amount The new minimum transfer amount.
    public fun update_minimum_transfer_amount(aptos_framework: &signer, minimum_transfer_amount: u64
    ) acquires SponsorshipConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<SponsorshipConfig>(@aptos_framework).minimum_transfer_amount = minimum_transfer_amount;
    }

    /// Updates the sponsorship amount.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param sponsorship_amount The new sponsorship amount.
    public fun update_sponsorship_amount(aptos_framework: &signer, sponsorship_amount: u64
    ) acquires SponsorshipConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        borrow_global_mut<SponsorshipConfig>(@aptos_framework).sponsorship_amount = sponsorship_amount;
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests initialization of the bridge configuration.
    fun test_initialization(aptos_framework: &signer) {
        initialize(aptos_framework);
        assert!(exists<BridgeConfig>(@aptos_framework), 0);
    }

    #[test(aptos_framework = @aptos_framework, new_operator = @0xcafe)]
    /// Tests updating the bridge operator and emitting the corresponding event.
    fun test_update_bridge_operator(aptos_framework: &signer, new_operator: address
    ) acquires BridgeConfig {
        initialize(aptos_framework);
        update_bridge_operator(aptos_framework, new_operator);

        assert!(
            event::was_event_emitted<BridgeConfigOperatorUpdated>(
                &BridgeConfigOperatorUpdated {
                    old_operator: @aptos_framework,
                    new_operator,
                }
            ), 0);

        assert!(bridge_operator() == new_operator, 0);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad, new_operator = @0xcafe)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that updating the bridge operator with an invalid signer fails.
    fun test_failing_update_bridge_operator(aptos_framework: &signer, bad: &signer, new_operator: address
    ) acquires BridgeConfig {
        initialize(aptos_framework);
        update_bridge_operator(bad, new_operator);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests that the correct operator is validated successfully.
    fun test_is_valid_operator(aptos_framework: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        assert_is_caller_operator(aptos_framework);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 0x1, location = 0x1::atomic_bridge_configuration)]
    /// Tests that an incorrect operator is not validated and results in an abort.
    fun test_is_not_valid_operator(aptos_framework: &signer, bad: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        assert_is_caller_operator(bad);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests we can update the initiator time lock
    fun test_update_initiator_time_lock(aptos_framework: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        set_initiator_time_lock_duration(aptos_framework, 1);
        assert!(initiator_timelock_duration() == 1, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests we can update the initiator time lock
    fun test_update_counterparty_time_lock(aptos_framework: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        set_counterparty_time_lock_duration(aptos_framework, 1);
        assert!(counterparty_timelock_duration() == 1, 0);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that an incorrect signer cannot update the initiator time lock
    fun test_not_able_to_set_initiator_time_lock(aptos_framework: &signer, bad: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        set_initiator_time_lock_duration(bad, 1);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that an incorrect signer cannot update the counterparty time lock
    fun test_not_able_to_set_counterparty_time_lock(aptos_framework: &signer, bad: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        set_counterparty_time_lock_duration(bad, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests we can update the minimum transfer amount
    fun test_update_minimum_transfer_amount(aptos_framework: &signer) acquires SponsorshipConfig {
        initialize(aptos_framework);
        update_minimum_transfer_amount(aptos_framework, 1);
        assert!(minimum_transfer_amount() == 1, 0);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that an incorrect signer cannot update the minimum transfer amount
    fun test_not_able_to_update_minimum_transfer_amount(aptos_framework: &signer, bad: &signer) acquires SponsorshipConfig {
        initialize(aptos_framework);
        update_minimum_transfer_amount(bad, 1);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests we can update the sponsorship amount
    fun test_update_sponsorship_amount(aptos_framework: &signer) acquires SponsorshipConfig {
        initialize(aptos_framework);
        update_sponsorship_amount(aptos_framework, 1);
        assert!(sponsorship_amount() == 1, 0);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that an incorrect signer cannot update the sponsorship amount
    fun test_not_able_to_update_sponsorship_amount(aptos_framework: &signer, bad: &signer) acquires SponsorshipConfig {
        initialize(aptos_framework);
        update_sponsorship_amount(bad, 1);
    }

}

module aptos_framework::atomic_bridge {
    use std::features;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::atomic_bridge_configuration;
    use aptos_framework::atomic_bridge_store;
    use aptos_framework::coin;
    use aptos_framework::coin::{BurnCapability, MintCapability};
    use aptos_framework::fungible_asset::{BurnRef, MintRef};
    use aptos_framework::system_addresses;
    #[test_only]
    use aptos_framework::account;
    #[test_only]
    use aptos_framework::aptos_coin;
    #[test_only]
    use aptos_framework::timestamp;

    friend aptos_framework::atomic_bridge_counterparty;
    friend aptos_framework::atomic_bridge_initiator;
    friend aptos_framework::genesis;

    const EATOMIC_BRIDGE_NOT_ENABLED : u64 = 0x1;

    struct AptosCoinBurnCapability has key {
        burn_cap: BurnCapability<AptosCoin>,
    }

    struct AptosCoinMintCapability has key {
        mint_cap: MintCapability<AptosCoin>,
    }

    struct AptosFABurnCapabilities has key {
        burn_ref: BurnRef,
    }

    struct AptosFAMintCapabilities has key {
        burn_ref: MintRef,
    }

    /// Initializes the atomic bridge by setting up necessary configurations.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    public fun initialize(aptos_framework: &signer) {
        atomic_bridge_configuration::initialize(aptos_framework);
        atomic_bridge_store::initialize(aptos_framework);
    }

    #[test_only]
    /// Initializes the atomic bridge for testing purposes, including setting up accounts and timestamps.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    public fun initialize_for_test(aptos_framework: &signer) {
        timestamp::set_time_has_started_for_testing(aptos_framework);
        account::create_account_for_test(@aptos_framework);
        features::change_feature_flags_for_testing(
            aptos_framework,
            vector[features::get_atomic_bridge_feature()],
            vector[]
        );
        initialize(aptos_framework);

        let (burn_cap, mint_cap) = aptos_coin::initialize_for_test(aptos_framework);

        store_aptos_coin_mint_cap(aptos_framework, mint_cap);
        store_aptos_coin_burn_cap(aptos_framework, burn_cap);
    }

    /// Stores the burn capability for AptosCoin, converting to a fungible asset reference if the feature is enabled.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param burn_cap The burn capability for AptosCoin.
    public fun store_aptos_coin_burn_cap(aptos_framework: &signer, burn_cap: BurnCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (features::operations_default_to_fa_apt_store_enabled()) {
            let burn_ref = coin::convert_and_take_paired_burn_ref(burn_cap);
            move_to(aptos_framework, AptosFABurnCapabilities { burn_ref });
        } else {
            move_to(aptos_framework, AptosCoinBurnCapability { burn_cap })
        }
    }

    /// Stores the mint capability for AptosCoin.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param mint_cap The mint capability for AptosCoin.
    public fun store_aptos_coin_mint_cap(aptos_framework: &signer, mint_cap: MintCapability<AptosCoin>) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, AptosCoinMintCapability { mint_cap })
    }

    /// Mints a specified amount of AptosCoin to a recipient's address.
    ///
    /// @param recipient The address of the recipient to mint coins to.
    /// @param amount The amount of AptosCoin to mint.
    /// @abort If the mint capability is not available.
    public(friend) fun mint(recipient: address, amount: u64) acquires AptosCoinMintCapability {
        assert!(features::abort_atomic_bridge_enabled(), EATOMIC_BRIDGE_NOT_ENABLED);

        coin::deposit(recipient, coin::mint(
            amount,
            &borrow_global<AptosCoinMintCapability>(@aptos_framework).mint_cap
        ));
    }

    /// Burns a specified amount of AptosCoin from an address.
    ///
    /// @param from The address from which to burn AptosCoin.
    /// @param amount The amount of AptosCoin to burn.
    /// @abort If the burn capability is not available.
    public(friend) fun burn(from: address, amount: u64) acquires AptosCoinBurnCapability {
        assert!(features::abort_atomic_bridge_enabled(), EATOMIC_BRIDGE_NOT_ENABLED);

        coin::burn_from(
            from,
            amount,
            &borrow_global<AptosCoinBurnCapability>(@aptos_framework).burn_cap,
        );
    }
}

module aptos_framework::atomic_bridge_counterparty {
    use aptos_framework::account;
    use aptos_framework::coin;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::aptos_account;
    use aptos_framework::atomic_bridge;
    use aptos_framework::atomic_bridge_configuration;
    use aptos_framework::atomic_bridge_store;
    use aptos_framework::atomic_bridge_store::create_hashlock;
    use aptos_framework::ethereum;
    use aptos_framework::ethereum::EthereumAddress;
    use aptos_framework::event::{Self, EventHandle}; 
    #[test_only]
    use aptos_framework::atomic_bridge::initialize_for_test;
    #[test_only]
    use aptos_framework::atomic_bridge_store::{valid_bridge_transfer_id, valid_hash_lock, plain_secret};
    #[test_only]
    use aptos_framework::ethereum::valid_eip55;
    #[test_only]
    use aptos_framework::signer;
    #[test_only]
    use aptos_framework::timestamp;

    #[event]
    /// An event triggered upon locking assets for a bridge transfer
    struct BridgeTransferLockedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        initiator: vector<u8>,
        recipient: address,
        amount: u64,
        hash_lock: vector<u8>,
        time_lock: u64,
    }

    #[event]
    /// An event triggered upon completing a bridge transfer
    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    }

    #[event]
    /// An event triggered upon cancelling a bridge transfer
    struct BridgeTransferCancelledEvent has store, drop {
        bridge_transfer_id: vector<u8>,
    }

    /// This struct will store the event handles for bridge events.
    struct BridgeCounterpartyEvents has key, store {
        bridge_transfer_locked_events: EventHandle<BridgeTransferLockedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
        bridge_transfer_cancelled_events: EventHandle<BridgeTransferCancelledEvent>,
    }

    /// Initializes the module and stores the `EventHandle`s in the resource.
    public fun initialize(aptos_framework: &signer) {
        move_to(aptos_framework, BridgeCounterpartyEvents {
            bridge_transfer_locked_events: account::new_event_handle<BridgeTransferLockedEvent>(aptos_framework),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(aptos_framework),
            bridge_transfer_cancelled_events: account::new_event_handle<BridgeTransferCancelledEvent>(aptos_framework),
        });
    }

    /// Checks whether a recipient is eligible for sponsorship.
    ///
    /// @param recipient The address of the recipient to check.
    /// @return A boolean indicating whether the recipient is eligible for sponsorship.
    public(friend) fun is_sponsorship_eligible(recipient: address, transfer_amount : u64) : bool {
        // A minimum transfer amount is required to be eligible for sponsorship
        // This is used to make attacks on this sponsorhip mechanism prohibitively expensive
        if (transfer_amount < atomic_bridge_configuration::minimum_transfer_amount()) {
            return false
        };
        !account::exists_at(recipient) || coin::balance<AptosCoin>(recipient) == 0
    }

    /// Sponsors a recipient by transferring enought Aptos Coint to make the claim to them
    ///
    /// @param recipient The address of the recipient to sponsor.
    /// @param amount The amount of Aptos Coin to transfer.
    /// @abort If the transfer fails.
    public(friend) fun sponsor(sponsor: &signer, recipient: address) : u64 {
        let amount = atomic_bridge_configuration::sponsorship_amount();
        aptos_account::transfer(sponsor, recipient, amount);
        amount
    }

    /// Sponsors a recipient by transferring enough Aptos Coin to make the claim to them if they are eligible
    ///
    /// @param recipient The address of the recipient to sponsor.
    /// @param amount The amount of Aptos Coin to transfer.
    /// @abort If the transfer fails.
    /// Returns the amount remaining in the transfer.
    public(friend) fun sponsor_if_eligible(sponsor: &signer, recipient: address, amount: u64) : u64 {
        if(is_sponsorship_eligible(recipient, amount)) {
            amount - sponsor(sponsor, recipient)
        } else {
            amount
        }
    }


    /// Locks assets for a bridge transfer by the initiator.
    ///
    /// @param caller The signer representing the bridge operator.
    /// @param initiator The initiator's Ethereum address as a vector of bytes.
    /// @param bridge_transfer_id The unique identifier for the bridge transfer.
    /// @param hash_lock The hash lock for securing the transfer.
    /// @param time_lock The time lock duration for the transfer.
    /// @param recipient The address of the recipient on the Aptos blockchain.
    /// @param amount The amount of assets to be locked.
    /// @abort If the caller is not the bridge operator.
    public entry fun lock_bridge_transfer_assets (
        caller: &signer,
        initiator: vector<u8>,
        bridge_transfer_id: vector<u8>,
        hash_lock: vector<u8>,
        recipient: address,
        amount: u64
    ) acquires BridgeCounterpartyEvents {
        atomic_bridge_configuration::assert_is_caller_operator(caller);
        let ethereum_address = ethereum::ethereum_address_no_eip55(initiator);
        let time_lock = atomic_bridge_configuration::counterparty_timelock_duration();

        // sponsor the claim transaction if the recipient is valid for sponsorship
        // sponsorship will always be restricted by the amount of Aptos Coin available to the bridge operator
        let post_sponsorship_amount = sponsor_if_eligible(caller, recipient, amount);

        let details = atomic_bridge_store::create_details(
            ethereum_address,
            recipient,
            post_sponsorship_amount,
            hash_lock,
            time_lock
        );

        // bridge_store::add_counterparty(bridge_transfer_id, details);
        atomic_bridge_store::add(bridge_transfer_id, details);

        let bridge_events = borrow_global_mut<BridgeCounterpartyEvents>(@aptos_framework);

        event::emit_event(
            &mut bridge_events.bridge_transfer_locked_events,
            BridgeTransferLockedEvent {
                bridge_transfer_id,
                initiator,
                recipient,
                amount,
                hash_lock,
                time_lock,
            },
        );
    }

    /// Completes a bridge transfer by revealing the pre-image.
    ///
    /// @param bridge_transfer_id The unique identifier for the bridge transfer.
    /// @param pre_image The pre-image that matches the hash lock to complete the transfer.
    /// @abort If the caller is not the bridge operator or the hash lock validation fails.
    public entry fun complete_bridge_transfer (
        bridge_transfer_id: vector<u8>,
        pre_image: vector<u8>,
    ) acquires BridgeCounterpartyEvents {
        let (recipient, amount) = atomic_bridge_store::complete_transfer<EthereumAddress, address>(
            bridge_transfer_id,
            create_hashlock(pre_image)
        );

        // Mint, fails silently
        atomic_bridge::mint(recipient, amount);
        
        let bridge_counterparty_events = borrow_global_mut<BridgeCounterpartyEvents>(@aptos_framework);
        event::emit_event(
            &mut bridge_counterparty_events.bridge_transfer_completed_events,
            BridgeTransferCompletedEvent {
                bridge_transfer_id,
                pre_image,
            },
        );
    }

    /// Aborts a bridge transfer if the time lock has expired.
    ///
    /// @param caller The signer representing the bridge operator.
    /// @param bridge_transfer_id The unique identifier for the bridge transfer.
    /// @abort If the caller is not the bridge operator or if the time lock has not expired.
    public entry fun abort_bridge_transfer (
        caller: &signer,
        bridge_transfer_id: vector<u8>
    ) acquires BridgeCounterpartyEvents {
        atomic_bridge_configuration::assert_is_caller_operator(caller);

        atomic_bridge_store::cancel_transfer<EthereumAddress, address>(bridge_transfer_id);

        let bridge_counterparty_events = borrow_global_mut<BridgeCounterpartyEvents>(@aptos_framework);
        event::emit_event(
            &mut bridge_counterparty_events.bridge_transfer_cancelled_events,
            BridgeTransferCancelledEvent {
                bridge_transfer_id,
            },
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_lock_assets(aptos_framework: &signer) acquires BridgeCounterpartyEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        let initiator = valid_eip55();
        let bridge_transfer_id = valid_bridge_transfer_id();
        let hash_lock = valid_hash_lock();
        let recipient = @0xcafe;
        let amount = 1;

        lock_bridge_transfer_assets(aptos_framework,
                                    initiator,
                                    bridge_transfer_id,
                                    hash_lock,
                                    recipient,
                                    amount);

        let bridge_counterparty_events = borrow_global<BridgeCounterpartyEvents>(signer::address_of(aptos_framework));
        let lock_events = event::emitted_events_by_handle(&bridge_counterparty_events.bridge_transfer_locked_events);

        // Assert that the event was emitted
        let expected_event = BridgeTransferLockedEvent {
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
            hash_lock,
            time_lock: atomic_bridge_configuration::counterparty_timelock_duration(),
        };
        assert!(std::vector::contains(&lock_events, &expected_event), 0);

    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_abort_transfer_of_assets(aptos_framework: &signer) acquires BridgeCounterpartyEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        let initiator = valid_eip55();
        let bridge_transfer_id = valid_bridge_transfer_id();
        let hash_lock = valid_hash_lock();
        let recipient = @0xcafe;
        let amount = 1;

        lock_bridge_transfer_assets(aptos_framework,
            initiator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount);

        timestamp::fast_forward_seconds(atomic_bridge_configuration::counterparty_timelock_duration() + 1);
        abort_bridge_transfer(aptos_framework, bridge_transfer_id);

        let bridge_counterparty_events = borrow_global<BridgeCounterpartyEvents>(signer::address_of(aptos_framework));
        let cancel_events = event::emitted_events_by_handle(&bridge_counterparty_events.bridge_transfer_cancelled_events);
        let expected_event = BridgeTransferCancelledEvent { bridge_transfer_id };
        assert!(std::vector::contains(&cancel_events, &expected_event), 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_complete_transfer_of_assets(aptos_framework: &signer) acquires BridgeCounterpartyEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        let initiator = valid_eip55();
        let bridge_transfer_id = valid_bridge_transfer_id();
        let hash_lock = valid_hash_lock();
        let recipient = @0xcafe;
        let amount = 1;

        // Create an account for our recipient
        aptos_account::create_account(recipient);

        lock_bridge_transfer_assets(aptos_framework,
            initiator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount);

        complete_bridge_transfer(bridge_transfer_id, plain_secret());

        let bridge_counterparty_events = borrow_global<BridgeCounterpartyEvents>(signer::address_of(aptos_framework));
        let complete_events = event::emitted_events_by_handle(&bridge_counterparty_events.bridge_transfer_completed_events);
        let expected_event = BridgeTransferCompletedEvent {
            bridge_transfer_id,
            pre_image: plain_secret(),
        };
        assert!(std::vector::contains(&complete_events, &expected_event), 0);

    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x1, location = atomic_bridge_store)]
    fun test_failing_complete_transfer_of_assets(aptos_framework: &signer) acquires BridgeCounterpartyEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        let initiator = valid_eip55();
        let bridge_transfer_id = valid_bridge_transfer_id();
        let hash_lock = valid_hash_lock();
        let recipient = @0xcafe;
        let amount = 1;

        lock_bridge_transfer_assets(aptos_framework,
            initiator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount);

        complete_bridge_transfer(bridge_transfer_id, b"not the secret");
    }

    #[test(aptos_framework = @aptos_framework, third_party = @0xdead)]
    /// Test eligible sponsor 
    fun test_lock_for_eligible_sponsor(aptos_framework: &signer, third_party: &signer) acquires BridgeCounterpartyEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);

        // transfer some Aptos Coin from the third party to the framework
        aptos_account::create_account(signer::address_of(third_party));
        atomic_bridge::mint(signer::address_of(third_party), 1_000_000_000);
        aptos_account::transfer(third_party, signer::address_of(aptos_framework), 1_000_000_000);

        let initiator = valid_eip55();
        let bridge_transfer_id = valid_bridge_transfer_id();
        let hash_lock = valid_hash_lock();
        let recipient = @0xcafe;
        let amount = atomic_bridge_configuration::minimum_transfer_amount();

        lock_bridge_transfer_assets(aptos_framework,
            initiator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount);

        let sponsored_amount = atomic_bridge_configuration::sponsorship_amount();

        // check that the recipient has been sponsored
        assert!(coin::balance<AptosCoin>(recipient) == sponsored_amount, 0);

    }

    #[test(aptos_framework = @aptos_framework, third_party = @0xdead)]
    /// Test ineligible sponsor
    fun test_lock_for_ineligible_sponsor(aptos_framework: &signer,  third_party: &signer) acquires BridgeCounterpartyEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);

        // mint some coin for the framework
        aptos_account::create_account(signer::address_of(third_party));
        atomic_bridge::mint(signer::address_of(third_party), 1_000_000_000);
        aptos_account::transfer(third_party, signer::address_of(aptos_framework), 1_000_000_000);

        let initiator = valid_eip55();
        let bridge_transfer_id = valid_bridge_transfer_id();
        let hash_lock = valid_hash_lock();
        let recipient = @0xcafe;
        let amount = atomic_bridge_configuration::minimum_transfer_amount() - 1;

        lock_bridge_transfer_assets(aptos_framework,
            initiator,
            bridge_transfer_id,
            hash_lock,
            recipient,
            amount);

        // check that the recipient has not been sponsored
        assert!(coin::balance<AptosCoin>(recipient) == 0, 0);

    }

}