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

module aptos_framework::native_bridge {

    use aptos_framework::account;
    use aptos_framework::native_bridge_core;
    use aptos_framework::native_bridge_configuration;
    use aptos_framework::native_bridge_configuration::assert_is_caller_operator;
    use aptos_framework::native_bridge_store;
    use aptos_framework::native_bridge_store::{create_hashlock, bridge_transfer_id};
    use aptos_framework::ethereum;
    use aptos_framework::ethereum::EthereumAddress;
    use aptos_framework::event::{Self, EventHandle}; 
    use aptos_framework::signer;
    use aptos_framework::system_addresses;
    #[test_only]
    use std::vector;
    #[test_only]
    use aptos_framework::aptos_account;
    #[test_only]
    use aptos_framework::aptos_coin::AptosCoin;
    #[test_only]
    use aptos_framework::native_bridge_store::{valid_hash_lock, assert_valid_bridge_transfer_id, plain_secret};
    #[test_only]
    use aptos_framework::coin;
    #[test_only]
    use aptos_framework::ethereum::valid_eip55;
    #[test_only]
    use aptos_framework::timestamp;

    #[event]
    /// An event triggered upon initiating a bridge transfer
    struct BridgeTransferInitiatedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        initiator: address,
        recipient: vector<u8>,
        amount: u64,
        nonce: u64,
    }

    #[event]
    /// An event triggered upon completing a bridge transfer
    struct BridgeTransferCompletedEvent has store, drop {
        bridge_transfer_id: vector<u8>,
        initiator: vector<u8>,
        recipient: address,
        amount: u64,
        nonce: u64,
    }

    /// This struct will store the event handles for bridge events.
    struct BridgeEvents has key, store {
        bridge_transfer_intiated_events: EventHandle<BridgeTransferInitiatedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
    }

    struct Nonce has key {
        value: u64
    }

    /// Increment and get the current nonce  
    fun increment_and_get_nonce(signer: address): u64 acquires Nonce {  
        let nonce_ref = borrow_global_mut<Nonce>(@aptos_framework);  
        nonce_ref.value = nonce_ref.value + 1;  
        nonce_ref.value  
    }  

    /// Initializes the module and stores the `EventHandle`s in the resource.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);

        // Ensure the nonce is not already initialized
        assert!(
            !exists<Nonce>(signer::address_of(aptos_framework)),
            2
        );

        // Create the Nonce resource with an initial value of 1
        move_to<Nonce>(aptos_framework, Nonce { value: 0 });

        move_to(aptos_framework, BridgeEvents {
            bridge_transfer_initiated_events: account::new_event_handle<BridgeTransferInitiatedEvent>(aptos_framework),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(aptos_framework),
        });
    }

    /// Initiate a bridge transfer of MOVE from Movement to the base layer  
    /// Anyone can initiate a bridge transfer from the source chain  
    /// The amount is burnt from the initiator and the module-level nonce is incremented  
    /// @param initiator The initiator's Ethereum address as a vector of bytes.  
    /// @param recipient The address of the recipient on the Aptos blockchain.  
    /// @param amount The amount of assets to be locked.  
    public entry fun initiate_bridge_transfer(  
        initiator: &signer,  
        recipient: vector<u8>,  
        amount: u64  
    ) acquires BridgeEvents, Nonce {  
        let initiator_address = signer::address_of(initiator);  
        let ethereum_address = ethereum::ethereum_address(recipient);  
    
        // Increment and retrieve the nonce  
        let nonce = increment_and_get_nonce(initiator_address);  
    
        // Create bridge transfer details  
        let details = native_bridge_store::create_details(  
            initiator_address,  
            ethereum_address, 
            amount,  
            nonce  
        );  
    
        // Generate a unique bridge transfer ID  
        // Todo: pass the nonce in here and modify the function to take a nonce. Or only use the nonce in native_bridge_store
        let bridge_transfer_id = native_bridge_store::bridge_transfer_id(&details);  
    
        // Add the transfer details to storage  
        native_bridge_store::add(bridge_transfer_id, details);
    
        // Burn the amount from the initiator  
        native_bridge::burn(initiator_address, amount);  
    
        let bridge_events = borrow_global_mut<BridgeEvents>(@aptos_framework);

        // Emit an event with nonce  
        event::emit_event(  
             &mut bridge_events.bridge_transfer_events,
            BridgeTransferInitiatedEvent {  
                bridge_transfer_id,  
                initiator: initiator_address,  
                recipient,  
                amount,  
                nonce,  
            }  
        );  
    }

     /// Completes a bridge transfer by the initiator.  
    ///  
    /// @param caller The signer representing the bridge operator.  
    /// @param initiator The initiator's Ethereum address as a vector of bytes.  
    /// @param bridge_transfer_id The unique identifier for the bridge transfer.  
    /// @param recipient The address of the recipient on the Aptos blockchain.  
    /// @param amount The amount of assets to be locked.  
    /// @param nonce The unique nonce for the transfer.    
    /// @abort If the caller is not the bridge operator.  
    public entry fun complete_bridge_transfer(  
        caller: &signer,  
        bridge_transfer_id: vector<u8>,
        initiator: vector<u8>,  
        recipient: address,  
        amount: u64,  
        nonce: u64  
    ) acquires BridgeEvents {  
        native_bridge_configuration::assert_is_caller_operator(caller);  
        let ethereum_address = ethereum::ethereum_address_no_eip55(initiator);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&initiator));
        vector::append(&mut combined_bytes, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes, bcs::to_bytes(&amount));
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));
        assert!(keccak256(combined_bytes) == bridge_transfer_id, EINVALID_BRIDGE_TRANSFER_ID);
        // todo: expect it to be empty
        let retrieved_bridge_transfer_id = native_bridge_store::get_bridge_transfer_id_from_nonce(nonce);
        set_nonce_to_bridge_transfer_id(nonce, bridge_transfer_id);
 
        // Mint to recipient  
        native_bridge::mint(recipient, amount);

        let bridge_events = borrow_global_mut<BridgeEvents>(@aptos_framework);
        event::emit_event(  
            &mut bridge_events.bridge_transfer_completed_events,
            BridgeTransferCompletedEvent {  
                bridge_transfer_id,  
                initiator,  
                recipient,  
                amount,  
                nonce,  
            },  
        );  
    }  

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 0x10006, location = 0x1::coin)] //EINSUFFICIENT_BALANCE
    fun test_initiate_bridge_transfer_insufficient_balance(
        sender: &signer,
        aptos_framework: &signer,
    ) acquires BridgeEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        native_bridge::initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let amount = 1000;

        initiate_bridge_transfer(
            sender,
            recipient,
            amount
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_complete_bridge_transfer(aptos_framework: &signer) acquires BridgeEvents {
        initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        let initiator = valid_eip55();
        let recipient = @0xcafe;
        let amount = 1;
        let nonce = 5;

        // Create a bridge transfer ID algorithmically
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&initiator));
        vector::append(&mut combined_bytes, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes, bcs::to_bytes(&amount));
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));
        let bridge_transfer_id = keccak256(combined_bytes);

        // Create an account for our recipient
        aptos_account::create_account(recipient);

        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
            nonce
        );

        let bridge_events = borrow_global<BridgeEvents>(signer::address_of(aptos_framework));
        let complete_events = event::emitted_events_by_handle(&bridge_events.bridge_transfer_completed_events);

        // Assert that the event was emitted
        let expected_event = BridgeTransferCompletedEvent {
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
            nonce,
        };

        assert!(std::vector::contains(&complete_events, &expected_event), 0);
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    // #[expected_failure(abort_code = 0x1, location = 0x1::native_bridge_configuration)] // EINVALID_BRIDGE_OPERATOR
    fun test_complete_bridge_transfer_by_sender(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        native_bridge::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = valid_eip55();
        let amount = 1000;
        let account_balance = amount + 1;

        // Mint some coins
        native_bridge::mint(sender_address, account_balance);

        assert!(coin::balance<AptosCoin>(sender_address) == account_balance, 0);

        initiate_bridge_transfer(
            sender,
            recipient,
            amount
        );

        let bridge_events = borrow_global<BridgeEvents>(@aptos_framework);
        let bridge_transfer_initiated_events = event::emitted_events_by_handle(
            &bridge_initiator_events.bridge_transfer_initiated_events
        );   
        let bridge_transfer_initiated_event = vector::borrow(&bridge_transfer_initiated_events, 0);

        let bridge_transfer_id = bridge_transfer_initiated_event.bridge_transfer_id;
        
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = EINVALID_BRIDGE_TRANSFER_ID, location = 0x1::smart_table)] // ENOT_FOUND
    fun test_complete_bridge_with_errorneous_bridge_id_by_operator(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeEvents {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        native_bridge::initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);

        let bridge_transfer_id = b"guessing the id";

        // As operator I send a complete request and it should fail
        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id,
            valid_eip55(),
            sender_address,
            1000,
            1
        );
    }
}

