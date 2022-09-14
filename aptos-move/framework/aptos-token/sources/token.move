/// This module provides the foundation for Tokens.
module aptos_token::token {
    use std::error;
    use std::signer;
    use std::string::String;
    use std::vector;
    use std::option::{Self, Option};

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
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
    const ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM: u64 = 5;
    const EINSUFFICIENT_BALANCE: u64 = 6;
    const EINVALID_COLLECTION_NAME: u64 = 7;
    const EINVALID_TOKEN_MERGE: u64 = 8;
    const EMINT_WOULD_EXCEED_TOKEN_MAXIMUM: u64 = 9;
    const ENO_BURN_CAPABILITY: u64 = 10;
    const ENO_MINT_CAPABILITY: u64 = 11;
    const ETOKEN_ALREADY_EXISTS: u64 = 12;
    const ETOKEN_NOT_PUBLISHED: u64 = 13;
    const ETOKEN_STORE_NOT_PUBLISHED: u64 = 14;
    const ETOKEN_SPLIT_AMOUNT_LARGER_THEN_TOKEN_AMOUNT: u64 = 15;
    const EFIELD_NOT_MUTABLE: u64 = 16;
    const ENO_MUTATE_CAPABILITY: u64 = 17;
    const ETOEKN_PROPERTY_EXISTED: u64 = 18;
    const ENO_TOKEN_IN_TOKEN_STORE: u64 = 19;
    const ENON_ZERO_PROPERTY_VERSION_ONLY_ONE_INSTANCE: u64 = 20;
    const EUSER_NOT_OPT_IN_DIRECT_TRANSFER: u64 = 21;
    const EWITHDRAW_ZERO: u64 = 22;
    const ENOT_TRACKING_SUPPLY: u64 = 23;
    const ENFT_NOT_SPLITABLE: u64 = 24;

    //
    // Core data structures for holding tokens
    //

    struct Token has store {
        id: TokenId,
        // the amount of tokens. Only property_version = 0 can have a value bigger than 1.
        amount: u64,
        // The properties with this token.
        // when property_version = 0, the token_properties are the same as default_properties in TokenData, we don't store it.
        // when the property_map mutates, a new property_version is assigned to the token.
        token_properties: PropertyMap,
    }

    /// global unique identifier of a token
    struct TokenId has store, copy, drop {
        // the id to the common token data shared by token with different property_version
        token_data_id: TokenDataId,
        // the property_version of a token.
        // Token with dfiferent property_version can have different value of PropertyMap
        property_version: u64,
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

    /// The shared TokenData by tokens with different property_version
    struct TokenData has store {
        // the maxium of tokens can be minted from this token
        maximum: u64,
        // the current largest property_version
        largest_property_version: u64,
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
        // store customized properties and their values for token with property_version 0
        default_properties: PropertyMap,
        //control the TokenData field mutability
        mutability_config: TokenMutabilityConfig,
    }

    /// The royalty of a token
    struct Royalty has copy, drop, store {
        royalty_points_numerator: u64,
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
        direct_transfer: bool,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
        burn_events: EventHandle<BurnTokenEvent>,
        mutate_token_property_events: EventHandle<MutateTokenPropertyMapEvent>,
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
        collection_data: Table<String, CollectionData>,
        token_data: Table<TokenDataId, TokenData>,
        create_collection_events: EventHandle<CreateCollectionEvent>,
        create_token_data_events: EventHandle<CreateTokenDataEvent>,
        mint_token_events: EventHandle<MintTokenEvent>,
    }

    /// Represent the collection metadata
    struct CollectionData has store {
        // Describes the collection
        description: String,
        // Unique name within this creators account for this collection
        name: String,
        // URL for additional information /media
        uri: String,
        // Total number of distinct token_data tracked by the collection
        supply: u64,
        // maximum number of token_data allowed within this collections
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
    struct CreateTokenDataEvent has drop, store {
        id: TokenDataId,
        description: String,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        name: String,
        mutability_config: TokenMutabilityConfig,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>,
    }

    /// mint token event. This event triggered when creator adds more supply to existing token
    struct MintTokenEvent has drop, store {
        id: TokenDataId,
        amount: u64,
    }

