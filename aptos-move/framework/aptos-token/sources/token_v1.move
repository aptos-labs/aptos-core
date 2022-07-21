/// This module provides the foundation for Tokens.
module aptos_token::token_v1 {
    use std::string::{String, Self};
    use std::error;
    use aptos_std::event::{Self, EventHandle};
    use std::signer;
    use std::vector;

    use aptos_std::table::{Self, Table};
    use aptos_token::property_map::{Self, PropertyMap};

    const TOKEN_MAX_MUTABLE_IND: u64 = 0;
    const TOKEN_URI_MUTABLE_IND: u64 = 1;
    const TOKEN_DESCRIPTION_MUTABLE_IND: u64 = 2;
    const TOKEN_ROYALTY_MUTABLE_IND: u64 = 3;
    const TOKEN_PROPERTY_MUTABLE_IND: u64 = 4;
    const TOKEN_PROPERTY_VALUE_MUTABLE_IND: u64 = 5;

    const COLLECTION_DESCRIPTION_MUTABLE_IND: u64 = 0;
    const COLLECTION_URI_MUTABLE_IND: u64 = 1;
    const COLLECTION_MAX_MUTABLE_IND: u64 = 2;

    const EALREADY_HAS_BALANCE: u64 = 0;
    const EBALANCE_NOT_PUBLISHED: u64 = 1;
    const ECOLLECTIONS_NOT_PUBLISHED: u64 = 2;
    const ECOLLECTION_NOT_PUBLISHED: u64 = 3;
    const ECOLLECTION_ALREADY_EXISTS: u64 = 4;
    const ECREATE_WOULD_EXCEED_MAXIMUM: u64 = 5;
    const EINSUFFICIENT_BALANCE: u64 = 6;
    const EINVALID_COLLECTION_NAME: u64 = 7;
    const EINVALID_TOKEN_MERGE: u64 = 8;
    const EMINT_WOULD_EXCEED_MAXIMUM: u64 = 9;
    const ENO_BURN_CAPABILITY: u64 = 10;
    const ENO_MINT_CAPABILITY: u64 = 11;
    const ETOKEN_ALREADY_EXISTS: u64 = 12;
    const ETOKEN_NOT_PUBLISHED: u64 = 13;
    const ETOKEN_STORE_NOT_PUBLISHED: u64 = 14;
    const ETOKEN_SPLIT_AMOUNT_LARGER_THEN_TOKEN_AMOUNT: u64 = 15;
    const EFIELD_NOT_MUTABLE: u64 = 16;
    const EBURNCAP_EXISTS_OR_CREATED_FOR_TOKEN: u64 = 17;
    const EONLY_CREATOR_CAN_CREATE_BURN_CAP: u64 = 18;
    const EONLY_CREATOR_CAN_DELEGATE_BURN_CAP: u64 = 19;
    const ETOKEN_CAPABILITY_STORE_NOT_EXISTS: u64 = 20;
    const ETOKEN_NOT_EXISTS_IN_CAPABILITY_STORE: u64 = 21;
    const EONLY_TOKEN_OWNER_CAN_HAVE_BURN_CAP: u64 = 22;
    const ENOT_OWN_THE_CAPABILITY: u64 = 23;
    const ENO_MUTATE_CAPABILITY: u64 = 24;
    const ETOKEN_SHOULDNOT_EXIST_IN_TOKEN_STORE: u64 = 25;


    //
    // Core data structures for holding tokens
    //

    struct Token has store {
        id: TokenId,
        // the amount of tokens. Only serial_number = 0 can have a value bigger than 1.
        value: u64,
    }

    /// global unique identifier of a token
    struct TokenId has store, copy, drop {
        // the id to the common token data shared by token with different serial number
        token_data_id: TokenDataId,
        // the serial_number of a token. Token with dfiferent serial number can have different value of PropertyMap
        serial_number: u64,
    }

    /// globally unique identifier of tokendata
    struct TokenDataId has copy, drop, store {
        // The creator of this token
        creator: address,
        // The collection or set of related tokens within the creator's account
        collection: String,
        // the name of this token
        name: String,
    }

    /// The shared TokenData by tokens with different serial_number
    struct TokenData has store {
        // id of this token data
        id: TokenDataId,
        // the maxium of tokens can be minted from this token
        maximum: u64,
        // the current largest serial number
        largest_serial_number: u64,
        // Total number of tokens minted for this TokenData
        supply: u64,
        // URL for additional information / media
        uri: String,
        // the royalty of the token
        royalty: Royalty,
        // The name of this Token
        name: String,
        // Describes this Token
        description: String,
        // store customized properties and their values for token with serial_number 0
        properties: PropertyMap,
        //control the TokenData field mutability
        mutability_config: TokenMutabilityConfig,
    }