module aptos_framework::native_bridge_store {
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

    friend aptos_framework::native_bridge;

    #[test_only]
    use std::hash::sha3_256;
    #[test_only]
    use aptos_framework::ethereum;
    #[test_only]
    use aptos_framework::native_bridge_configuration;

    /// Error codes
    const EINVALID_PRE_IMAGE : u64 = 0x1;
    const ENOT_PENDING_TRANSACTION : u64 = 0x2;
    const EZERO_AMOUNT : u64 = 0x3;
    const EINVALID_BRIDGE_TRANSFER_ID : u64 = 0x4;
    const ENATIVE_BRIDGE_NOT_ENABLED : u64 = 0x5;

    const MAX_U64 : u64 = 0xFFFFFFFFFFFFFFFF;

    struct AddressPair<Initiator: store, Recipient: store> has store, copy {
        initiator: Initiator,
        recipient: Recipient,
    }

    /// A smart table wrapper
    struct SmartTableWrapper<K, V> has key, store {
        inner: SmartTable<K, V>,
    }

    // Unique bridge store nonce
    struct Nonce has key {
        inner: u64
    }

    /// Details on the transfer
    struct OutboundBridgeTransfer<Initiator: store, Recipient: store> has store, copy {
        addresses: AddressPair<Initiator, Recipient>,
        amount: u64,
        nonce: u64
    }