    ///
    struct BurnTokenEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    ///
    struct MutateTokenPropertyMapEvent has drop, store {
        old_id: TokenId,
        new_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    }

    /// create collection event with creator address and collection name
    struct CreateCollectionEvent has drop, store {
        creator: address,
        collection_name: String,
        uri: String,
        description: String,
        maximum: u64,
    }

    //
    // Creator Entry functions
    //

    /// create a empty token collection with parameters
    public entry fun create_collection_script(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        maximum: u64,
        mutate_setting: vector<bool>,
    ) acquires Collections {
        create_collection(
            creator,
            name,
            description,
            uri,
            maximum,
            mutate_setting
        );
    }

    /// Mint more token from an existing token_data. Mint only adds more token to property_version 0
    public entry fun mint_script(
        account: &signer,
        token_data_address: address,
        collection: String,
        name: String,
        amount: u64,
    ) acquires Collections, TokenStore {
        let token_data_id = create_token_data_id(
            token_data_address,
            collection,
            name,
        );
        // only creator of the tokendata can mint more tokens for now
        assert!(token_data_id.creator == signer::address_of(account),  error::permission_denied(ENO_MINT_CAPABILITY));
        mint_token(
            account,
            token_data_id,
            amount,
        );
    }

    //
    // Transaction Entry functions
    //

    public entry fun direct_transfer_script(
        sender: &signer,
        receiver: &signer,
        creators_address: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ) acquires TokenStore {
        let token_id = create_token_id_raw(creators_address, collection, name, property_version);
        direct_transfer(sender, receiver, token_id, amount);
    }

    public entry fun initialize_token_script(account: &signer) {
        initialize_token_store(account);
    }

    public entry fun opt_in_direct_transfer(account: &signer, opt_in: bool) acquires TokenStore {
        let addr = signer::address_of(account);
        initialize_token_store(account);
        let opt_in_flag = &mut borrow_global_mut<TokenStore>(addr).direct_transfer;
        *opt_in_flag = opt_in;
    }

    public fun  mutate_one_token(
        account: &signer,
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ): TokenId acquires Collections, TokenStore {
        let creator = token_id.token_data_id.creator;
        assert!(signer::address_of(account) == creator, ENO_MUTATE_CAPABILITY);
        // validate if the properties is mutable
        assert!(exists<Collections>(creator), ECOLLECTIONS_NOT_PUBLISHED);
        let all_token_data = &mut borrow_global_mut<Collections>(
            creator
        ).token_data;

        assert!(table::contains(all_token_data, token_id.token_data_id), ETOKEN_NOT_PUBLISHED);
        let token_data = table::borrow_mut(all_token_data, token_id.token_data_id);

        assert!(token_data.mutability_config.properties, EFIELD_NOT_MUTABLE);
        // check if the property_version is 0 to determine if we need to update the property_version
        if (token_id.property_version == 0) {
            let token = withdraw_with_event_internal(token_owner, token_id, 1);
            // give a new property_version for each token
            let cur_property_version = token_data.largest_property_version + 1;
            let new_token_id = create_token_id(token_id.token_data_id, cur_property_version);
            let new_token = Token {
                id: new_token_id,
                amount: 1,
                token_properties: *&token_data.default_properties,
            };
            direct_deposit(token_owner, new_token);
            update_token_property_internal(token_owner, new_token_id, keys, values, types);
            event::emit_event<MutateTokenPropertyMapEvent>(
                &mut borrow_global_mut<TokenStore>(token_owner).mutate_token_property_events,
                MutateTokenPropertyMapEvent {
                    old_id: token_id,
                    new_id: new_token_id,
                    keys,
                    values,
                    types
                },
            );

            token_data.largest_property_version = cur_property_version;
            // burn the orignial property_version 0 token after mutation
            let Token {id: _, amount: _, token_properties: _} = token;
            new_token_id
        } else {
            // only 1 copy for the token with property verion bigger than 0
            update_token_property_internal(token_owner, token_id, keys, values, types);
            event::emit_event<MutateTokenPropertyMapEvent>(
                &mut borrow_global_mut<TokenStore>(token_owner).mutate_token_property_events,
                MutateTokenPropertyMapEvent {
                    old_id: token_id,
                    new_id: token_id,
                    keys,
                    values,
                    types
                },
            );
            token_id
        }
    }