    /// The royalty of a token
    struct Royalty has copy, drop, store {
        royalty_points_nominator: u64,
        royalty_points_denominator: u64,
        // if the token is jointly owned by multiple creators, the group of creators should create a shared account.
        // the payee_address will be the shared account address.
        payee_address: address,
    }

    /// This config specifies which fields in the TokenData are mutable
    struct TokenMutabilityConfig has copy, store, drop {
        // control if the token maximum is mutable
        maximum: bool,
        // control if the token uri is mutable
        uri: bool,
        // control if the token royalty is mutable
        royalty: bool,
        // control if the token description is mutable
        description: bool,
        // control if the property map is mutable
        properties: bool,
    }

    /// Represents token resources owned by token owner
    struct TokenStore has key {
        // the tokens owned by a token owner
        tokens: Table<TokenId, Token>,
        // used for storing token PropertyMap that has a serial number bigger than 0
        token_properties: Table<TokenId, PropertyMap>,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
    }

    /// This config specifies which fields in the Collection are mutable
    struct CollectionMutabilityConfig has copy, store, drop {
        // control if description is mutable
        description: bool,
        // control if uri is mutable
        uri: bool,
        // control if collection maxium is mutable
        maximum: bool,
    }

    /// Represent collection and token metadata for a creator
    struct Collections has key {
        collections: Table<String, Collection>,
        token_data: Table<TokenDataId, TokenData>,
        mint_capabilities: Table<TokenId, MintCapability>,
        create_collection_events: EventHandle<CreateCollectionEvent>,
        create_token_events: EventHandle<CreateTokenEvent>,
        mint_token_events: EventHandle<MintTokenEvent>,
    }

    /// Represent the collection metadata
    struct Collection has store {
        // Describes the collection
        description: String,
        // Unique name within this creators account for this collection
        name: String,
        // URL for additional information /media
        uri: String,
        // Total number of distinct TokenData tracked by the collection
        count: u64,
        // maximum number of TokenData allowed within this collections
        maximum: u64,
        // control which collection field is mutable
        mutability_config: CollectionMutabilityConfig,
    }

    /// Set of data sent to the event stream during a receive
    struct DepositEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    /// Set of data sent to the event stream during a withdrawal
    struct WithdrawEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    /// token creation event id of token created
    struct CreateTokenEvent has drop, store {
        id: TokenId,
        initial_balance: u64,
    }

    /// mint token event. This event triggered when creator adds more supply to existing token
    struct MintTokenEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    /// create collection event with creator address and collection name
    struct CreateCollectionEvent has drop, store {
        creator: address,
        collection_name: String,
        uri: String,
        description: String,
        maximum: u64,
    }

    /// Capability required to mint tokens.
    struct MintCapability has store {
        token_id: TokenId,
    }

    //
    // Creator Script functions
    //

    /// create a empty token collection with parameters
    public entry fun create_collection_script(
        creator: &signer,
        name: vector<u8>,
        description: vector<u8>,
        uri: vector<u8>,
        maximum: u64,
        mutate_setting: vector<bool>,
    ) acquires Collections {
        create_collection(
            creator,
            string::utf8(name),
            string::utf8(description),
            string::utf8(uri),
            maximum,
            mutate_setting
        );
    }

    /// create token with raw inputs
    public entry fun create_token_script(
        creator: &signer,
        collection: vector<u8>,
        name: vector<u8>,
        description: vector<u8>,
        balance: u64,
        maximum: u64,
        uri: vector<u8>,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_nominator: u64,
        token_mutate_setting: vector<bool>,
        property_keys: vector<vector<u8>>,
        property_values: vector<vector<u8>>,
        property_types: vector<vector<u8>>,
    ) acquires Collections, TokenStore, TokenAuthorityStore {
        create_token(
            creator,
            string::utf8(collection),
            string::utf8(name),
            string::utf8(description),
            balance,
            maximum,
            string::utf8(uri),
            royalty_payee_address,
            royalty_points_denominator,
            royalty_points_nominator,
            token_mutate_setting,
            property_map::generate_string_vector(property_keys),
            property_values,
            property_map::generate_string_vector(property_types),
        );
    }

    /// Mint more token from an existing token_data. Mint only adds more token to serial_number 0
    public entry fun mint(
        account: &signer,
        token_data_address: address,
        collection: vector<u8>,
        name: vector<u8>,
        amount: u64,
    ) acquires Collections, TokenStore {
        let token_data_id = create_token_data_id(
            token_data_address,
            string::utf8(collection),
            string::utf8(name),
        );
        // TODO: check based on mint_capability
        assert!(token_data_id.creator == signer::address_of(account), ENO_MINT_CAPABILITY);
        mint_token(
            account,
            token_data_id,
            amount,
        );
    }

    //
    // Transaction Script functions
    //

    public entry fun direct_transfer_script(
        sender: &signer,
        receiver: &signer,
        creators_address: address,
        collection: vector<u8>,
        name: vector<u8>,
        amount: u64,
        serial_number: u64
    ) acquires TokenStore {
        let token_id = create_token_id_raw(creators_address, collection, name, serial_number);
        direct_transfer(sender, receiver, token_id, amount);
    }

