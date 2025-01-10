module aptos_framework::native_bridge {

    use std::features;
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_framework::ethereum::{Self, EthereumAddress};    
    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle}; 
    use aptos_framework::signer;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    #[test_only]
    use aptos_framework::aptos_account;
    #[test_only]
    use aptos_framework::ethereum::valid_eip55;
    use std::bcs;
    use std::vector;
    use aptos_std::aptos_hash::keccak256;
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::coin::{Self, BurnCapability, MintCapability};
    use aptos_framework::fungible_asset::{BurnRef, MintRef};
    #[test_only]
    use aptos_framework::aptos_coin;

    const ETRANSFER_ALREADY_PROCESSED: u64 = 1;
    const EINVALID_BRIDGE_TRANSFER_ID: u64 = 2;
    const EEVENT_NOT_FOUND: u64 = 3;
    const EINVALID_NONCE: u64 = 4;
    const EINVALID_AMOUNT: u64 = 5;
    const ENONCE_NOT_FOUND: u64 = 6;
    const EZERO_AMOUNT: u64 = 7;
    const ENATIVE_BRIDGE_NOT_ENABLED: u64 = 8;
    const EINCORRECT_NONCE: u64 = 9;
    const EID_NOT_FOUND: u64 = 10;
    const EINVALID_BRIDGE_RELAYER: u64 = 11;
    const ESAME_FEE: u64 = 0x2;
    const ESAME_VALUE: u64 = 0x3;
    const ERATE_LIMIT_EXCEEDED: u64 = 0x4;

    friend aptos_framework::genesis;

    #[event]
    /// Event emitted when the bridge relayer is updated.
    struct BridgeConfigRelayerUpdated has store, drop {
        old_relayer: address,
        new_relayer: address,
    }

    #[event]
    /// An event triggered upon change of bridgefee
    struct BridgeFeeChangedEvent has store, drop {
        old_bridge_fee: u64,
        new_bridge_fee: u64,
    }

    #[event]
    /// An event triggered upon change of insurance budget divider
    struct BridgeInsuranceBudgetDividerChangedEvent has store, drop {
        old_insurance_budget_divider: u64,
        new_insurance_budget_divider: u64,
    }

    #[event]
    /// An event triggered upon change of insurance fund
    struct BridgeInsuranceFundChangedEvent has store, drop {
        old_insurance_fund: address,
        new_insurance_fund: address,
    }

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
        bridge_transfer_initiated_events: EventHandle<BridgeTransferInitiatedEvent>,
        bridge_transfer_completed_events: EventHandle<BridgeTransferCompletedEvent>,
    }

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

    /// A nonce to ensure the uniqueness of bridge transfers
    struct Nonce has key {
        value: u64
    }

    struct OutboundRateLimitBudget has key, store {
        day: SmartTable<u64, u64>,
    }

    struct InboundRateLimitBudget has key, store {
        day: SmartTable<u64, u64>,
    }

    /// A smart table wrapper
    struct SmartTableWrapper<K, V> has key, store {
        inner: SmartTable<K, V>,
    }

    /// Details on the outbound transfer
    struct OutboundTransfer has store, copy {
        bridge_transfer_id: vector<u8>,
        initiator: address,
        recipient: EthereumAddress,
        amount: u64,
    }

    struct BridgeConfig has key {
        bridge_relayer: address,
        insurance_fund: address,
        insurance_budget_divider: u64,
        bridge_fee: u64,
    }

    /// Initializes the module and stores the `EventHandle`s in the resource.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);

        let bridge_config = BridgeConfig {
            bridge_relayer: signer::address_of(aptos_framework),
            insurance_fund: signer::address_of(aptos_framework),
            insurance_budget_divider: 4,
            bridge_fee: 40_000_000_000,
        };
        move_to(aptos_framework, bridge_config);

        // Ensure the nonce is not already initialized
        assert!(
            !exists<Nonce>(signer::address_of(aptos_framework)),
            2
        );

        // Create the Nonce resource with an initial value of 0
        move_to<Nonce>(aptos_framework, Nonce { 
            value: 0
        });
        

        move_to(aptos_framework, BridgeEvents {
            bridge_transfer_initiated_events: account::new_event_handle<BridgeTransferInitiatedEvent>(aptos_framework),
            bridge_transfer_completed_events: account::new_event_handle<BridgeTransferCompletedEvent>(aptos_framework),
        });
        system_addresses::assert_aptos_framework(aptos_framework);

        let outbound_rate_limit_budget = OutboundRateLimitBudget {
            day: smart_table::new(),
        };

        move_to(aptos_framework, outbound_rate_limit_budget);


        let inbound_rate_limit_budget = InboundRateLimitBudget {
            day: smart_table::new(),
        };

        move_to(aptos_framework, inbound_rate_limit_budget);

        let nonces_to_details = SmartTableWrapper<u64, OutboundTransfer> {
            inner: smart_table::new(),
        };

        move_to(aptos_framework, nonces_to_details);

        let ids_to_inbound_nonces = SmartTableWrapper<vector<u8>, u64> {
            inner: smart_table::new(),
        };

        move_to(aptos_framework, ids_to_inbound_nonces);
    }

    /// Converts a u64 to a 32-byte vector.
    /// 
    /// @param value The u64 value to convert.
    /// @return A 32-byte vector containing the u64 value in little-endian order.
    /// 
    /// How BCS works: https://github.com/zefchain/bcs?tab=readme-ov-file#booleans-and-integers
    /// 
    /// @example: a u64 value 0x12_34_56_78_ab_cd_ef_00 is converted to a 32-byte vector:
    /// [0x00, 0x00, ..., 0x00, 0x12, 0x34, 0x56, 0x78, 0xab, 0xcd, 0xef, 0x00]
    public(friend) fun normalize_u64_to_32_bytes(value: &u64): vector<u8> {
        let r = bcs::to_bytes(&(*value as u256));
        // BCS returns the bytes in reverse order, so we reverse the result.
        vector::reverse(&mut r);
        r
    }

    /// Checks if a bridge transfer ID is associated with an inbound nonce.
    /// @param bridge_transfer_id The bridge transfer ID.
    /// @return `true` if the ID is associated with an existing inbound nonce, `false` otherwise.
    public(friend) fun is_inbound_nonce_set(bridge_transfer_id: vector<u8>): bool acquires SmartTableWrapper {
        let table = borrow_global<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework);
        smart_table::contains(&table.inner, bridge_transfer_id)
    }

    /// Creates bridge transfer details with validation.
    ///
    /// @param initiator The initiating party of the transfer.
    /// @param recipient The receiving party of the transfer.
    /// @param amount The amount to be transferred.
    /// @param nonce The unique nonce for the transfer.
    /// @return A `BridgeTransferDetails` object.
    /// @abort If the amount is zero or locks are invalid.
    public(friend) fun create_details(initiator: address, recipient: EthereumAddress, amount: u64, nonce: u64)
        : OutboundTransfer {
        assert!(amount > 0, EZERO_AMOUNT);

        // Create a bridge transfer ID algorithmically
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, bcs::to_bytes(&initiator));
        vector::append(&mut combined_bytes, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes, bcs::to_bytes(&amount));
        vector::append(&mut combined_bytes, bcs::to_bytes(&nonce));
        let bridge_transfer_id = keccak256(combined_bytes);

        OutboundTransfer {
            bridge_transfer_id,
            initiator,
            recipient,
            amount,
        }
    }

    /// Record details of an initiated transfer for quick lookup of details, mapping bridge transfer ID to transfer details 
    ///
    /// @param bridge_transfer_id Bridge transfer ID.
    /// @param details The bridge transfer details
    public(friend) fun add(nonce: u64, details: OutboundTransfer) acquires SmartTableWrapper {
        assert!(features::abort_native_bridge_enabled(), ENATIVE_BRIDGE_NOT_ENABLED);

        let table = borrow_global_mut<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework);
        smart_table::add(&mut table.inner, nonce, details);
    }

    /// Record details of a completed transfer, mapping bridge transfer ID to inbound nonce
    ///
    /// @param bridge_transfer_id Bridge transfer ID.
    /// @param details The bridge transfer details
    public(friend) fun set_bridge_transfer_id_to_inbound_nonce(bridge_transfer_id: vector<u8>, inbound_nonce: u64) acquires SmartTableWrapper {
        assert!(features::abort_native_bridge_enabled(), ENATIVE_BRIDGE_NOT_ENABLED);

        assert_valid_bridge_transfer_id(&bridge_transfer_id);
        let table = borrow_global_mut<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework);
        smart_table::add(&mut table.inner, bridge_transfer_id, inbound_nonce);
    }

    /// Asserts that the bridge transfer ID is valid.
    ///
    /// @param bridge_transfer_id The bridge transfer ID to validate.
    /// @abort If the ID is invalid.
    public(friend) fun assert_valid_bridge_transfer_id(bridge_transfer_id: &vector<u8>) {
        assert!(vector::length(bridge_transfer_id) == 32, EINVALID_BRIDGE_TRANSFER_ID);
    }

    /// Generates a unique outbound bridge transfer ID based on transfer details and nonce.
    ///
    /// @param details The bridge transfer details.
    /// @return The generated bridge transfer ID.
    public(friend) fun bridge_transfer_id(initiator: address, recipient: EthereumAddress, amount: u64, nonce: u64) : vector<u8> {
        // Serialize each param
        let initiator_bytes = bcs::to_bytes<address>(&initiator);
        let recipient_bytes = ethereum::get_inner_ethereum_address(recipient);
        let amount_bytes = normalize_u64_to_32_bytes(&amount);
        let nonce_bytes = normalize_u64_to_32_bytes(&nonce);
        //Contatenate then hash and return bridge transfer ID
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, initiator_bytes);
        vector::append(&mut combined_bytes, recipient_bytes);
        vector::append(&mut combined_bytes, amount_bytes);
        vector::append(&mut combined_bytes, nonce_bytes);
        keccak256(combined_bytes)
    }

    #[view]
    /// Retrieves the address of the current bridge relayer.
    ///
    /// @return The address of the current bridge relayer.
    public fun bridge_relayer(): address acquires BridgeConfig {
        borrow_global_mut<BridgeConfig>(@aptos_framework).bridge_relayer
    }

    #[view]
    /// Retrieves the address of the current insurance fund.
    /// 
    /// @return The address of the current insurance fund.
    public fun insurance_fund(): address acquires BridgeConfig {
        borrow_global_mut<BridgeConfig>(@aptos_framework).insurance_fund
    }

    #[view]
    /// Retrieves the current insurance budget divider.
    /// 
    /// @return The current insurance budget divider.
    public fun insurance_budget_divider(): u64 acquires BridgeConfig {
        borrow_global_mut<BridgeConfig>(@aptos_framework).insurance_budget_divider
    }

    #[view]
    /// Retrieves the current bridge fee.
    /// 
    /// @return The current bridge fee.
    public fun bridge_fee(): u64 acquires BridgeConfig {
        borrow_global_mut<BridgeConfig>(@aptos_framework).bridge_fee
    }
    
    #[view]
    /// Gets the bridge transfer details (`OutboundTransfer`) from the given nonce.
    /// @param nonce The nonce of the bridge transfer.
    /// @return The `OutboundTransfer` struct containing the transfer details.
    /// @abort If the nonce is not found in the smart table.
    public fun get_bridge_transfer_details_from_nonce(nonce: u64): OutboundTransfer acquires SmartTableWrapper {
        let table = borrow_global<SmartTableWrapper<u64, OutboundTransfer>>(@aptos_framework);
        
        // Check if the nonce exists in the table
        assert!(smart_table::contains(&table.inner, nonce), ENONCE_NOT_FOUND);

        // If it exists, return the associated `OutboundTransfer` details
        *smart_table::borrow(&table.inner, nonce)
    }

    #[view]
    /// Gets inbound `nonce` from `bridge_transfer_id`
    /// @param bridge_transfer_id The ID bridge transfer.
    /// @return the nonce
    /// @abort If the nonce is not found in the smart table.
    public fun get_inbound_nonce_from_bridge_transfer_id(bridge_transfer_id: vector<u8>): u64 acquires SmartTableWrapper {
        let table = borrow_global<SmartTableWrapper<vector<u8>, u64>>(@aptos_framework);

         // Check if the nonce exists in the table
        assert!(smart_table::contains(&table.inner, bridge_transfer_id), ENONCE_NOT_FOUND);

        // If it exists, return the associated nonce
        *smart_table::borrow(&table.inner, bridge_transfer_id)
    }

    /// Increment and get the current nonce  
    fun increment_and_get_nonce(): u64 acquires Nonce {  
        let nonce_ref = borrow_global_mut<Nonce>(@aptos_framework);  
        nonce_ref.value = nonce_ref.value + 1;  
        nonce_ref.value  
    } 

    #[test_only]
    /// Initializes the native bridge for testing purposes
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    public fun initialize_for_test(aptos_framework: &signer) {
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

    /// Initiate a bridge transfer of MOVE from Movement to Ethereum
    /// Anyone can initiate a bridge transfer from the source chain  
    /// The amount is burnt from the initiator and the module-level nonce is incremented  
    /// @param initiator The initiator's Ethereum address as a vector of bytes.  
    /// @param recipient The address of the recipient on the Aptos blockchain.  
    /// @param amount The amount of assets to be locked.  
    public entry fun initiate_bridge_transfer(  
        initiator: &signer,  
        recipient: vector<u8>,  
        amount: u64  
    ) acquires BridgeEvents, Nonce, AptosCoinBurnCapability, AptosCoinMintCapability, SmartTableWrapper, OutboundRateLimitBudget, BridgeConfig {
        let initiator_address = signer::address_of(initiator);  
        let ethereum_address = ethereum::ethereum_address_20_bytes(recipient);

        // Ensure the amount is enough for the bridge fee and charge for it
        let new_amount = charge_bridge_fee(amount);

        assert_outbound_rate_limit_budget_not_exceeded(new_amount);

        // Increment and retrieve the nonce  
        let nonce = increment_and_get_nonce();  

        // Create bridge transfer details  
        let details = create_details(  
            initiator_address,  
            ethereum_address, 
            new_amount,  
            nonce  
        );  

        let bridge_transfer_id = bridge_transfer_id(
            initiator_address, 
            ethereum_address, 
            new_amount, 
            nonce
        ); 
    
        // Add the transfer details to storage  
        add(nonce, details);

        // Burn the amount from the initiator  
        burn(initiator_address, amount);  

        let bridge_events = borrow_global_mut<BridgeEvents>(@aptos_framework);

        // Emit an event with nonce
        event::emit_event(  
             &mut bridge_events.bridge_transfer_initiated_events,
            BridgeTransferInitiatedEvent {  
                bridge_transfer_id,  
                initiator: initiator_address,
                recipient,  
                amount: new_amount,
                nonce,  
            }  
        );  
    }

    /// Completes a bridge transfer on the destination chain.
    ///  
    /// @param caller The signer representing the bridge relayer.  
    /// @param initiator The initiator's Ethereum address as a vector of bytes.  
    /// @param bridge_transfer_id The unique identifier for the bridge transfer.  
    /// @param recipient The address of the recipient on the Aptos blockchain.  
    /// @param amount The amount of assets to be locked.  
    /// @param nonce The unique nonce for the transfer.    
    /// @abort If the caller is not the bridge relayer or the transfer has already been processed.  
    public entry fun complete_bridge_transfer(
        caller: &signer,
        bridge_transfer_id: vector<u8>,
        initiator: vector<u8>,
        recipient: address,
        amount: u64,
        nonce: u64
    ) acquires BridgeEvents, AptosCoinMintCapability, SmartTableWrapper, InboundRateLimitBudget, BridgeConfig {
        // Ensure the caller is the bridge relayer
        assert_is_caller_relayer(caller);
        assert_inbound_rate_limit_budget_not_exceeded(amount);

        // Check if the bridge transfer ID is already associated with an inbound nonce
        let inbound_nonce_exists = is_inbound_nonce_set(bridge_transfer_id);
        assert!(!inbound_nonce_exists, ETRANSFER_ALREADY_PROCESSED);
        assert!(nonce > 0, EINVALID_NONCE);

        // Validate the bridge_transfer_id by reconstructing the hash
        let recipient_bytes = bcs::to_bytes(&recipient);
        let amount_bytes = normalize_u64_to_32_bytes(&amount);
        let nonce_bytes = normalize_u64_to_32_bytes(&nonce);

        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, initiator);
        vector::append(&mut combined_bytes, recipient_bytes);
        vector::append(&mut combined_bytes, amount_bytes);
        vector::append(&mut combined_bytes, nonce_bytes);

        assert!(keccak256(combined_bytes) == bridge_transfer_id, EINVALID_BRIDGE_TRANSFER_ID);

        // Record the transfer as completed by associating the bridge_transfer_id with the inbound nonce
        set_bridge_transfer_id_to_inbound_nonce(bridge_transfer_id, nonce);

        // Mint to the recipient
        mint(recipient, amount);

        // Emit the event
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

    /// Charge bridge fee to the initiate bridge transfer.
    /// 
    /// @param initiator The signer representing the initiator.
    /// @param amount The amount to be charged.
    /// @return The new amount after deducting the bridge fee.
    fun charge_bridge_fee(amount: u64) : u64 acquires AptosCoinMintCapability, BridgeConfig {
        let bridge_fee = bridge_fee();
        let bridge_relayer = bridge_relayer();
        assert!(amount > bridge_fee, EINVALID_AMOUNT);
        let new_amount = amount - bridge_fee;
        mint(bridge_relayer, bridge_fee);
        new_amount
    }

    /// Updates the bridge relayer, requiring governance validation.
    ///
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param new_relayer The new address to be set as the bridge relayer.
    /// @abort If the current relayer is the same as the new relayer.
    public fun update_bridge_relayer(aptos_framework: &signer, new_relayer: address
    )   acquires BridgeConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let bridge_config = borrow_global_mut<BridgeConfig>(@aptos_framework);
        let old_relayer = bridge_config.bridge_relayer;
        assert!(old_relayer != new_relayer, EINVALID_BRIDGE_RELAYER);

        bridge_config.bridge_relayer = new_relayer;

        event::emit(
            BridgeConfigRelayerUpdated {
                old_relayer,
                new_relayer,
            },
        );
    }

    /// Updates the bridge fee, requiring relayer validation.
    /// 
    /// @param relayer The signer representing the Relayer.
    /// @param new_bridge_fee The new bridge fee to be set.
    /// @abort If the new bridge fee is the same as the old bridge fee.
    public entry fun update_bridge_fee(relayer: &signer, new_bridge_fee: u64
    ) acquires BridgeConfig {
        assert_is_caller_relayer(relayer);
        let bridge_config = borrow_global_mut<BridgeConfig>(@aptos_framework);
        let old_bridge_fee = bridge_config.bridge_fee;
        assert!(old_bridge_fee != new_bridge_fee, ESAME_FEE);
        bridge_config.bridge_fee = new_bridge_fee;

        event::emit(
            BridgeFeeChangedEvent {
                old_bridge_fee,
                new_bridge_fee,
            },
        );
    }

    /// Updates the insurance fund, requiring governance validation.
    /// 
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param new_insurance_fund The new insurance fund to be set.
    /// @abort If the new insurance fund is the same as the old insurance fund.
    public entry fun update_insurance_fund(aptos_framework: &signer, new_insurance_fund: address
    ) acquires BridgeConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let bridge_config = borrow_global_mut<BridgeConfig>(@aptos_framework);
        let old_insurance_fund = bridge_config.insurance_fund;
        assert!(old_insurance_fund != new_insurance_fund, ESAME_VALUE);
        bridge_config.insurance_fund = new_insurance_fund;

        event::emit(
            BridgeInsuranceFundChangedEvent {
                old_insurance_fund,
                new_insurance_fund,
            },
        );
    }

    /// Updates the insurance budget divider, requiring governance validation.
    /// 
    /// @param aptos_framework The signer representing the Aptos framework.
    /// @param new_insurance_budget_divider The new insurance budget divider to be set.
    /// @abort If the new insurance budget divider is the same as the old insurance budget divider.
    public entry fun update_insurance_budget_divider(aptos_framework: &signer, new_insurance_budget_divider: u64
    ) acquires BridgeConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        let bridge_config = borrow_global_mut<BridgeConfig>(@aptos_framework);
        let old_insurance_budget_divider = bridge_config.insurance_budget_divider;
        assert!(old_insurance_budget_divider != new_insurance_budget_divider, ESAME_VALUE);
        bridge_config.insurance_budget_divider = new_insurance_budget_divider;

        event::emit(
            BridgeInsuranceBudgetDividerChangedEvent {
                old_insurance_budget_divider,
                new_insurance_budget_divider,
            },
        );
    }

    /// Asserts that the caller is the current bridge relayer.
    ///
    /// @param caller The signer whose authority is being checked.
    /// @abort If the caller is not the current bridge relayer.
    public(friend) fun assert_is_caller_relayer(caller: &signer
    ) acquires BridgeConfig {
        assert!(borrow_global<BridgeConfig>(@aptos_framework).bridge_relayer == signer::address_of(caller), EINVALID_BRIDGE_RELAYER);
    }

    /// Asserts that the rate limit budget is not exceeded.
    /// 
    /// @param amount The amount to be transferred.
    fun assert_outbound_rate_limit_budget_not_exceeded(amount: u64) acquires OutboundRateLimitBudget, BridgeConfig {
        let insurance_fund = borrow_global<BridgeConfig>(@aptos_framework).insurance_fund;
        let insurance_budget_divider = borrow_global<BridgeConfig>(@aptos_framework).insurance_budget_divider;
        let table = borrow_global_mut<OutboundRateLimitBudget>(@aptos_framework);
        
        let day = timestamp::now_seconds() / 86400;
        let current_budget = smart_table::borrow_mut_with_default(&mut table.day, day, 0);
        smart_table::upsert(&mut table.day, day, *current_budget + amount);
        let rate_limit = coin::balance<AptosCoin>(insurance_fund) / insurance_budget_divider;
        assert!(*smart_table::borrow(&table.day, day) < rate_limit, ERATE_LIMIT_EXCEEDED);
    }

    /// Asserts that the rate limit budget is not exceeded.
    /// 
    /// @param amount The amount to be transferred.
    fun assert_inbound_rate_limit_budget_not_exceeded(amount: u64) acquires InboundRateLimitBudget, BridgeConfig {
        let insurance_fund = borrow_global<BridgeConfig>(@aptos_framework).insurance_fund;
        let insurance_budget_divider = borrow_global<BridgeConfig>(@aptos_framework).insurance_budget_divider;
        let table = borrow_global_mut<InboundRateLimitBudget>(@aptos_framework);
        
        let day = timestamp::now_seconds() / 86400;
        let current_budget = smart_table::borrow_mut_with_default(&mut table.day, day, 0);
        smart_table::upsert(&mut table.day, day, *current_budget + amount);
        let rate_limit = coin::balance<AptosCoin>(insurance_fund) / insurance_budget_divider;
        assert!(*smart_table::borrow(&table.day, day) < rate_limit, ERATE_LIMIT_EXCEEDED);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests initialization of the bridge configuration.
    fun test_initialization(aptos_framework: &signer) {
        initialize_for_test(aptos_framework);
        assert!(exists<BridgeConfig>(@aptos_framework), 0);
    }

    #[test(aptos_framework = @aptos_framework, new_relayer = @0xcafe)]
    /// Tests updating the bridge relayer and emitting the corresponding event.
    fun test_update_bridge_relayer(aptos_framework: &signer, new_relayer: address
    ) acquires BridgeConfig {
        initialize_for_test(aptos_framework);
        update_bridge_relayer(aptos_framework, new_relayer);

        assert!(
            event::was_event_emitted<BridgeConfigRelayerUpdated>(
                &BridgeConfigRelayerUpdated {
                    old_relayer: @aptos_framework,
                    new_relayer,
                }
            ), 0);

        assert!(bridge_relayer() == new_relayer, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests updating the insurance budget divider and emitting the corresponding event.
    fun test_update_insurance_budget_divider(aptos_framework: &signer
    ) acquires BridgeConfig {
        initialize_for_test(aptos_framework);
        let old_insurance_budget_divider = insurance_budget_divider();
        let new_insurance_budget_divider = 5;
        update_insurance_budget_divider(aptos_framework, new_insurance_budget_divider);

        assert!(
            event::was_event_emitted<BridgeInsuranceBudgetDividerChangedEvent>(
                &BridgeInsuranceBudgetDividerChangedEvent {
                    old_insurance_budget_divider: old_insurance_budget_divider,
                    new_insurance_budget_divider: new_insurance_budget_divider,
                }
            ), 0);

        assert!(insurance_budget_divider() == new_insurance_budget_divider, 0);
    }

    #[test(aptos_framework = @aptos_framework, new_insurance_fund = @0xdead)]
    /// Tests updating the insurance fund and emitting the corresponding event.
    fun test_update_insurance_fund(aptos_framework: &signer, new_insurance_fund: address
    ) acquires BridgeConfig {
        initialize_for_test(aptos_framework);
        update_insurance_fund(aptos_framework, new_insurance_fund);

        assert!(
            event::was_event_emitted<BridgeInsuranceFundChangedEvent>(
                &BridgeInsuranceFundChangedEvent {
                    old_insurance_fund: @aptos_framework,
                    new_insurance_fund: new_insurance_fund,
                }
            ), 0);

        assert!(insurance_fund() == new_insurance_fund, 0);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests updating the bridge relayer and emitting the corresponding event.
    fun test_update_bridge_fee(aptos_framework: &signer
    ) acquires BridgeConfig {
        let new_fee = 100;
        initialize_for_test(aptos_framework);
        let old_bridge_fee = bridge_fee();
        update_bridge_fee(aptos_framework, new_fee);

        assert!(
            event::was_event_emitted<BridgeFeeChangedEvent>(
                &BridgeFeeChangedEvent {
                    old_bridge_fee: old_bridge_fee,
                    new_bridge_fee: new_fee,
                }
            ), 0);

        assert!(bridge_fee() == new_fee, 0);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad, new_relayer = @0xcafe)]
    #[expected_failure(abort_code = 0x50003, location = 0x1::system_addresses)]
    /// Tests that updating the bridge relayer with an invalid signer fails.
    fun test_failing_update_bridge_relayer(aptos_framework: &signer, bad: &signer, new_relayer: address
    ) acquires BridgeConfig {
        initialize_for_test(aptos_framework);
        update_bridge_relayer(bad, new_relayer);
    }

    #[test(aptos_framework = @aptos_framework)]
    /// Tests that the correct relayer is validated successfully.
    fun test_is_valid_relayer(aptos_framework: &signer) acquires BridgeConfig {
        initialize_for_test(aptos_framework);
        assert_is_caller_relayer(aptos_framework);
    }

    #[test(aptos_framework = @aptos_framework, bad = @0xbad)]
    #[expected_failure(abort_code = 11, location = Self)]
    /// Tests that an incorrect relayer is not validated and results in an abort.
    fun test_is_not_valid_relayer(aptos_framework: &signer, bad: &signer) acquires BridgeConfig {
        initialize_for_test(aptos_framework);
        assert_is_caller_relayer(bad);
    }

    #[test(aptos_framework = @aptos_framework, relayer = @0xcafe, sender = @0x726563697069656e740000000000000000000000000000000000000000000000, insurance_fund = @0xbeaf)]
    fun test_initiate_bridge_transfer_happy_path(
        sender: &signer,
        aptos_framework: &signer,
        relayer: &signer,
        insurance_fund: &signer
    ) acquires BridgeEvents, Nonce, AptosCoinMintCapability, AptosCoinBurnCapability, SmartTableWrapper, OutboundRateLimitBudget, BridgeConfig {
        let sender_address = signer::address_of(sender);
        let relayer_address = signer::address_of(relayer);
        initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);
        let amount = 1000;
        let bridge_fee = 40;
        update_bridge_fee(aptos_framework, bridge_fee);
        let insurance_fund_address = signer::address_of(insurance_fund);
        aptos_account::create_account(insurance_fund_address);
        update_insurance_fund(aptos_framework, insurance_fund_address);        

        // grant the insurance fund 4 * amount of coins
        mint(insurance_fund_address, amount * 4 + 4);

        timestamp::set_time_has_started_for_testing(aptos_framework);
        
        
        // Update the bridge relayer so it can receive the bridge fee
        update_bridge_relayer(aptos_framework, relayer_address);
        let bridge_relayer = bridge_relayer();
        aptos_account::create_account(bridge_relayer);

        // Mint coins to the sender to ensure they have sufficient balance
        let account_balance = amount + 1;
        // Mint some coins
        mint(sender_address, account_balance);

        // Specify the recipient and transfer amount
        let recipient = ethereum::eth_address_20_bytes();

        // Perform the bridge transfer
        initiate_bridge_transfer(
            sender,
            recipient,
            amount
        );

        let bridge_events = borrow_global<BridgeEvents>(@aptos_framework);
        let initiated_events = event::emitted_events_by_handle(
            &bridge_events.bridge_transfer_initiated_events
        );
        assert!(vector::length(&initiated_events) == 1, EEVENT_NOT_FOUND);
        let first_elem = vector::borrow(&initiated_events, 0);
        assert!(first_elem.amount == amount - bridge_fee, 0);
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff, relayer = @0xcafe, insurance_fund = @0xbeaf)]
    #[expected_failure(abort_code = 0x10006, location = 0x1::coin)] 
    fun test_initiate_bridge_transfer_insufficient_balance(
        sender: &signer,
        aptos_framework: &signer,
        relayer: &signer,
        insurance_fund: &signer
    ) acquires BridgeEvents, Nonce, AptosCoinBurnCapability, AptosCoinMintCapability, SmartTableWrapper, OutboundRateLimitBudget, BridgeConfig {
        let sender_address = signer::address_of(sender);
        let relayer_address = signer::address_of(relayer);
        let insurance_fund_address = signer::address_of(insurance_fund);
        initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);

        let recipient = ethereum::eth_address_20_bytes();
        let amount = 1000;
        let bridge_fee = 40;
        aptos_account::create_account(insurance_fund_address);
        update_insurance_fund(aptos_framework, insurance_fund_address);
        

        // grant the insurance fund 4 * amount of coins
        mint(insurance_fund_address, amount * 4 + 4);

        timestamp::set_time_has_started_for_testing(aptos_framework);
        update_bridge_fee(aptos_framework, bridge_fee);

        // Update the bridge relayer so it can receive the bridge fee
        update_bridge_relayer(aptos_framework, relayer_address);
        let bridge_relayer = bridge_relayer();
        aptos_account::create_account(bridge_relayer);

        initiate_bridge_transfer(
            sender,
            recipient,
            amount
        );
    }

    #[test(aptos_framework = @aptos_framework)]
    fun test_complete_bridge_transfer(aptos_framework: &signer) acquires BridgeEvents, AptosCoinMintCapability, SmartTableWrapper, InboundRateLimitBudget, BridgeConfig {
        initialize_for_test(aptos_framework);
        let initiator = b"5B38Da6a701c568545dCfcB03FcB875f56beddC4";
        let recipient = @0x726563697069656e740000000000000000000000000000000000000000000000;
        let insurance_fund = @0xbeaf;
        let amount = 100;
        let nonce = 1;

        // Create a bridge transfer ID algorithmically
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, initiator);
        vector::append(&mut combined_bytes, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes, normalize_u64_to_32_bytes(&amount));
        vector::append(&mut combined_bytes, normalize_u64_to_32_bytes(&nonce));
        let bridge_transfer_id = keccak256(combined_bytes);

        aptos_account::create_account(insurance_fund);
        update_insurance_fund(aptos_framework, insurance_fund);

        // grant the insurance fund 4 * amount of coins
        mint(insurance_fund, amount * 4 + 4);

        // Create an account for our recipient
        aptos_account::create_account(recipient);
        timestamp::set_time_has_started_for_testing(aptos_framework);
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
        assert!(bridge_transfer_id == expected_event.bridge_transfer_id, 0)
    }

    #[test(aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 4, location = Self)] 
    fun test_complete_bridge_transfer_rate_limit(aptos_framework: &signer) acquires BridgeEvents, AptosCoinMintCapability, SmartTableWrapper, InboundRateLimitBudget, BridgeConfig {
        initialize_for_test(aptos_framework);
        let initiator = b"5B38Da6a701c568545dCfcB03FcB875f56beddC4";
        let recipient = @0x726563697069656e740000000000000000000000000000000000000000000000;
        let insurance_fund = @0xbeaf;
        let day = 86400;
        let amount = 100;
        let nonce = 1;

        // Create a bridge transfer ID algorithmically
        let combined_bytes = vector::empty<u8>();
        vector::append(&mut combined_bytes, initiator);
        vector::append(&mut combined_bytes, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes, normalize_u64_to_32_bytes(&amount));
        vector::append(&mut combined_bytes, normalize_u64_to_32_bytes(&nonce));
        let bridge_transfer_id = keccak256(combined_bytes);

        aptos_account::create_account(insurance_fund);
        update_insurance_fund(aptos_framework, insurance_fund);

        // grant the insurance fund 4 * amount of coins
        mint(insurance_fund, amount * 4 + 4);

        // Create an account for our recipient
        aptos_account::create_account(recipient);
        timestamp::set_time_has_started_for_testing(aptos_framework);
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
        assert!(bridge_transfer_id == expected_event.bridge_transfer_id, 0);

        // reset the rate limit
        timestamp::fast_forward_seconds(day);

        nonce = nonce + 1;
        let combined_bytes2 = vector::empty<u8>();
        vector::append(&mut combined_bytes2, initiator);
        vector::append(&mut combined_bytes2, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes2, normalize_u64_to_32_bytes(&amount));
        vector::append(&mut combined_bytes2, normalize_u64_to_32_bytes(&nonce));
        let bridge_transfer_id2 = keccak256(combined_bytes2);
        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id2,
            initiator,
            recipient,
            amount,
            nonce
        );
        
        nonce = nonce + 1;

        let combined_bytes3 = vector::empty<u8>();
        vector::append(&mut combined_bytes3, initiator);
        vector::append(&mut combined_bytes3, bcs::to_bytes(&recipient));
        vector::append(&mut combined_bytes3, normalize_u64_to_32_bytes(&amount));
        vector::append(&mut combined_bytes3, normalize_u64_to_32_bytes(&nonce));
        let bridge_transfer_id3 = keccak256(combined_bytes2);
        // expect to fail as it reaches the rate limit
        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id3,
            initiator,
            recipient,
            amount,
            nonce
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = 11, location = Self)] 
    fun test_complete_bridge_transfer_by_non_relayer(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeEvents, AptosCoinMintCapability, SmartTableWrapper, InboundRateLimitBudget, BridgeConfig {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);

        let bridge_transfer_id = b"guessing the id";

        // As relayer I send a complete request and it should fail
        complete_bridge_transfer(
            sender,
            bridge_transfer_id,
            valid_eip55(),
            sender_address,
            1000,
            1
        );
    }

    #[test(aptos_framework = @aptos_framework, sender = @0xdaff)]
    #[expected_failure(abort_code = EINVALID_BRIDGE_TRANSFER_ID, location = Self)] // ENOT_FOUND
    fun test_complete_bridge_with_erroneous_bridge_id_by_relayer(
        sender: &signer,
        aptos_framework: &signer
    ) acquires BridgeEvents, AptosCoinMintCapability, SmartTableWrapper, InboundRateLimitBudget, BridgeConfig {
        let sender_address = signer::address_of(sender);
        // Create an account for our recipient
        initialize_for_test(aptos_framework);
        aptos_account::create_account(sender_address);
        let insurance_fund = @0xbeaf;

        let bridge_transfer_id = b"guessing the id";

        aptos_account::create_account(insurance_fund);
        update_insurance_fund(aptos_framework, insurance_fund);

        // grant the insurance fund 4 * amount of coins
        mint(insurance_fund, 1000 * 4 + 4);

        timestamp::set_time_has_started_for_testing(aptos_framework);
        // As relayer I send a complete request and it should fail
        complete_bridge_transfer(
            aptos_framework,
            bridge_transfer_id,
            valid_eip55(),
            sender_address,
            1000,
            1
        );
    }

    #[test]
    /// Test normalisation (serialization) of u64 to 32 bytes
    fun test_normalize_u64_to_32_bytes() {
        test_normalize_u64_to_32_bytes_helper(0x64, 
            vector[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0x64]);
        test_normalize_u64_to_32_bytes_helper(0x6400, 
            vector[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0x64,0x00]);
        test_normalize_u64_to_32_bytes_helper(0x00_32_00_00_64_00, 
            vector[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0x32,0,0,0x64,0x00]);
    }

    /// Test serialization of u64 to 32 bytes
    fun test_normalize_u64_to_32_bytes_helper(x: u64, expected: vector<u8>) {
        let r = normalize_u64_to_32_bytes(&x);
        assert!(vector::length(&r) == 32, 0);
        assert!(r == expected, 0);
    }
}