    /// mutate the token property and save the new property in TokenStore
    /// if the token property_version is 0, we will create a new property_version per token to generate a new token_id per token
    /// if the token property_version is not 0, we will just update the propertyMap and use the existing token_id (property_version)
    public entry fun mutate_token_properties(
        account: &signer,
        token_owner: address,
        creator: address,
        collection_name: String,
        token_name: String,
        token_property_version: u64,
        amount: u64,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) acquires Collections, TokenStore {
        assert!(signer::address_of(account) == creator, error::not_found(ENO_MUTATE_CAPABILITY));
        // validate if the properties is mutable
        assert!(exists<Collections>(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &mut borrow_global_mut<Collections>(
            creator
        ).token_data;

        let token_id: TokenId = create_token_id_raw(creator, collection_name, token_name, token_property_version);
        assert!(table::contains(all_token_data, token_id.token_data_id), error::not_found(ETOKEN_NOT_PUBLISHED));
        let token_data = table::borrow_mut(all_token_data, token_id.token_data_id);
        assert!(token_data.mutability_config.properties, error::permission_denied(EFIELD_NOT_MUTABLE));
        // check if the property_version is 0 to determine if we need to update the property_version
        let i = 0;
        // give a new property_version for each token
        while (i < amount) {
            mutate_one_token(account, token_owner, token_id, keys, values, types);
            i = i + 1;
        };
    }

    fun update_token_property_internal(
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) acquires TokenStore {
        let tokens = &mut borrow_global_mut<TokenStore>(token_owner).tokens;
        assert!(table::contains(tokens, token_id), error::not_found(ENO_TOKEN_IN_TOKEN_STORE));
        let value = &mut table::borrow_mut(tokens, token_id).token_properties;

        property_map::update_property_map(value, keys, values, types);
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
    fun direct_deposit(account_addr: address, token: Token) acquires TokenStore {
        let token_store = borrow_global_mut<TokenStore>(account_addr);

        event::emit_event<DepositEvent>(
            &mut token_store.deposit_events,
            DepositEvent { id: token.id, amount: token.amount },
        );

        assert!(
            exists<TokenStore>(account_addr),
            error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
        );

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
        table::add(tokens, token_id, Token { amount: 0, id: token_id, token_properties: property_map::empty() });
    }

    public fun initialize_token_store(account: &signer) {
        if (!exists<TokenStore>(signer::address_of(account))) {
            move_to(
                account,
                TokenStore {
                    tokens: table::new(),
                    direct_transfer: false,
                    deposit_events: account::new_event_handle<DepositEvent>(account),
                    withdraw_events: account::new_event_handle<WithdrawEvent>(account),
                    burn_events: account::new_event_handle<BurnTokenEvent>(account),
                    mutate_token_property_events: account::new_event_handle<MutateTokenPropertyMapEvent>(account),
                },
            );
        }
    }

    public fun merge(dst_token: &mut Token, source_token: Token) {
        assert!(&dst_token.id == &source_token.id, error::invalid_argument(EINVALID_TOKEN_MERGE));
        //only property_version = 0 token require merge
        dst_token.amount = dst_token.amount + source_token.amount;
        let Token { id: _, amount: _, token_properties: _ } = source_token;
    }

    public fun split(dst_token: &mut Token, amount: u64): Token {
        assert!(dst_token.id.property_version == 0, error::invalid_state(ENFT_NOT_SPLITABLE));
        assert!(dst_token.amount > amount,  error::invalid_argument(ETOKEN_SPLIT_AMOUNT_LARGER_THEN_TOKEN_AMOUNT));
        dst_token.amount = dst_token.amount - amount;
        Token {
            id: dst_token.id,
            amount,
            token_properties: property_map::empty(),
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
        let opt_in_transfer = borrow_global<TokenStore>(to).direct_transfer;
        assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));
        let token = withdraw_token(from, id, amount);
        direct_deposit(to, token);
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
        // It does not make sense to withdraw 0 tokens.
        assert!(amount > 0, error::invalid_argument(EWITHDRAW_ZERO));
        // Make sure the account has sufficient tokens to withdraw.
        assert!(balance_of(account_addr, id) >= amount, error::invalid_argument(EINSUFFICIENT_BALANCE));

        assert!(
            exists<TokenStore>(account_addr),
            error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
        );

        let token_store = borrow_global_mut<TokenStore>(account_addr);
        event::emit_event<WithdrawEvent>(
            &mut token_store.withdraw_events,
            WithdrawEvent{ id, amount },
        );
        let tokens = &mut borrow_global_mut<TokenStore>(account_addr).tokens;
        assert!(
            table::contains(tokens, id),
            error::not_found(EBALANCE_NOT_PUBLISHED),
        );
        // balance > amount and amount > 0 indirectly asserted that balance > 0.
        let balance = &mut table::borrow_mut(tokens, id).amount;
        if (*balance > amount) {
            *balance = *balance - amount;
            Token{ id, amount, token_properties: property_map::empty() }
        } else {
            table::remove(tokens, id)
        }
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
                    collection_data: table::new(),
                    token_data: table::new(),
                    create_collection_events: account::new_event_handle<CreateCollectionEvent>(creator),
                    create_token_data_events: account::new_event_handle<CreateTokenDataEvent>(creator),
                    mint_token_events: account::new_event_handle<MintTokenEvent>(creator),
                },
            )
        };