    public entry fun initialize_token_script(account: &signer) {
        initialize_token_store(account);
    }

    /// mutate the token property and save the new property in TokenStore
    /// if the token serial_number is 0, we will create a new serial number per token and store the properties
    /// if the token serial_number is not 0, we will just update the propertyMap
    public fun mutate_token_properties(
        account: &signer,
        token_owner: address,
        token_id: TokenId,
        amount: u64,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) acquires Collections, TokenStore {
        // TODO: mutate based on capability
        assert!(signer::address_of(account) == token_owner, ENO_MUTATE_CAPABILITY);
        // validate if the properties is mutable
        assert!(exists<Collections>(token_id.token_data_id.creator), ECOLLECTIONS_NOT_PUBLISHED);
        let all_token_data = &mut borrow_global_mut<Collections>(
            token_id.token_data_id.creator
        ).token_data;
        let token_data = table::borrow_mut(all_token_data, token_id.token_data_id);

        assert!(token_data.mutability_config.properties, EFIELD_NOT_MUTABLE);
        let addr = signer::address_of(account);
        // check if the serial_number is 0 to determine if we need to update the serial_number
        if (token_id.serial_number == 0) {
            let token = withdraw_with_event_internal(addr, token_id, amount);
            let i = 0;
            let largest_serial_number = token_data.largest_serial_number;
            // give a new serial number for each token
            while (i < token.value) {
                let cur_serial_number = largest_serial_number + i + 1;
                let new_token_id = create_token_id(token_id.token_data_id, cur_serial_number);
                let new_token = Token {
                    id: new_token_id,
                    value: 1,
                };
                // update the token largest serial number
                direct_deposit(token_owner, new_token);
                update_token_property_internal(token_owner, new_token_id, keys, values, types);
                i = i + 1;
            };
            token_data.largest_serial_number = largest_serial_number + token.value;

            // burn the orignial serial 0 token after mutation
            let Token {id: _, value: _} = token;

        } else {
            update_token_property_internal(token_owner, token_id, keys, values, types);
        };
    }

    fun update_token_property_internal(
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) acquires TokenStore {
        let token_properties = &mut borrow_global_mut<TokenStore>(token_owner).token_properties;
        if (table::contains(token_properties, token_id)){
            let value = table::borrow_mut(token_properties, token_id);
            property_map::update_property_map(value, keys, values, types);
        } else {
            let properties = property_map::new(keys, values, types);
            table::add(token_properties, token_id, properties);
        }
    }

    /// Deposit the token balance into the owner's account and emit an event.
    public fun deposit_token(account: &signer, token: Token) acquires TokenStore {
        let account_addr = signer::address_of(account);
        initialize_token_store(account);
        let tokens = &mut borrow_global_mut<TokenStore>(account_addr).tokens;
        if (!table::contains(tokens, token.id)) {
            initialize_token(account, token.id);
        };

        direct_deposit(account_addr, token)
    }

    /// Deposit the token balance into the recipients account and emit an event.
    public fun direct_deposit(account_addr: address, token: Token) acquires TokenStore {
        let token_store = borrow_global_mut<TokenStore>(account_addr);

        event::emit_event<DepositEvent>(
            &mut token_store.deposit_events,
            DepositEvent { id: token.id, amount: token.value },
        );

        direct_deposit_without_event(account_addr, token);
    }

    /// Deposit the token balance into the recipients account without emitting an event.
    public fun direct_deposit_without_event(account_addr: address, token: Token) acquires TokenStore {
        assert!(
            exists<TokenStore>(account_addr),
            error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
        );
        let token_store = borrow_global_mut<TokenStore>(account_addr);


        if (!table::contains(&token_store.tokens, token.id)) {
            table::add(&mut token_store.tokens, token.id, token);
        } else {
            let recipient_token = table::borrow_mut(&mut token_store.tokens, token.id);
            merge(recipient_token, token);
        };
    }

    public fun direct_transfer(
        sender: &signer,
        receiver: &signer,
        token_id: TokenId,
        amount: u64,
    ) acquires TokenStore {
        let token = withdraw_token(sender, token_id, amount);
        deposit_token(receiver, token);
        transfer_token_property(signer::address_of(sender), signer::address_of(receiver), token_id);
    }

    fun transfer_token_property(from: address, to: address, token_id: TokenId) acquires TokenStore {
        // only need to transfer token properties if serial_number is bigger than 0
        if (token_id.serial_number > 0) {
            let token_props = &mut borrow_global_mut<TokenStore>(from).token_properties;
            if (table::contains(token_props, token_id)) {
                let kvs = table::remove(token_props, token_id);
                let dst_token_props = &mut borrow_global_mut<TokenStore>(to).token_properties;
                assert!(!table::contains(dst_token_props, token_id), ETOKEN_SHOULDNOT_EXIST_IN_TOKEN_STORE);
                table::add(dst_token_props, token_id, kvs);
            }
        };

    }