    /// Initializes the initiators tables and nonce.
    ///
    /// @param aptos_framework The signer for Aptos framework.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        move_to(aptos_framework, Nonce {
            inner: 0,
        });

        let initiators = SmartTableWrapper<vector<u8>, OutboundBridgeTransfer<address, EthereumAddress>> {
            inner: smart_table::new(),
        };

        let noncesToBridgeTransferIds = SmartTableWrapper<u64, vector<u8>> {
            inner: smart_table::new(),
        };

        move_to(aptos_framework, initiators);
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
    /// @param nonce The unique nonce for the transfer.
    /// @return A `BridgeTransferDetails` object.
    /// @abort If the amount is zero or locks are invalid.
    public(friend) fun create_details<Initiator: store, Recipient: store>(initiator: Initiator, recipient: Recipient, amount: u64, nonce: u64)
        : OutboundBridgeTransfer<Initiator, Recipient> {
        assert!(amount > 0, EZERO_AMOUNT);

        OutboundBridgeTransfer {
            addresses: AddressPair {
                initiator,
                recipient
            },
            amount,
            nonce,
        }
    }

    /// Record details of a transfer
    ///
    /// @param bridge_transfer_id Bridge transfer ID.
    /// @param details The bridge transfer details
    public(friend) fun add<Initiator: store, Recipient: store>(bridge_transfer_id: vector<u8>, details: OutboundBridgeTransfer<Initiator, Recipient>) acquires SmartTableWrapper {
        assert!(features::abort_native_bridge_enabled(), ENATIVE_BRIDGE_NOT_ENABLED);

        assert_valid_bridge_transfer_id(&bridge_transfer_id);
        let table = borrow_global_mut<SmartTableWrapper<vector<u8>, OutboundBridgeTransfer<Initiator, Recipient>>>(@aptos_framework);
        smart_table::add(&mut table.inner, bridge_transfer_id, details);
    }

     /// Record details of a transfer
    ///
    /// @param bridge_transfer_id Bridge transfer ID.
    /// @param details The bridge transfer details
    public(friend) fun set_nonce_to_bridge_transfer_id<Initiator: store, Recipient: store>(nonce: u64, bridge_transefer_id: vector<u8>) acquires SmartTableWrapper {
        assert!(features::abort_native_bridge_enabled(), ENATIVE_BRIDGE_NOT_ENABLED);

        assert_valid_bridge_transfer_id(&bridge_transfer_id);
        let table = borrow_global_mut<SmartTableWrapper<u64, vector<u8>>>(@aptos_framework);
        smart_table::add(&mut table.inner, nonce, bridge_transfer_id);
    }

    /// Asserts that the bridge transfer ID is valid.
    ///
    /// @param bridge_transfer_id The bridge transfer ID to validate.
    /// @abort If the ID is invalid.
    public(friend) fun assert_valid_bridge_transfer_id(bridge_transfer_id: &vector<u8>) {
        assert!(vector::length(bridge_transfer_id) == 32, EINVALID_BRIDGE_TRANSFER_ID);
    }

    /// Validates and completes a bridge transfer by confirming the hash lock and state.
    ///
    /// @param hash_lock The hash lock used to validate the transfer.
    /// @param details The mutable reference to the bridge transfer details to be completed.
    /// @return A tuple containing the recipient and the amount of the transfer.
    /// @abort If the hash lock is invalid, the transfer is not pending, or the hash lock does not match.
    fun complete_details<Initiator: store, Recipient: store + copy>(hash_lock: vector<u8>, details: &mut OutboundBridgeTransfer<Initiator, Recipient>) : (Recipient, u64) {
        assert_valid_hash_lock(&hash_lock);
        assert_pending(details);
        assert_correct_hash_lock(details, hash_lock);
        assert_within_timelock(details);

        complete(details);

        (details.addresses.recipient, details.amount)
    }

    /// Generates a unique bridge transfer ID based on transfer details and nonce.
    ///
    /// @param details The bridge transfer details.
    /// @return The generated bridge transfer ID.
    public(friend) fun bridge_transfer_id<Initiator: store, Recipient: store>(details: &OutboundBridgeTransfer<Initiator, Recipient>) : vector<u8> acquires Nonce {
        let nonce = borrow_global_mut<Nonce>(@aptos_framework);
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.addresses.initiator));
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.addresses.recipient));
        vector::append(&mut combined_bytes, details.amount);
        vector::append(&mut combined_bytes, details.nonce);
        if (nonce.inner == MAX_U64) {
            nonce.inner = 0;  // Wrap around to 0 if at maximum value
        } else {
            nonce.inner = nonce.inner + 1;  // Safe to increment without overflow
        };
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce.inner));

        keccak256(combined_bytes)
    }

    /// Generates a unique bridge transfer ID based on transfer details and nonce.
    ///
    /// @param details The bridge transfer details.
    /// @return The generated bridge transfer ID.
    public(friend) fun bridge_transfer_id<Initiator: store, Recipient: store>(details: &OutboundBridgeTransfer<Initiator, Recipient>) : vector<u8> acquires Nonce {
        let nonce = borrow_global_mut<Nonce>(@aptos_framework);
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.addresses.initiator));
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.addresses.recipient));
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.amount));
        vector::append(&mut combined_bytes, bcs::to_bytes(&details.nonce));
        keccak256(combined_bytes)
    }

    #[view]
    /// Gets initiator bridge transfer details given a bridge transfer ID
    ///
    /// @param bridge_transfer_id A 32-byte vector of unsigned 8-bit integers.
    /// @return A `OutboundBridgeTransfer` struct.
    /// @abort If there is no transfer in the atomic bridge store.
    public fun get_bridge_transfer_details(
        bridge_transfer_id: vector<u8>
    ): OutboundBridgeTransfer<address, EthereumAddress> acquires SmartTableWrapper {
        get_bridge_transfer_details(bridge_transfer_id)
    }
    
    #[view]
    /// gets bridge_transfer_id from nonce
    /// @param nonce The nonce of the bridge transfer
    /// @return The bridge transfer id
    public fun get_bridge_transfer_id_from_nonce(nonce: u64): vector<u8> acquires SmartTableWrapper {
        let table = borrow_global<SmartTableWrapper<u64, vector<u8>>>(@aptos_framework);
        if (smart_table::contains(&table.inner, nonce)) {
            let bridge_transfer_id = smart_table::borrow(&table.inner, nonce);
            *bridge_transfer_id
        } else {
            0x0
        }
    }

    fun get_bridge_transfer_details<Initiator: store + copy, Recipient: store + copy>(bridge_transfer_id: vector<u8>
    ): OutboundBridgeTransfer<Initiator, Recipient> acquires SmartTableWrapper {
        let table = borrow_global<SmartTableWrapper<vector<u8>, OutboundBridgeTransfer<Initiator, Recipient>>>(@aptos_framework);

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
            vector[features::get_native_bridge_feature()],
            vector[]
        );
        native_bridge_configuration::initialize(aptos_framework);
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

        let OutboundBridgeTransfer {
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
            vector[features::get_native_bridge_feature()],
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

        let OutboundBridgeTransfer {
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

module aptos_framework::native_bridge_configuration {
    use std::signer;
    use aptos_framework::event;
    use aptos_framework::system_addresses;

    friend aptos_framework::native_bridge;

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
    #[expected_failure(abort_code = 0x1, location = 0x1::native_bridge_configuration)]
    /// Tests that an incorrect operator is not validated and results in an abort.
    fun test_is_not_valid_operator(aptos_framework: &signer, bad: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        assert_is_caller_operator(bad);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that an incorrect signer cannot update the initiator time lock
    fun test_not_able_to_set_initiator_time_lock(aptos_framework: &signer, bad: &signer) acquires BridgeConfig {
        initialize(aptos_framework);
        set_initiator_time_lock_duration(bad, 1);
    }
}

module aptos_framework::native_bridge_core {
    use std::features;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::native_bridge_configuration;
    use aptos_framework::native_bridge_store;
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

    friend aptos_framework::native_bridge;
    friend aptos_framework::genesis;

    const ENATIVE_BRIDGE_NOT_ENABLED : u64 = 0x1;

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
        native_bridge_configuration::initialize(aptos_framework);
        native_bridge_store::initialize(aptos_framework);
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
            vector[features::get_native_bridge_feature()],
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
        assert!(features::abort_native_bridge_enabled(), ENATIVE_BRIDGE_NOT_ENABLED);

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
        assert!(features::abort_native_bridge_enabled(), ENATIVE_BRIDGE_NOT_ENABLED);

        coin::burn_from(
            from,
            amount,
            &borrow_global<AptosCoinBurnCapability>(@aptos_framework).burn_cap,
        );
    }
}