        let collection_data = &mut borrow_global_mut<Collections>(account_addr).collection_data;

        assert!(
            !table::contains(collection_data, name),
            error::already_exists(ECOLLECTION_ALREADY_EXISTS),
        );

        let mutability_config = create_collection_mutability_config(&mutate_setting);
        let collection = CollectionData{
            description,
            name: *&name,
            uri,
            supply: 0,
            maximum,
            mutability_config
        };

        table::add(collection_data, name, collection);
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

    public fun check_collection_exists(creator: address, name: String): bool acquires Collections {
        assert!(
            exists<Collections>(creator),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let collection_data = &borrow_global<Collections>(creator).collection_data;
        table::contains(collection_data, name)
    }

    public fun create_tokendata(
        account: &signer,
        collection: String,
        name: String,
        description: String,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
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
            table::contains(&collections.collection_data, token_data_id.collection),
            error::already_exists(ECOLLECTION_NOT_PUBLISHED),
        );
        assert!(
            !table::contains(&collections.token_data, token_data_id),
            error::already_exists(ETOKEN_ALREADY_EXISTS),
        );

        let collection = table::borrow_mut(&mut collections.collection_data, token_data_id.collection);

        // if collection maximum == 0, user don't want to enforce supply constraint.
        // we don't track supply to make token creation parallelizable
        if (collection.maximum > 0) {
            collection.supply = collection.supply + 1;
            assert!(
                collection.maximum >= collection.supply,
                error::invalid_argument(ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM),
            );
        };

        let token_data = TokenData {
            maximum,
            largest_property_version: 0,
            supply: 0,
            uri,
            royalty: Royalty{
                royalty_points_denominator,
                royalty_points_numerator,
                payee_address: royalty_payee_address,
            },
            name,
            description,
            default_properties: property_map::new(property_keys, property_values, property_types),
            mutability_config: token_mutate_config,
        };

        table::add(&mut collections.token_data, token_data_id, token_data);

        event::emit_event<CreateTokenDataEvent>(
            &mut collections.create_token_data_events,
            CreateTokenDataEvent {
                id: token_data_id,
                description,
                maximum,
                uri,
                royalty_payee_address,
                royalty_points_denominator,
                royalty_points_numerator,
                name,
                mutability_config: token_mutate_config,
                property_keys,
                property_values,
                property_types,
            },
        );
        token_data_id
    }

    /// return the number of distinct token_data_id created under this collection
    public fun get_collection_supply(creator_address: address, collection_name: String): Option<u64> acquires Collections {
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let collections = &borrow_global<Collections>(creator_address).collection_data;
        assert!(table::contains(collections, collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));
        let collection_data = table::borrow(collections, collection_name);

        if (collection_data.maximum > 0) {
            option::some(collection_data.supply)
        } else {
            option::none()
        }
    }