    public fun initialize_token(account: &signer, token_id: TokenId) acquires TokenStore {
        let account_addr = signer::address_of(account);
        assert!(
            exists<TokenStore>(account_addr),
            error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
        );
        let tokens = &mut borrow_global_mut<TokenStore>(account_addr).tokens;

        assert!(
            !table::contains(tokens, token_id),
            error::already_exists(EALREADY_HAS_BALANCE),
        );
        table::add(tokens, token_id, Token { value: 0, id: token_id });
    }

    public fun initialize_token_store(account: &signer) {
        if (!exists<TokenStore>(signer::address_of(account))) {
            move_to(
                account,
                TokenStore {
                    tokens: table::new(),
                    token_properties: table::new(),
                    deposit_events: event::new_event_handle<DepositEvent>(account),
                    withdraw_events: event::new_event_handle<WithdrawEvent>(account),
                },
            );
        }
    }

    public fun merge(dst_token: &mut Token, source_token: Token) {
        assert!(&dst_token.id == &source_token.id, error::invalid_argument(EINVALID_TOKEN_MERGE));
        dst_token.value = dst_token.value + source_token.value;
        let Token { id: _, value: _ } = source_token;
    }

    public fun split(dst_token: &mut Token, amount: u64): Token {
        assert!(dst_token.value >= amount, ETOKEN_SPLIT_AMOUNT_LARGER_THEN_TOKEN_AMOUNT);
        dst_token.value = dst_token.value - amount;
        Token {
            id: dst_token.id,
            value: amount
        }
    }

    public fun token_id(token: &Token): &TokenId {
        &token.id
    }

    /// Transfers `amount` of tokens from `from` to `to`.
    public fun transfer(
        from: &signer,
        id: TokenId,
        to: address,
        amount: u64,
    ) acquires TokenStore {
        let token = withdraw_token(from, id, amount);
        direct_deposit(to, token);
        transfer_token_property(signer::address_of(from), to, id);
    }

    public fun withdraw_token(
        account: &signer,
        id: TokenId,
        amount: u64,
    ): Token acquires TokenStore {
        let account_addr = signer::address_of(account);
        withdraw_with_event_internal(account_addr, id, amount)
    }

    fun withdraw_with_event_internal(
        account_addr: address,
        id: TokenId,
        amount: u64,
    ): Token acquires TokenStore {
        let token_store = borrow_global_mut<TokenStore>(account_addr);
        event::emit_event<WithdrawEvent>(
            &mut token_store.withdraw_events,
            WithdrawEvent{ id, amount },
        );
        assert!(
            exists<TokenStore>(account_addr),
            error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
        );
        let tokens = &mut borrow_global_mut<TokenStore>(account_addr).tokens;
        assert!(
            table::contains(tokens, id),
            error::not_found(EBALANCE_NOT_PUBLISHED),
        );
        let balance = &mut table::borrow_mut(tokens, id).value;

        *balance = *balance - amount;
        Token{ id, value: amount }
    }

    //
    // Public functions for creating and maintaining tokens
    //

    /// Create a new collection to hold tokens
    public fun create_collection(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        maximum: u64,
        mutate_setting: vector<bool>
    ) acquires Collections {
        let account_addr = signer::address_of(creator);
        if (!exists<Collections>(account_addr)) {
            move_to(
                creator,
                Collections{
                    collections: table::new(),
                    token_data: table::new(),
                    mint_capabilities: table::new(),
                    create_collection_events: event::new_event_handle<CreateCollectionEvent>(creator),
                    create_token_events: event::new_event_handle<CreateTokenEvent>(creator),
                    mint_token_events: event::new_event_handle<MintTokenEvent>(creator),
                },
            )
        };

        let collections = &mut borrow_global_mut<Collections>(account_addr).collections;

        assert!(
            !table::contains(collections, name),
            error::already_exists(ECOLLECTION_ALREADY_EXISTS),
        );

        let mutability_config = create_collection_mutability_config(&mutate_setting);
        let collection = Collection{
            description,
            name: *&name,
            uri,
            count: 0,
            maximum,
            mutability_config
        };

        table::add(collections, name, collection);
        let collection_handle = borrow_global_mut<Collections>(account_addr);
        event::emit_event<CreateCollectionEvent>(
            &mut collection_handle.create_collection_events,
            CreateCollectionEvent {
                creator: account_addr,
                collection_name: *&name,
                uri,
                description,
                maximum,
            }
        );
    }