    /// return the number of distinct token_id created under this collection
    public fun get_token_supply(creator_address: address, token_data_id: TokenDataId): Option<u64> acquires Collections {
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &borrow_global<Collections>(creator_address).token_data;
        assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_NOT_PUBLISHED));
        let token_data = table::borrow(all_token_data, token_data_id);

        if (token_data.maximum > 0 ) {
            option::some(token_data.supply)
        } else {
            option::none<u64>()
        }
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

    /// create token with raw inputs
    public entry fun create_token_script(
        account: &signer,
        collection: String,
        name: String,
        description: String,
        balance: u64,
        maximum: u64,
        uri: String,
        royalty_payee_address: address,
        royalty_points_denominator: u64,
        royalty_points_numerator: u64,
        mutate_setting: vector<bool>,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>
    ) acquires Collections, TokenStore {
        let token_mut_config = create_token_mutability_config(&mutate_setting);

        let tokendata_id = create_tokendata(
            account,
            collection,
            name,
            description,
            maximum,
            uri,
            royalty_payee_address,
            royalty_points_denominator,
            royalty_points_numerator,
            token_mut_config,
            property_keys,
            property_values,
            property_types
        );

        mint_token(
            account,
            tokendata_id,
            balance,
        );
    }

    public fun mint_token(
        account: &signer,
        token_data_id: TokenDataId,
        amount: u64,
    ): TokenId acquires Collections, TokenStore {
        assert!(token_data_id.creator == signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
        let creator_addr = token_data_id.creator;
        let all_token_data = &mut borrow_global_mut<Collections>(creator_addr).token_data;
        assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_NOT_PUBLISHED));
        let token_data = table::borrow_mut(all_token_data, token_data_id);

        if (token_data.maximum > 0 ) {
            assert!(token_data.supply + amount <= token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));
            token_data.supply = token_data.supply + amount;
        };

        // we add more tokens with property_version 0
        let token_id = create_token_id(token_data_id, 0);
        deposit_token(account,
            Token{
                id: token_id,
                amount,
                token_properties: property_map::empty(), // same as default properties no need to store
            }
        );
        event::emit_event<MintTokenEvent>(
            &mut borrow_global_mut<Collections>(creator_addr).mint_token_events,
            MintTokenEvent {
                id: token_data_id,
                amount,
            }
        );