    public fun create_tokendata(
        account: &signer,
        collection: String,
        name: String,
        description: String,
        amount: u64,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_nominator: u64,
        token_mutate_config: TokenMutabilityConfig,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>
    ): TokenDataId acquires Collections {
        let account_addr = signer::address_of(account);
        assert!(
            exists<Collections>(account_addr),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );
        let collections = borrow_global_mut<Collections>(account_addr);

        let token_data_id = create_token_data_id(account_addr, collection, name);

        assert!(
            table::contains(&collections.collections, token_data_id.collection),
            error::already_exists(ECOLLECTION_NOT_PUBLISHED),
        );
        assert!(
            !table::contains(&collections.token_data, token_data_id),
            error::already_exists(ETOKEN_ALREADY_EXISTS),
        );

        let collection = table::borrow_mut(&mut collections.collections, token_data_id.collection);
        collection.count = collection.count + 1;
        assert!(
            collection.maximum >= collection.count,
            ECREATE_WOULD_EXCEED_MAXIMUM,
        );

        let supply = amount;
        assert!(
            maximum >= supply,
            ECREATE_WOULD_EXCEED_MAXIMUM,
        );

        let token_data = TokenData {
            id: token_data_id,
            maximum,
            largest_serial_number: 0,
            supply,
            uri,
            royalty: Royalty{
                royalty_points_denominator,
                royalty_points_nominator,
                payee_address: royalty_payee_address,
            },
            name,
            description,
            properties: property_map::new(property_keys, property_values, property_types),
            mutability_config: token_mutate_config,
        };

        table::add(&mut collections.token_data, token_data_id, token_data);
        token_data_id
    }

    public fun create_token_mutability_config(mutate_setting: &vector<bool>): TokenMutabilityConfig {
        TokenMutabilityConfig{
            maximum: *vector::borrow(mutate_setting, TOKEN_MAX_MUTABLE_IND),
            uri: *vector::borrow(mutate_setting, TOKEN_URI_MUTABLE_IND),
            royalty: *vector::borrow(mutate_setting, TOKEN_ROYALTY_MUTABLE_IND),
            description: *vector::borrow(mutate_setting, TOKEN_DESCRIPTION_MUTABLE_IND),
            properties: *vector::borrow(mutate_setting, TOKEN_PROPERTY_MUTABLE_IND),
        }
    }

    public fun create_collection_mutability_config(mutate_setting: &vector<bool>): CollectionMutabilityConfig {
        CollectionMutabilityConfig{
            description: *vector::borrow(mutate_setting, COLLECTION_DESCRIPTION_MUTABLE_IND),
            uri: *vector::borrow(mutate_setting, COLLECTION_URI_MUTABLE_IND),
            maximum: *vector::borrow(mutate_setting, COLLECTION_MAX_MUTABLE_IND),
        }
    }

    public fun create_token(
        account: &signer,
        collection: String,
        name: String,
        description: String,
        balance: u64,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_nominator: u64,
        mutate_setting: vector<bool>,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>
    ): TokenId acquires Collections, TokenStore, TokenAuthorityStore {
        let token_mut_config = create_token_mutability_config(&mutate_setting);

        let tokendata_id = create_tokendata(
            account,
            collection,
            name,
            description,
            0, // haven't minted token yet
            maximum,
            uri,
            royalty_payee_address,
            royalty_points_denominator,
            royalty_points_nominator,
            token_mut_config,
            property_keys,
            property_values,
            property_types
        );

        let token_id = mint_token(
            account,
            tokendata_id,
            balance,
        );

        // initialize capability associated with token
        initialize_token_authority_store(account);
        create_burn_authority(account, token_id);

        token_id
    }

    fun get_properties_to_be_updated(
        token_data: &TokenData,
        keys: &vector<String>,
        values: &vector<vector<u8>>,
        types: &vector<String>
    ): PropertyMap {
        let res_keys = vector::empty<String>();
        let res_vals = vector::empty<vector<u8>>();
        let res_types = vector::empty<String>();
        let default_properties = &token_data.properties;
        let i = 0;
        while (i < vector::length(keys)) {
            let k = vector::borrow(keys, i);
            let v = vector::borrow(values, i);
            let t = vector::borrow(types, i);
            if (property_map::contains_key(default_properties, k) ) {
                if ( property_map::borrow_type(property_map::borrow(default_properties, k)) == *t &&
                     property_map::borrow_value(property_map::borrow(default_properties, k)) != *v ) {
                    vector::push_back(&mut res_keys, *k);
                    vector::push_back(&mut res_vals, *v);
                    vector::push_back(&mut res_types, *t);
                };
            };
            i = i + 1;
        };
        property_map::new(res_keys, res_vals, res_types)
    }

    public fun mint_token(
        account: &signer,
        token_data_id: TokenDataId,
        amount: u64,
    ): TokenId acquires Collections, TokenStore {
        assert!(token_data_id.creator == signer::address_of(account), ENO_MINT_CAPABILITY);
        let creator_addr = token_data_id.creator;
        let all_token_data = &mut borrow_global_mut<Collections>(creator_addr).token_data;
        let token_data = table::borrow_mut(all_token_data, token_data_id);

        assert!(token_data.supply + amount <= token_data.maximum, 1);

        token_data.supply = token_data.supply + amount;

        // we add more tokens with serial_number 0
        let token_id = create_token_id(token_data_id, 0);
        deposit_token(account,
            Token{
                id: token_id,
                value: amount
            }
        );
        token_id
    }

    public fun create_token_id(token_data_id: TokenDataId, serial_number: u64): TokenId {
        TokenId{
            token_data_id,
            serial_number,
        }
    }

    public fun create_token_data_id(
        creator: address,
        collection: String,
        name: String,
    ): TokenDataId {
        TokenDataId { creator, collection, name }
    }

    public fun create_token_id_raw(
        creator: address,
        collection: vector<u8>,
        name: vector<u8>,
        serial_number: u64,
    ): TokenId {
        TokenId{
            token_data_id: create_token_data_id(creator, string::utf8(collection), string::utf8(name)),
            serial_number,
        }
    }

    public entry fun burn_script(
        account: &signer,
        token_id: TokenId,
        amount: u64,
    ) acquires TokenStore, Collections, TokenAuthorityStore {
        let addr = signer::address_of(account);
        assert!(balance_of(addr, token_id) >= amount, EINSUFFICIENT_BALANCE);
        let token = withdraw_token(account, token_id, amount);
        let burn_cap = acquire_burn_capability(account, token_id);
        burn(burn_cap, token)
    }