        token_id
    }


    /// create tokens and directly deposite to receiver's address. The receiver should opt-in direct transfer
    public fun mint_token_to(
        account: &signer,
        receiver: address,
        token_data_id: TokenDataId,
        amount: u64,
    ) acquires Collections, TokenStore {
        assert!(exists<TokenStore>(receiver), error::not_found(ETOKEN_STORE_NOT_PUBLISHED));
        let opt_in_transfer = borrow_global<TokenStore>(receiver).direct_transfer;
        assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));

        assert!(token_data_id.creator == signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
        let creator_addr = token_data_id.creator;
        let all_token_data = &mut borrow_global_mut<Collections>(creator_addr).token_data;
        assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_NOT_PUBLISHED));
        let token_data = table::borrow_mut(all_token_data, token_data_id);

        if (token_data.maximum > 0 ) {
            assert!(token_data.supply + amount <= token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));
            token_data.supply = token_data.supply + amount;
        };

        // we add more tokens with property_version 0
        let token_id = create_token_id(token_data_id, 0);
        direct_deposit(receiver,
            Token{
                id: token_id,
                amount,
                token_properties: property_map::empty(), // same as default properties no need to store
            }
        );

        event::emit_event<MintTokenEvent>(
            &mut borrow_global_mut<Collections>(creator_addr).mint_token_events,
            MintTokenEvent {
                id: token_data_id,
                amount,
            }
        );
    }

    public entry fun burn(
        owner: &signer,
        creators_address: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64
    ) acquires Collections, TokenStore {
        let token_id = create_token_id_raw(creators_address, collection, name, property_version);
        let creator_addr = token_id.token_data_id.creator;
        assert!(
            exists<Collections>(creator_addr),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let collections = borrow_global_mut<Collections>(creator_addr);
        assert!(
            table::contains(&collections.token_data, token_id.token_data_id),
            error::not_found(ETOKEN_NOT_PUBLISHED),
        );

        // Burn the tokens.
        let Token { id: _, amount: burned_amount, token_properties: _ } = withdraw_token(owner, token_id, amount);
        let token_store = borrow_global_mut<TokenStore>(signer::address_of(owner));
        event::emit_event<BurnTokenEvent>(
            &mut token_store.burn_events,
            BurnTokenEvent { id: token_id, amount: burned_amount},
        );

        // Decrease the supply correspondingly by the amount of tokens burned.
        let token_data = table::borrow_mut(
            &mut collections.token_data,
            token_id.token_data_id,
        );
        token_data.supply = token_data.supply - burned_amount;

        // Delete the token_data if supply drops to 0.
        if (token_data.supply == 0) {
            let TokenData {
                maximum: _,
                largest_property_version: _,
                supply: _,
                uri: _,
                royalty: _,
                name: _,
                description: _,
                default_properties: _,
                mutability_config: _,
            } = table::remove(&mut collections.token_data, token_id.token_data_id);
        };
    }

    public fun create_token_id(token_data_id: TokenDataId, property_version: u64): TokenId {
        TokenId{
            token_data_id,
            property_version,
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
        collection: String,
        name: String,
        property_version: u64,
    ): TokenId {
        TokenId{
            token_data_id: create_token_data_id(creator, collection, name),
            property_version,
        }
    }

    public fun balance_of(owner: address, id: TokenId): u64 acquires TokenStore {
        let token_store = borrow_global<TokenStore>(owner);
        if (table::contains(&token_store.tokens, id)) {
            table::borrow(&token_store.tokens, id).amount
        } else {
            0
        }
    }

    public fun get_royalty(token_id: TokenId): Royalty acquires Collections {
        let token_data_id = token_id.token_data_id;
        let creator_addr = token_data_id.creator;
        assert!(exists<Collections>(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &borrow_global<Collections>(creator_addr).token_data;
        assert!(table::contains(all_token_data, token_data_id), error::not_found(ETOKEN_NOT_PUBLISHED));

        let token_data = table::borrow(all_token_data, token_data_id);
        token_data.royalty
    }

    public fun get_royalty_numerator(royalty: &Royalty): u64 {
        royalty.royalty_points_numerator
    }

    public fun get_royalty_denominator(royalty: &Royalty): u64 {
        royalty.royalty_points_denominator
    }

    public fun get_royalty_payee(royalty: &Royalty): address {
        royalty.payee_address
    }

    public fun get_token_amount(token: &Token): u64 {
        token.amount
    }

    /// return the creator address, collection name, token name and property_version
    public fun get_token_id_fields(token_id: &TokenId): (address, String, String, u64) {
        (
            token_id.token_data_id.creator,
            token_id.token_data_id.collection,
            token_id.token_data_id.name,
            token_id.property_version,
        )
    }

    public fun get_token_data_id_fields(token_data_id: &TokenDataId): (address, String, String) {
        (
            token_data_id.creator,
            token_data_id.collection,
            token_data_id.name,
        )
    }

    /// return a copy of the token property map.
    /// if property_version = 0, return the default property map
    /// if property_version > 0, return the property value stored at owner's token store
    public fun get_property_map(owner: address, token_id: TokenId): PropertyMap acquires Collections, TokenStore {
        assert!(balance_of(owner, token_id) > 0, error::not_found(EINSUFFICIENT_BALANCE));
        // if property_version = 0, return default property map
        if (token_id.property_version == 0) {
            let creator_addr = token_id.token_data_id.creator;
            let all_token_data = &borrow_global<Collections>(creator_addr).token_data;
            assert!(table::contains(all_token_data, token_id.token_data_id), error::not_found(ETOKEN_NOT_PUBLISHED));
            let token_data = table::borrow(all_token_data, token_id.token_data_id);
            *&token_data.default_properties
        } else {
            let tokens = &borrow_global<TokenStore>(owner).tokens;
            *&table::borrow(tokens, token_id).token_properties
        }
    }

    // ****************** TEST-ONLY FUNCTIONS **************

    #[test_only]
    use std::string;

    #[test(creator = @0x1, owner = @0x2)]
    public fun create_withdraw_deposit_token(
        creator: signer,
        owner: signer
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(&creator));
        account::create_account_for_test(signer::address_of(&owner));
        let token_id = create_collection_and_token(&creator, 1, 1, 1);

        let token = withdraw_token(&creator, token_id, 1);
        deposit_token(&owner, token);
    }

    #[test(creator = @0xCC, owner = @0xCB)]
    public fun create_withdraw_deposit(
        creator: signer,
        owner: signer
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(&creator));
        account::create_account_for_test(signer::address_of(&owner));
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
    public entry fun test_collection_maximum(creator: signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(&creator));
        let token_id = create_collection_and_token(&creator, 2, 2, 1);
        let default_keys = vector<String>[ string::utf8(b"attack"), string::utf8(b"num_of_use") ];
        let default_vals = vector<vector<u8>>[ bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5) ];
        let default_types = vector<String>[ string::utf8(b"u64"), string::utf8(b"u64") ];
        let mutate_setting = vector<bool>[ false, false, false, false, false, false ];

        create_token_script(
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

    #[test(creator = @0xFA, owner = @0xAF)]
    public entry fun direct_transfer_test(
        creator: signer,
        owner: signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(&creator));
        account::create_account_for_test(signer::address_of(&owner));
        let token_id = create_collection_and_token(&creator, 2, 2, 2);
        direct_transfer(&creator, &owner, token_id, 1);
        let token = withdraw_token(&owner, token_id, 1);
        deposit_token(&creator, token);
    }

    #[test_only]
    public fun get_collection_name(): String {
        use std::string;
        string::utf8(b"Hello, World")
    }

    #[test_only]
    public fun get_token_name(): String {
        use std::string;
        string::utf8(b"Token")
    }

    #[test_only]
    public entry fun create_collection_and_token(
        creator: &signer,
        amount: u64,
        collection_max: u64,
        token_max: u64
    ): TokenId acquires Collections, TokenStore {
        use std::string;
        use std::bcs;
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
        let default_vals = vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)];
        let default_types = vector<String>[string::utf8(b"u64"), string::utf8(b"u64")];
        let mutate_setting = vector<bool>[false, false, false, false, true];
        create_token_script(
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
        );
        create_token_id_raw(signer::address_of(creator), get_collection_name(), get_token_name(), 0)
    }

    #[test(creator = @0xFF)]
    fun test_create_events_generation(creator: signer) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(&creator));
        create_collection_and_token(&creator, 1, 2, 1);
        let collections = borrow_global<Collections>(signer::address_of(&creator));
        assert!(event::counter(&collections.create_collection_events) == 1, 1);
    }

    #[test(creator = @0xAF)]
    fun test_create_token_from_tokendata(creator: &signer) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));

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
    fun test_mutate_token_property(creator: &signer, owner: &signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));

        // token owner mutate the token property
        let token_id = create_collection_and_token(creator, 2, 4, 4);
        assert!(token_id.property_version == 0, 1);
        let new_keys = vector<String>[
            string::utf8(b"attack"), string::utf8(b"num_of_use")
        ];
        let new_vals = vector<vector<u8>>[
            bcs::to_bytes<u64>(&1), bcs::to_bytes<u64>(&1)
        ];
        let new_types = vector<String>[
            string::utf8(b"u64"), string::utf8(b"u64")
        ];

        mutate_token_properties(
            creator,
            token_id.token_data_id.creator,
            token_id.token_data_id.creator,
            token_id.token_data_id.collection,
            token_id.token_data_id.name,
            token_id.property_version,
            2,
            new_keys,
            new_vals,
            new_types,
        );
        // should have two new property_version from the orignal two tokens
        let new_id_1 = create_token_id(token_id.token_data_id, 1);
        let new_id_2 = create_token_id(token_id.token_data_id, 2);
        let new_id_3 = create_token_id(token_id.token_data_id, 3);

        assert!(balance_of(signer::address_of(creator), new_id_1) == 1, 1);
        assert!(balance_of(signer::address_of(creator), new_id_2) == 1, 1);
        assert!(balance_of(signer::address_of(creator), token_id) == 0, 1);

        let creator_props = &borrow_global<TokenStore>(signer::address_of(creator)).tokens;
        let token = table::borrow(creator_props, new_id_1);

        assert!(property_map::length(&token.token_properties) == 2, property_map::length(&token.token_properties));
        // mutate token with property_version > 0 should not generate new property_version
        mutate_token_properties(
            creator,
            signer::address_of(creator),
            new_id_1.token_data_id.creator,
            new_id_1.token_data_id.collection,
            new_id_1.token_data_id.name,
            new_id_1.property_version,
            1,
            new_keys,
            new_vals,
            new_types
        );
        assert!(balance_of(signer::address_of(creator), new_id_3) == 0, 1);
        // transfer token with property_version > 0 also transfer the token properties
        initialize_token_store(owner);
        opt_in_direct_transfer(owner, true);
        transfer(creator, new_id_1, signer::address_of(owner), 1);

        let props = &borrow_global<TokenStore>(signer::address_of(owner)).tokens;
        assert!(table::contains(props, new_id_1), 1);
        let token = table::borrow(props, new_id_1);
        assert!(property_map::length(&token.token_properties) == 2, property_map::length(&token.token_properties));
    }

    #[test(creator = @0xAF, owner = @0xBB)]
    #[expected_failure(abort_code = 393219)]
    fun test_mutate_token_property_fail(creator: &signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(creator, 2, 4, 4);
        assert!(token_id.property_version == 0, 1);
        // only be able to mutate the attributed defined when creating the token
        let new_keys = vector<String>[
            string::utf8(b"attack"), string::utf8(b"num_of_use"), string::utf8(b"wrong_attribute")
        ];
        let new_vals = vector<vector<u8>>[
            bcs::to_bytes<u64>(&1), bcs::to_bytes<u64>(&1), bcs::to_bytes<u64>(&1)
        ];
        let new_types = vector<String>[
            string::utf8(b"u64"), string::utf8(b"u64"), string::utf8(b"u64")
        ];

        mutate_token_properties(
            creator,
            token_id.token_data_id.creator,
            token_id.token_data_id.creator,
            token_id.token_data_id.collection,
            token_id.token_data_id.name,
            token_id.property_version,
            2,
            new_keys,
            new_vals,
            new_types,
        );
    }

    #[test(creator = @0xAF, owner = @0xBB)]
    fun test_get_property_map_should_not_update_source_value(creator: &signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(creator, 2, 4, 4);
        assert!(token_id.property_version == 0, 1);
        // only be able to mutate the attributed defined when creating the token
        let new_keys = vector<String>[
            string::utf8(b"attack"), string::utf8(b"num_of_use")
        ];
        let new_vals = vector<vector<u8>>[
            bcs::to_bytes<u64>(&1), bcs::to_bytes<u64>(&1)
        ];
        let new_types = vector<String>[
            string::utf8(b"u64"), string::utf8(b"u64")
        ];
        let pm = get_property_map(signer::address_of(creator), token_id);
        assert!(property_map::length(&pm) == 2, 1);
        let new_token_id = mutate_one_token(
            creator,
            signer::address_of(creator),
            token_id,
            new_keys,
            new_vals,
            new_types,
        );
        let updated_pm = get_property_map(signer::address_of(creator), new_token_id);
        assert!(property_map::length(&updated_pm) == 2, 1);
        property_map::update_property_value(
            &mut updated_pm,
            &string::utf8(b"attack"),
            property_map::create_property_value<u64>(&2),
        );

        assert!(property_map::read_u64(&updated_pm, &string::utf8(b"attack")) == 2, 1);
        let og_pm = get_property_map(signer::address_of(creator), new_token_id);
        assert!(property_map::read_u64(&og_pm, &string::utf8(b"attack")) == 1, 1);
    }
}