    public fun burn(burn_cap: BurnCapability, token: Token) acquires Collections {
        assert!(burn_cap.token_id == token.id, ENO_BURN_CAPABILITY);
        let creator_addr = token.id.token_data_id.creator;
        assert!(
            exists<Collections>(creator_addr),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let collections = borrow_global_mut<Collections>(creator_addr);
        assert!(
            table::contains(&collections.token_data, token.id.token_data_id),
            error::not_found(ETOKEN_NOT_PUBLISHED),
        );

        let token_data = table::borrow_mut(
            &mut collections.token_data,
            token.id.token_data_id,
        );
        token_data.supply = token_data.supply - token.value;
        let Token { id: _, value: _ } = token;
    }

    public fun balance_of(owner: address, id: TokenId): u64 acquires TokenStore {
        let token_store = borrow_global<TokenStore>(owner);
        if (table::contains(&token_store.tokens, id)) {
            table::borrow(&token_store.tokens, id).value
        } else {
            0
        }
    }

    public fun get_royalty(token_id: TokenId): Royalty acquires Collections {
        let token_data_id = token_id.token_data_id;
        let creator_addr = token_data_id.creator;
        assert!(exists<Collections>(creator_addr), ECOLLECTIONS_NOT_PUBLISHED);
        let all_token_data = &borrow_global<Collections>(creator_addr).token_data;
        assert!(table::contains(all_token_data, token_data_id), ETOKEN_NOT_PUBLISHED);

        let token_data = table::borrow(all_token_data, token_data_id);
        token_data.royalty
    }

    public fun get_royalty_nominator(royalty: &Royalty): u64 {
        royalty.royalty_points_nominator
    }

    public fun get_royalty_denominator(royalty: &Royalty): u64 {
        royalty.royalty_points_denominator
    }

    public fun get_royalty_payee(royalty: &Royalty): address {
        royalty.payee_address
    }

    /********************Token Capability*****************************/
    struct TokenAuthority has store {
        burn: bool
    }

    struct BurnCapability has drop {
        token_id: TokenId,
    }

    struct TokenAuthorityStore has key {
        token_auths: Table<TokenId, TokenAuthority>,
    }

    /// initialize capability store for storing all token capabilities
    /// this function should be called by any account that plan to own tokens
    public entry fun initialize_token_authority_store_script(creator: &signer) {
        initialize_token_authority_store(creator);
    }

    public fun acquire_burn_capability(
        account: &signer,
        token_id: TokenId
    ): BurnCapability acquires TokenAuthorityStore {
        assert!(exist_burn_authority(signer::address_of(account), token_id), ENO_BURN_CAPABILITY);
        BurnCapability {
            token_id
        }
    }

    public fun initialize_token_authority_store(creator: &signer) {
        let token_auths = table::new<TokenId, TokenAuthority>();
        move_to(creator, TokenAuthorityStore {
            token_auths
        })
    }

    /// create burn authority for a token id.
    /// 1. only token creator can create this authority,
    /// 2. token creator can create only 1 burn authority per token
    public fun create_burn_authority(creator: &signer, token_id: TokenId) acquires TokenAuthorityStore{
        let addr = signer::address_of(creator);
        assert!(token_id.token_data_id.creator == addr, EONLY_CREATOR_CAN_CREATE_BURN_CAP);
        if (!exists<TokenAuthorityStore>(addr)) {
            initialize_token_authority_store(creator);
        };
        let token_auths = &mut borrow_global_mut<TokenAuthorityStore>(addr).token_auths;
        assert!(!table::contains(token_auths, token_id), EBURNCAP_EXISTS_OR_CREATED_FOR_TOKEN);

        table::add(token_auths, token_id, TokenAuthority {burn: true});
    }

    /// delegate burn capability
    /// only existing capability holder is allowed to delegate to the owner of the token
    public fun delegate_burn_authority(
        cap_owner: &signer,
        to: address,
        token_id: TokenId,
    ) acquires TokenAuthorityStore, TokenStore {
        let addr = signer::address_of(cap_owner);
        assert!(exist_burn_authority(addr, token_id), ENOT_OWN_THE_CAPABILITY);
        let token_auths = &mut borrow_global_mut<TokenAuthorityStore>(addr).token_auths;
        let token_cap = table::borrow_mut(token_auths, token_id);
        token_cap.burn = false; // capability is disable

        assert!(exists<TokenAuthorityStore>(to), ETOKEN_CAPABILITY_STORE_NOT_EXISTS);
        assert!(balance_of(to, token_id)>0, EONLY_TOKEN_OWNER_CAN_HAVE_BURN_CAP);

        let owner_auth_store = &mut borrow_global_mut<TokenAuthorityStore>(to).token_auths;
        if (table::contains(owner_auth_store, token_id)) {
            let cap = table::borrow_mut(owner_auth_store, token_id);
            cap.burn = true;
        } else {
            table::add(owner_auth_store, token_id, TokenAuthority {burn: true});
        };
    }

    /// validate if an account has the burn capability for a token_id
    public fun exist_burn_authority(account: address, token_id: TokenId): bool acquires TokenAuthorityStore{
        assert!(exists<TokenAuthorityStore>(account), ETOKEN_CAPABILITY_STORE_NOT_EXISTS);
        let auth_store = &borrow_global<TokenAuthorityStore>(account).token_auths;
        if (!table::contains(auth_store, token_id)) {
            false
        } else {
            let token_cap = table::borrow(auth_store, token_id);
            token_cap.burn
        }
    }

    // ****************** TEST-ONLY FUNCTIONS **************

    #[test(creator = @0x1, owner = @0x2)]
    public fun create_withdraw_deposit_token(
        creator: signer,
        owner: signer
    ) acquires Collections, TokenStore, TokenAuthorityStore {
        let token_id = create_collection_and_token(&creator, 1, 1, 1);

        let token = withdraw_token(&creator, token_id, 1);
        deposit_token(&owner, token);
    }

    #[test(creator = @0xCC, owner = @0xCB)]
    public fun create_withdraw_deposit(
        creator: signer,
        owner: signer
    ) acquires Collections, TokenStore, TokenAuthorityStore {
        let token_id = create_collection_and_token(&creator, 2, 5, 5);

        let token_0 = withdraw_token(&creator, token_id, 1);
        let token_1 = withdraw_token(&creator, token_id, 1);
        deposit_token(&owner, token_0);
        deposit_token(&creator, token_1);
        let token_2 = withdraw_token(&creator, token_id, 1);
        deposit_token(&owner, token_2);
    }

    #[test(creator = @0x1)]
    #[expected_failure] // (abort_code = 5)]
    public fun test_collection_maximum(creator: signer) acquires Collections, TokenStore, TokenAuthorityStore {
        let token_id = create_collection_and_token(&creator, 2, 2, 1);
        let default_keys = vector<String>[ string::utf8(b"attack"), string::utf8(b"num_of_use") ];
        let default_vals = vector<vector<u8>>[ b"10", b"5" ];
        let default_types = vector<String>[ string::utf8(b"integer"), string::utf8(b"integer") ];
        let mutate_setting = vector<bool>[ false, false, false, false, false, false ];

        create_token(
            &creator,
            token_id.token_data_id.collection,
            string::utf8(b"Token"),
            string::utf8(b"Hello, Token"),
            100,
            2,
            string::utf8(b"https://aptos.dev"),
            signer::address_of(&creator),
            100,
            0,
            mutate_setting,
            default_keys,
            default_vals,
            default_types,
        );
    }

    #[test(creator = @0x1, owner = @0x2)]
    public entry fun direct_transfer_test(
        creator: signer,
        owner: signer,
    ) acquires Collections, TokenStore, TokenAuthorityStore {
        let token_id = create_collection_and_token(&creator, 2, 2, 2);
        direct_transfer(&creator, &owner, token_id, 1);
        let token = withdraw_token(&owner, token_id, 1);
        deposit_token(&creator, token);
    }

    #[test_only]
    public fun get_collection_name(): String {
        string::utf8(b"Hello, World")
    }

    #[test_only]
    public fun get_token_name(): String {
        string::utf8(b"Token")
    }

    #[test_only]
    public fun create_collection_and_token(
        creator: &signer,
        amount: u64,
        collection_max: u64,
        token_max: u64
    ): TokenId acquires Collections, TokenStore, TokenAuthorityStore {
        let mutate_setting = vector<bool>[false, false, false];

        create_collection(
            creator,
            get_collection_name(),
            string::utf8(b"Collection: Hello, World"),
            string::utf8(b"https://aptos.dev"),
            collection_max,
            mutate_setting
        );

        let default_keys = vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")];
        let default_vals = vector<vector<u8>>[b"10", b"5"];
        let default_types = vector<String>[string::utf8(b"integer"), string::utf8(b"integer")];
        let mutate_setting = vector<bool>[false, false, false, false, true];
        create_token(
            creator,
            get_collection_name(),
            get_token_name(),
            string::utf8(b"Hello, Token"),
            amount,
            token_max,
            string::utf8(b"https://aptos.dev"),
            signer::address_of(creator),
            100,
            0,
            mutate_setting,
            default_keys,
            default_vals,
            default_types,
        )
    }

    #[test(creator = @0xFF)]
    fun test_create_events_generation(creator: signer) acquires Collections, TokenStore, TokenAuthorityStore {
        create_collection_and_token(&creator, 1, 2, 1);
        let collections = borrow_global<Collections>(signer::address_of(&creator));
        assert!(event::get_event_handle_counter(&collections.create_collection_events) == 1, 1);
        // TODO assert!(event::get_event_handle_counter(&collections.create_token_events) == 1, 1);
    }

    #[test(creator = @0xAF)]
    fun test_create_token_from_tokendata(creator: &signer) acquires Collections, TokenStore, TokenAuthorityStore {
        create_collection_and_token(creator, 2, 4, 4);
        let token_data_id = create_token_data_id(
            signer::address_of(creator),
            get_collection_name(),
            get_token_name());

        let token_id = mint_token(
            creator,
            token_data_id,
            1,
        );

        assert!(balance_of(signer::address_of(creator), token_id) == 3, 1);
    }
    #[test(creator = @0xAF, owner = @0xBB)]
    fun test_mutate_token_property(creator: &signer, owner: &signer) acquires Collections, TokenStore, TokenAuthorityStore {
        // token owner mutate the token property
        let token_id = create_collection_and_token(creator, 2, 4, 4);
        assert!(token_id.serial_number == 0, 1);
        let new_keys = vector<String>[
            string::utf8(b"attack"), string::utf8(b"num_of_use")
        ];
        let new_vals = vector<vector<u8>>[
            b"1", b"1"
        ];
        let new_types = vector<String>[
            string::utf8(b"integer"), string::utf8(b"integer")
        ];

        mutate_token_properties(
            creator,
            signer::address_of(creator),
            token_id,
            2,
            new_keys,
            new_vals,
            new_types
        );
        // should have two new serial number from the orignal two tokens
        let new_id_1 = create_token_id(token_id.token_data_id, 1);
        let new_id_2 = create_token_id(token_id.token_data_id, 2);
        let new_id_3 = create_token_id(token_id.token_data_id, 3);

        assert!(balance_of(signer::address_of(creator), new_id_1) == 1, 1);
        assert!(balance_of(signer::address_of(creator), new_id_2) == 1, 1);
        assert!(balance_of(signer::address_of(creator), token_id) == 0, 1);

        // mutate token with serial_number > 0 should not generate new serial number
        mutate_token_properties(
            creator,
            signer::address_of(creator),
            new_id_1,
            1,
            new_keys,
            new_vals,
            new_types
        );
        assert!(balance_of(signer::address_of(creator), new_id_3) == 0, 1);
        // transfer token with serial_numer > 0 also transfer the token properties
        initialize_token_store(owner);
        transfer(creator, new_id_1, signer::address_of(owner), 1);

        let props = &borrow_global<TokenStore>(signer::address_of(owner)).token_properties;
        assert!(table::contains(props, new_id_1), 1);
    }

    #[test(creator = @0x1, owner = @0x2)]
    public entry fun test_burn_token(
        creator: signer,
        owner: signer
    ) acquires Collections, TokenStore, TokenAuthorityStore {
        // creator create a token. creator can burn the token
        let token_id = create_collection_and_token(&creator, 2, 1, 2);

        burn_script(&creator, token_id, 1);

        // creator transfer the remaining 1 token. owner canot burn the token
        let owner_addr =  signer::address_of(&owner);

        // init owner to receive token
        initialize_token_store(&owner);
        initialize_token(&owner, token_id);
        transfer(&creator, token_id, owner_addr, 1);
        initialize_token_authority_store(&owner);
        assert!(!exist_burn_authority(owner_addr, token_id), 1);

        // creator delegate burn capability and owner can burn
        delegate_burn_authority(&creator, owner_addr, token_id);
        burn_script(&owner, token_id, 1);
    }
}
