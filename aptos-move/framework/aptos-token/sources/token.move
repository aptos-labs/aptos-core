/// This module provides the foundation for Tokens.
/// Checkout our developer doc on our token standard https://aptos.dev/standards
module aptos_token::token {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use std::vector;

    use aptos_framework::account;
    use aptos_framework::event::{Self, EventHandle};
    use aptos_framework::timestamp;
    use aptos_std::table::{Self, Table};
    use aptos_token::property_map::{Self, PropertyMap, PropertyValue};
    use aptos_token::token_event_store;

    //
    // Constants
    //

    const TOKEN_MAX_MUTABLE_IND: u64 = 0;
    const TOKEN_URI_MUTABLE_IND: u64 = 1;
    const TOKEN_ROYALTY_MUTABLE_IND: u64 = 2;
    const TOKEN_DESCRIPTION_MUTABLE_IND: u64 = 3;
    const TOKEN_PROPERTY_MUTABLE_IND: u64 = 4;
    const TOKEN_PROPERTY_VALUE_MUTABLE_IND: u64 = 5;

    const COLLECTION_DESCRIPTION_MUTABLE_IND: u64 = 0;
    const COLLECTION_URI_MUTABLE_IND: u64 = 1;
    const COLLECTION_MAX_MUTABLE_IND: u64 = 2;

    const MAX_COLLECTION_NAME_LENGTH: u64 = 128;
    const MAX_NFT_NAME_LENGTH: u64 = 128;
    const MAX_URI_LENGTH: u64 = 512;

    // Property key stored in default_properties controlling who can burn the token.
    // the corresponding property value is BCS serialized bool.
    const BURNABLE_BY_CREATOR: vector<u8> = b"TOKEN_BURNABLE_BY_CREATOR";
    const BURNABLE_BY_OWNER: vector<u8> = b"TOKEN_BURNABLE_BY_OWNER";
    const TOKEN_PROPERTY_MUTABLE: vector<u8> = b"TOKEN_PROPERTY_MUTATBLE";

    //
    // Errors
    //
    /// The token has balance and cannot be initialized
    const EALREADY_HAS_BALANCE: u64 = 0;

    /// There isn't any collection under this account
    const ECOLLECTIONS_NOT_PUBLISHED: u64 = 1;

    /// Cannot find collection in creator's account
    const ECOLLECTION_NOT_PUBLISHED: u64 = 2;

    /// The collection already exists
    const ECOLLECTION_ALREADY_EXISTS: u64 = 3;

    /// Exceeds the collection's maximal number of token_data
    const ECREATE_WOULD_EXCEED_COLLECTION_MAXIMUM: u64 = 4;

    /// Insufficient token balance
    const EINSUFFICIENT_BALANCE: u64 = 5;

    /// Cannot merge the two tokens with different token id
    const EINVALID_TOKEN_MERGE: u64 = 6;

    /// Exceed the token data maximal allowed
    const EMINT_WOULD_EXCEED_TOKEN_MAXIMUM: u64 = 7;

    /// No burn capability
    const ENO_BURN_CAPABILITY: u64 = 8;

    /// TokenData already exists
    const ETOKEN_DATA_ALREADY_EXISTS: u64 = 9;

    /// TokenData not published
    const ETOKEN_DATA_NOT_PUBLISHED: u64 = 10;

    /// TokenStore doesn't exist
    const ETOKEN_STORE_NOT_PUBLISHED: u64 = 11;

    /// Cannot split token to an amount larger than its amount
    const ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT: u64 = 12;

    /// The field is not mutable
    const EFIELD_NOT_MUTABLE: u64 = 13;

    /// Not authorized to mutate
    const ENO_MUTATE_CAPABILITY: u64 = 14;

    /// Token not in the token store
    const ENO_TOKEN_IN_TOKEN_STORE: u64 = 15;

    /// User didn't opt-in direct transfer
    const EUSER_NOT_OPT_IN_DIRECT_TRANSFER: u64 = 16;

    /// Cannot withdraw 0 token
    const EWITHDRAW_ZERO: u64 = 17;

    /// Cannot split a token that only has 1 amount
    const ENFT_NOT_SPLITABLE: u64 = 18;

    /// No mint capability
    const ENO_MINT_CAPABILITY: u64 = 19;

    /// The collection name is too long
    const ECOLLECTION_NAME_TOO_LONG: u64 = 25;

    /// The NFT name is too long
    const ENFT_NAME_TOO_LONG: u64 = 26;

    /// The URI is too long
    const EURI_TOO_LONG: u64 = 27;

    /// Cannot deposit a Token with 0 amount
    const ENO_DEPOSIT_TOKEN_WITH_ZERO_AMOUNT: u64 = 28;

    /// Cannot burn 0 Token
    const ENO_BURN_TOKEN_WITH_ZERO_AMOUNT: u64 = 29;

    /// Token is not burnable by owner
    const EOWNER_CANNOT_BURN_TOKEN: u64 = 30;

    /// Token is not burnable by creator
    const ECREATOR_CANNOT_BURN_TOKEN: u64 = 31;

    /// Reserved fields for token contract
    /// Cannot be updated by user
    const ECANNOT_UPDATE_RESERVED_PROPERTY: u64 = 32;

    /// TOKEN with 0 amount is not allowed
    const ETOKEN_CANNOT_HAVE_ZERO_AMOUNT: u64 = 33;

    /// Royalty invalid if the numerator is larger than the denominator
    const EINVALID_ROYALTY_NUMERATOR_DENOMINATOR: u64 = 34;

    /// Royalty payee account does not exist
    const EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST: u64 = 35;

    /// Collection or tokendata maximum must be larger than supply
    const EINVALID_MAXIMUM: u64 = 36;

    /// Token Properties count doesn't match
    const ETOKEN_PROPERTIES_COUNT_NOT_MATCH: u64 = 37;


    /// Withdraw capability doesn't have sufficient amount
    const EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT: u64 = 38;

    /// Withdraw proof expires
    const EWITHDRAW_PROOF_EXPIRES: u64 = 39;

    /// The property is reserved by token standard
    const EPROPERTY_RESERVED_BY_STANDARD: u64 = 40;

    //
    // Core data structures for holding tokens
    //
    struct Token has store {
        id: TokenId,
        /// the amount of tokens. Only property_version = 0 can have a value bigger than 1.
        amount: u64,
        /// The properties with this token.
        /// when property_version = 0, the token_properties are the same as default_properties in TokenData, we don't store it.
        /// when the property_map mutates, a new property_version is assigned to the token.
        token_properties: PropertyMap,
    }

    /// global unique identifier of a token
    struct TokenId has store, copy, drop {
        /// the id to the common token data shared by token with different property_version
        token_data_id: TokenDataId,
        /// The version of the property map; when a fungible token is mutated, a new property version is created and assigned to the token to make it an NFT
        property_version: u64,
    }

    /// globally unique identifier of tokendata
    struct TokenDataId has copy, drop, store {
        /// The address of the creator, eg: 0xcafe
        creator: address,
        /// The name of collection; this is unique under the same account, eg: "Aptos Animal Collection"
        collection: String,
        /// The name of the token; this is the same as the name field of TokenData
        name: String,
    }

    /// The shared TokenData by tokens with different property_version
    struct TokenData has store {
        /// The maximal number of tokens that can be minted under this TokenData; if the maximum is 0, there is no limit
        maximum: u64,
        /// The current largest property version of all tokens with this TokenData
        largest_property_version: u64,
        /// The number of tokens with this TokenData. Supply is only tracked for the limited token whose maximum is not 0
        supply: u64,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain storage; the URL length should be less than 512 characters, eg: https://arweave.net/Fmmn4ul-7Mv6vzm7JwE69O-I-vd6Bz2QriJO1niwCh4
        uri: String,
        /// The denominator and numerator for calculating the royalty fee; it also contains payee account address for depositing the Royalty
        royalty: Royalty,
        /// The name of the token, which should be unique within the collection; the length of name should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
        /// Describes this Token
        description: String,
        /// The properties are stored in the TokenData that are shared by all tokens
        default_properties: PropertyMap,
        /// Control the TokenData field mutability
        mutability_config: TokenMutabilityConfig,
    }

    /// The royalty of a token
    struct Royalty has copy, drop, store {
        royalty_points_numerator: u64,
        royalty_points_denominator: u64,
        /// if the token is jointly owned by multiple creators, the group of creators should create a shared account.
        /// the payee_address will be the shared account address.
        payee_address: address,
    }

    /// This config specifies which fields in the TokenData are mutable
    struct TokenMutabilityConfig has copy, store, drop {
        /// control if the token maximum is mutable
        maximum: bool,
        /// control if the token uri is mutable
        uri: bool,
        /// control if the token royalty is mutable
        royalty: bool,
        /// control if the token description is mutable
        description: bool,
        /// control if the property map is mutable
        properties: bool,
    }

    /// Represents token resources owned by token owner
    struct TokenStore has key {
        /// the tokens owned by a token owner
        tokens: Table<TokenId, Token>,
        direct_transfer: bool,
        deposit_events: EventHandle<DepositEvent>,
        withdraw_events: EventHandle<WithdrawEvent>,
        burn_events: EventHandle<BurnTokenEvent>,
        mutate_token_property_events: EventHandle<MutateTokenPropertyMapEvent>,
    }

    /// This config specifies which fields in the Collection are mutable
    struct CollectionMutabilityConfig has copy, store, drop {
        /// control if description is mutable
        description: bool,
        /// control if uri is mutable
        uri: bool,
        /// control if collection maxium is mutable
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
        /// A description for the token collection Eg: "Aptos Toad Overload"
        description: String,
        /// The collection name, which should be unique among all collections by the creator; the name should also be smaller than 128 characters, eg: "Animal Collection"
        name: String,
        /// The URI for the collection; its length should be smaller than 512 characters
        uri: String,
        /// The number of different TokenData entries in this collection
        supply: u64,
        /// If maximal is a non-zero value, the number of created TokenData entries should be smaller or equal to this maximum
        /// If maximal is 0, Aptos doesn't track the supply of this collection, and there is no limit
        maximum: u64,
        /// control which collectionData field is mutable
        mutability_config: CollectionMutabilityConfig,
    }

    /// capability to withdraw without signer, this struct should be non-copyable
    struct WithdrawCapability has drop, store {
        token_owner: address,
        token_id: TokenId,
        amount: u64,
        expiration_sec: u64,
    }

    /// Set of data sent to the event stream during a receive
    struct DepositEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    #[event]
    /// Set of data sent to the event stream during a receive
    struct TokenDeposit has drop, store {
        account: address,
        id: TokenId,
        amount: u64,
    }

    #[deprecated]
    #[event]
    /// Set of data sent to the event stream during a receive
    struct Deposit has drop, store {
        id: TokenId,
        amount: u64,
    }

    /// Set of data sent to the event stream during a withdrawal
    struct WithdrawEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    #[deprecated]
    #[event]
    /// Set of data sent to the event stream during a withdrawal
    struct Withdraw has drop, store {
        id: TokenId,
        amount: u64,
    }

    #[event]
    /// Set of data sent to the event stream during a withdrawal
    struct TokenWithdraw has drop, store {
        account: address,
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

    #[deprecated]
    #[event]
    struct CreateTokenData has drop, store {
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

    #[event]
    struct TokenDataCreation has drop, store {
        creator: address,
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

    #[deprecated]
    #[event]
    struct MintToken has drop, store {
        id: TokenDataId,
        amount: u64,
    }

    #[event]
    struct Mint has drop, store {
        creator: address,
        id: TokenDataId,
        amount: u64,
    }

    ///
    struct BurnTokenEvent has drop, store {
        id: TokenId,
        amount: u64,
    }

    #[deprecated]
    #[event]
    struct BurnToken has drop, store {
        id: TokenId,
        amount: u64,
    }

    #[event]
    struct Burn has drop, store {
        account: address,
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

    #[deprecated]
    #[event]
    struct MutateTokenPropertyMap has drop, store {
        old_id: TokenId,
        new_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    }

    #[event]
    struct MutatePropertyMap has drop, store {
        account: address,
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

    #[event]
    struct CreateCollection has drop, store {
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
        assert!(token_data_id.creator == signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
        mint_token(
            account,
            token_data_id,
            amount,
        );
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
        let token_id = create_token_id_raw(
            creator,
            collection_name,
            token_name,
            token_property_version,
        );
        // give a new property_version for each token
        for (i in 0..amount) {
            mutate_one_token(account, token_owner, token_id, keys, values, types);
        };
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

    public entry fun opt_in_direct_transfer(account: &signer, opt_in: bool) acquires TokenStore {
        let addr = signer::address_of(account);
        initialize_token_store(account);
        let opt_in_flag = &mut TokenStore[addr].direct_transfer;
        *opt_in_flag = opt_in;
        token_event_store::emit_token_opt_in_event(account, opt_in);
    }

    /// Transfers `amount` of tokens from `from` to `to`.
    /// The receiver `to` has to opt-in direct transfer first
    public entry fun transfer_with_opt_in(
        from: &signer,
        creator: address,
        collection_name: String,
        token_name: String,
        token_property_version: u64,
        to: address,
        amount: u64,
    ) acquires TokenStore {
        let token_id = create_token_id_raw(creator, collection_name, token_name, token_property_version);
        transfer(from, token_id, to, amount);
    }

    /// Burn a token by creator when the token's BURNABLE_BY_CREATOR is true
    /// The token is owned at address owner
    public entry fun burn_by_creator(
        creator: &signer,
        owner: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64,
    ) acquires Collections, TokenStore {
        let creator_address = signer::address_of(creator);
        assert!(amount > 0, error::invalid_argument(ENO_BURN_TOKEN_WITH_ZERO_AMOUNT));
        let token_id = create_token_id_raw(creator_address, collection, name, property_version);
        let creator_addr = token_id.token_data_id.creator;
        assert!(
            exists<Collections>(creator_addr),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let collections = &mut Collections[creator_address];
        assert!(
            collections.token_data.contains(token_id.token_data_id),
            error::not_found(ETOKEN_DATA_NOT_PUBLISHED),
        );

        let token_data = collections.token_data.borrow_mut(token_id.token_data_id);

        // The property should be explicitly set in the property_map for creator to burn the token
        assert!(
            token_data.default_properties.contains_key(&string::utf8(BURNABLE_BY_CREATOR)),
            error::permission_denied(ECREATOR_CANNOT_BURN_TOKEN)
        );

        let burn_by_creator_flag = token_data.default_properties.read_bool(&string::utf8(BURNABLE_BY_CREATOR));
        assert!(burn_by_creator_flag, error::permission_denied(ECREATOR_CANNOT_BURN_TOKEN));

        // Burn the tokens.
        let Token { id: _, amount: burned_amount, token_properties: _ } = withdraw_with_event_internal(owner, token_id, amount);
        let token_store = &mut TokenStore[owner];
        if (std::features::module_event_migration_enabled()) {
            event::emit(Burn { account: owner, id: token_id, amount: burned_amount });
        } else {
            event::emit_event<BurnTokenEvent>(
                &mut token_store.burn_events,
                BurnTokenEvent { id: token_id, amount: burned_amount }
            );
        };

        if (token_data.maximum > 0) {
            token_data.supply -= burned_amount;

            // Delete the token_data if supply drops to 0.
            if (token_data.supply == 0) {
                destroy_token_data(collections.token_data.remove(token_id.token_data_id));

                // update the collection supply
                let collection_data = collections.collection_data.borrow_mut(token_id.token_data_id.collection);
                if (collection_data.maximum > 0) {
                    collection_data.supply -= 1;
                    // delete the collection data if the collection supply equals 0
                    if (collection_data.supply == 0) {
                        destroy_collection_data(collections.collection_data.remove(collection_data.name));
                    };
                };
            };
        };
    }

    /// Burn a token by the token owner
    public entry fun burn(
        owner: &signer,
        creators_address: address,
        collection: String,
        name: String,
        property_version: u64,
        amount: u64
    ) acquires Collections, TokenStore {
        assert!(amount > 0, error::invalid_argument(ENO_BURN_TOKEN_WITH_ZERO_AMOUNT));
        let token_id = create_token_id_raw(creators_address, collection, name, property_version);
        let creator_addr = token_id.token_data_id.creator;
        assert!(
            exists<Collections>(creator_addr),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let collections = &mut Collections[creator_addr];
        assert!(
            collections.token_data.contains(token_id.token_data_id),
            error::not_found(ETOKEN_DATA_NOT_PUBLISHED),
        );

        let token_data = collections.token_data.borrow_mut(token_id.token_data_id);

        assert!(
            token_data.default_properties.contains_key(&string::utf8(BURNABLE_BY_OWNER)),
            error::permission_denied(EOWNER_CANNOT_BURN_TOKEN)
        );
        let burn_by_owner_flag = token_data.default_properties.read_bool(&string::utf8(BURNABLE_BY_OWNER));
        assert!(burn_by_owner_flag, error::permission_denied(EOWNER_CANNOT_BURN_TOKEN));

        // Burn the tokens.
        let Token { id: _, amount: burned_amount, token_properties: _ } = withdraw_token(owner, token_id, amount);
        let token_store = &mut TokenStore[signer::address_of(owner)];
        if (std::features::module_event_migration_enabled()) {
            event::emit(Burn { account: signer::address_of(owner), id: token_id, amount: burned_amount });
        } else {
            event::emit_event<BurnTokenEvent>(
                &mut token_store.burn_events,
                BurnTokenEvent { id: token_id, amount: burned_amount }
            );
        };

        // Decrease the supply correspondingly by the amount of tokens burned.
        let token_data = collections.token_data.borrow_mut(token_id.token_data_id);

        // only update the supply if we tracking the supply and maximal
        // maximal == 0 is reserved for unlimited token and collection with no tracking info.
        if (token_data.maximum > 0) {
            token_data.supply -= burned_amount;

            // Delete the token_data if supply drops to 0.
            if (token_data.supply == 0) {
                destroy_token_data(collections.token_data.remove(token_id.token_data_id));

                // update the collection supply
                let collection_data = collections.collection_data.borrow_mut(token_id.token_data_id.collection);

                // only update and check the supply for unlimited collection
                if (collection_data.maximum > 0){
                    collection_data.supply -= 1;
                    // delete the collection data if the collection supply equals 0
                    if (collection_data.supply == 0) {
                        destroy_collection_data(collections.collection_data.remove(collection_data.name));
                    };
                };
            };
        };
    }

    //
    // Public functions for creating and maintaining tokens
    //

    // Functions for mutating CollectionData fields
    public fun mutate_collection_description(creator: &signer, collection_name: String, description: String) acquires Collections {
        let creator_address = signer::address_of(creator);
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );
        assert!(collection_data.mutability_config.description, error::permission_denied(EFIELD_NOT_MUTABLE));
        token_event_store::emit_collection_description_mutate_event(creator, collection_name, collection_data.description, description);
        collection_data.description = description;
    }

    public fun mutate_collection_uri(creator: &signer, collection_name: String, uri: String) acquires Collections {
        assert!(uri.length() <= MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
        let creator_address = signer::address_of(creator);
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );
        assert!(collection_data.mutability_config.uri, error::permission_denied(EFIELD_NOT_MUTABLE));
        token_event_store::emit_collection_uri_mutate_event(creator, collection_name, collection_data.uri , uri);
        collection_data.uri = uri;
    }

    public fun mutate_collection_maximum(creator: &signer, collection_name: String, maximum: u64) acquires Collections {
        let creator_address = signer::address_of(creator);
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );
        // cannot change maximum from 0 and cannot change maximum to 0
        assert!(collection_data.maximum != 0 && maximum != 0, error::invalid_argument(EINVALID_MAXIMUM));
        assert!(maximum >= collection_data.supply, error::invalid_argument(EINVALID_MAXIMUM));
        assert!(collection_data.mutability_config.maximum, error::permission_denied(EFIELD_NOT_MUTABLE));
        token_event_store::emit_collection_maximum_mutate_event(creator, collection_name, collection_data.maximum, maximum);
        collection_data.maximum = maximum;
    }

    // Functions for mutating TokenData fields
    public fun mutate_tokendata_maximum(creator: &signer, token_data_id: TokenDataId, maximum: u64) acquires Collections {
        assert_tokendata_exists(creator, token_data_id);
        let all_token_data = &mut Collections[token_data_id.creator].token_data;
        let token_data = all_token_data.borrow_mut(token_data_id);
        // cannot change maximum from 0 and cannot change maximum to 0
        assert!(token_data.maximum != 0 && maximum != 0, error::invalid_argument(EINVALID_MAXIMUM));
        assert!(maximum >= token_data.supply, error::invalid_argument(EINVALID_MAXIMUM));
        assert!(token_data.mutability_config.maximum, error::permission_denied(EFIELD_NOT_MUTABLE));
        token_event_store::emit_token_maximum_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.maximum, maximum);
        token_data.maximum = maximum;
    }

    public fun mutate_tokendata_uri(
        creator: &signer,
        token_data_id: TokenDataId,
        uri: String
    ) acquires Collections {
        assert!(uri.length() <= MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
        assert_tokendata_exists(creator, token_data_id);

        let all_token_data = &mut Collections[token_data_id.creator].token_data;
        let token_data = all_token_data.borrow_mut(token_data_id);
        assert!(token_data.mutability_config.uri, error::permission_denied(EFIELD_NOT_MUTABLE));
        token_event_store::emit_token_uri_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.uri ,uri);
        token_data.uri = uri;
    }

    public fun mutate_tokendata_royalty(creator: &signer, token_data_id: TokenDataId, royalty: Royalty) acquires Collections {
        assert_tokendata_exists(creator, token_data_id);

        let all_token_data = &mut Collections[token_data_id.creator].token_data;
        let token_data = all_token_data.borrow_mut(token_data_id);
        assert!(token_data.mutability_config.royalty, error::permission_denied(EFIELD_NOT_MUTABLE));

        token_event_store::emit_token_royalty_mutate_event(
            creator,
            token_data_id.collection,
            token_data_id.name,
            token_data.royalty.royalty_points_numerator,
            token_data.royalty.royalty_points_denominator,
            token_data.royalty.payee_address,
            royalty.royalty_points_numerator,
            royalty.royalty_points_denominator,
            royalty.payee_address
        );
        token_data.royalty = royalty;
    }

    public fun mutate_tokendata_description(creator: &signer, token_data_id: TokenDataId, description: String) acquires Collections {
        assert_tokendata_exists(creator, token_data_id);

        let all_token_data = &mut Collections[token_data_id.creator].token_data;
        let token_data = all_token_data.borrow_mut(token_data_id);
        assert!(token_data.mutability_config.description, error::permission_denied(EFIELD_NOT_MUTABLE));
        token_event_store::emit_token_descrition_mutate_event(creator, token_data_id.collection, token_data_id.name, token_data.description, description);
        token_data.description = description;
    }

    /// Allow creator to mutate the default properties in TokenData
    public fun mutate_tokendata_property(
        creator: &signer,
        token_data_id: TokenDataId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) acquires Collections {
        assert_tokendata_exists(creator, token_data_id);
        let key_len = keys.length();
        let val_len = values.length();
        let typ_len = types.length();
        assert!(key_len == val_len, error::invalid_state(ETOKEN_PROPERTIES_COUNT_NOT_MATCH));
        assert!(key_len == typ_len, error::invalid_state(ETOKEN_PROPERTIES_COUNT_NOT_MATCH));

        let all_token_data = &mut Collections[token_data_id.creator].token_data;
        let token_data = all_token_data.borrow_mut(token_data_id);
        assert!(token_data.mutability_config.properties, error::permission_denied(EFIELD_NOT_MUTABLE));
        let old_values: vector<Option<PropertyValue>> = vector::empty();
        let new_values: vector<PropertyValue> = vector::empty();
        assert_non_standard_reserved_property(&keys);
        for (i in 0..keys.length()){
            let key = keys.borrow(i);
            let old_pv = if (token_data.default_properties.contains_key(key)) {
                option::some(*token_data.default_properties.borrow(key))
            } else {
                option::none<PropertyValue>()
            };
            old_values.push_back(old_pv);
            let new_pv = property_map::create_property_value_raw(values[i], types[i]);
            new_values.push_back(new_pv);
            if (old_pv.is_some()) {
                token_data.default_properties.update_property_value(key, new_pv);
            } else {
                token_data.default_properties.add(*key, new_pv);
            };
        };
        token_event_store::emit_default_property_mutate_event(creator, token_data_id.collection, token_data_id.name, keys, old_values, new_values);
    }

    /// Mutate the token_properties of one token.
    public fun mutate_one_token(
        account: &signer,
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ): TokenId acquires Collections, TokenStore {
        let creator = token_id.token_data_id.creator;
        assert!(signer::address_of(account) == creator, error::permission_denied(ENO_MUTATE_CAPABILITY));
        // validate if the properties is mutable
        assert!(exists<Collections>(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &mut Collections[
            creator
        ].token_data;

        assert!(all_token_data.contains(token_id.token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        let token_data = all_token_data.borrow_mut(token_id.token_data_id);

        // if default property is mutatable, token property is always mutable
        // we only need to check TOKEN_PROPERTY_MUTABLE when default property is immutable
        if (!token_data.mutability_config.properties) {
            assert!(
                token_data.default_properties.contains_key(&string::utf8(TOKEN_PROPERTY_MUTABLE)),
                error::permission_denied(EFIELD_NOT_MUTABLE)
            );

            let token_prop_mutable = token_data.default_properties.read_bool(&string::utf8(TOKEN_PROPERTY_MUTABLE));
            assert!(token_prop_mutable, error::permission_denied(EFIELD_NOT_MUTABLE));
        };

        // check if the property_version is 0 to determine if we need to update the property_version
        if (token_id.property_version == 0) {
            let token = withdraw_with_event_internal(token_owner, token_id, 1);
            // give a new property_version for each token
            let cur_property_version = token_data.largest_property_version + 1;
            let new_token_id = create_token_id(token_id.token_data_id, cur_property_version);
            let new_token = Token {
                id: new_token_id,
                amount: 1,
                token_properties: token_data.default_properties,
            };
            direct_deposit(token_owner, new_token);
            update_token_property_internal(token_owner, new_token_id, keys, values, types);
            if (std::features::module_event_migration_enabled()) {
                event::emit(MutatePropertyMap {
                    account: token_owner,
                    old_id: token_id,
                    new_id: new_token_id,
                    keys,
                    values,
                    types
                });
            } else {
                event::emit_event<MutateTokenPropertyMapEvent>(
                    &mut TokenStore[token_owner].mutate_token_property_events,
                    MutateTokenPropertyMapEvent {
                        old_id: token_id,
                        new_id: new_token_id,
                        keys,
                        values,
                        types
                    },
                );
            };

            token_data.largest_property_version = cur_property_version;
            // burn the orignial property_version 0 token after mutation
            let Token { id: _, amount: _, token_properties: _ } = token;
            new_token_id
        } else {
            // only 1 copy for the token with property verion bigger than 0
            update_token_property_internal(token_owner, token_id, keys, values, types);
            if (std::features::module_event_migration_enabled()) {
                event::emit(MutatePropertyMap {
                    account: token_owner,
                    old_id: token_id,
                    new_id: token_id,
                    keys,
                    values,
                    types
                });
            } else {
                event::emit_event<MutateTokenPropertyMapEvent>(
                    &mut TokenStore[token_owner].mutate_token_property_events,
                    MutateTokenPropertyMapEvent {
                        old_id: token_id,
                        new_id: token_id,
                        keys,
                        values,
                        types
                    },
                );
            };
            token_id
        }
    }

    public fun create_royalty(royalty_points_numerator: u64, royalty_points_denominator: u64, payee_address: address): Royalty {
        assert!(royalty_points_numerator <= royalty_points_denominator, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));
        // Question[Orderless]: Is it okay to remove this check to accommodate stateless accounts?
        // assert!(account::exists_at(payee_address), error::invalid_argument(EROYALTY_PAYEE_ACCOUNT_DOES_NOT_EXIST));
        Royalty {
            royalty_points_numerator,
            royalty_points_denominator,
            payee_address
        }
    }

    /// Deposit the token balance into the owner's account and emit an event.
    public fun deposit_token(account: &signer, token: Token) acquires TokenStore {
        let account_addr = signer::address_of(account);
        initialize_token_store(account);
        direct_deposit(account_addr, token)
    }

    /// direct deposit if user opt in direct transfer
    public fun direct_deposit_with_opt_in(account_addr: address, token: Token) acquires TokenStore {
        let opt_in_transfer = TokenStore[account_addr].direct_transfer;
        assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));
        direct_deposit(account_addr, token);
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
        dst_token.amount += source_token.amount;
        let Token { id: _, amount: _, token_properties: _ } = source_token;
    }

    public fun split(dst_token: &mut Token, amount: u64): Token {
        assert!(dst_token.id.property_version == 0, error::invalid_state(ENFT_NOT_SPLITABLE));
        assert!(dst_token.amount > amount, error::invalid_argument(ETOKEN_SPLIT_AMOUNT_LARGER_OR_EQUAL_TO_TOKEN_AMOUNT));
        assert!(amount > 0, error::invalid_argument(ETOKEN_CANNOT_HAVE_ZERO_AMOUNT));
        dst_token.amount -= amount;
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
        let opt_in_transfer = TokenStore[to].direct_transfer;
        assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));
        let token = withdraw_token(from, id, amount);
        direct_deposit(to, token);
    }


    /// Token owner can create this one-time withdraw capability with an expiration time
    public fun create_withdraw_capability(
        owner: &signer,
        token_id: TokenId,
        amount: u64,
        expiration_sec: u64,
    ): WithdrawCapability {
        WithdrawCapability {
            token_owner: signer::address_of(owner),
            token_id,
            amount,
            expiration_sec,
        }
    }

    /// Withdraw the token with a capability
    public fun withdraw_with_capability(
        withdraw_proof: WithdrawCapability,
    ): Token acquires TokenStore {
        // verify the delegation hasn't expired yet
        assert!(timestamp::now_seconds() <= withdraw_proof.expiration_sec, error::invalid_argument(EWITHDRAW_PROOF_EXPIRES));

        withdraw_with_event_internal(
            withdraw_proof.token_owner,
            withdraw_proof.token_id,
            withdraw_proof.amount,
        )
    }

    /// Withdraw the token with a capability.
    public fun partial_withdraw_with_capability(
        withdraw_proof: WithdrawCapability,
        withdraw_amount: u64,
    ): (Token, Option<WithdrawCapability>) acquires TokenStore {
        // verify the delegation hasn't expired yet
        assert!(timestamp::now_seconds() <= withdraw_proof.expiration_sec, error::invalid_argument(EWITHDRAW_PROOF_EXPIRES));

        assert!(withdraw_amount <= withdraw_proof.amount, error::invalid_argument(EINSUFFICIENT_WITHDRAW_CAPABILITY_AMOUNT));

        let res: Option<WithdrawCapability> = if (withdraw_amount == withdraw_proof.amount) {
            option::none<WithdrawCapability>()
        } else {
            option::some(
                WithdrawCapability {
                    token_owner: withdraw_proof.token_owner,
                    token_id: withdraw_proof.token_id,
                    amount: withdraw_proof.amount - withdraw_amount,
                    expiration_sec: withdraw_proof.expiration_sec,
                }
            )
        };

        (
            withdraw_with_event_internal(
                withdraw_proof.token_owner,
                withdraw_proof.token_id,
                withdraw_amount,
            ),
            res
        )

    }

    public fun withdraw_token(
        account: &signer,
        id: TokenId,
        amount: u64,
    ): Token acquires TokenStore {
        let account_addr = signer::address_of(account);
        withdraw_with_event_internal(account_addr, id, amount)
    }

    /// Create a new collection to hold tokens
    public fun create_collection(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        maximum: u64,
        mutate_setting: vector<bool>
    ) acquires Collections {
        assert!(name.length() <= MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));
        assert!(uri.length() <= MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
        let account_addr = signer::address_of(creator);
        if (!exists<Collections>(account_addr)) {
            move_to(
                creator,
                Collections {
                    collection_data: table::new(),
                    token_data: table::new(),
                    create_collection_events: account::new_event_handle<CreateCollectionEvent>(creator),
                    create_token_data_events: account::new_event_handle<CreateTokenDataEvent>(creator),
                    mint_token_events: account::new_event_handle<MintTokenEvent>(creator),
                },
            )
        };

        let collection_data = &mut Collections[account_addr].collection_data;

        assert!(
            !collection_data.contains(name),
            error::already_exists(ECOLLECTION_ALREADY_EXISTS),
        );

        let mutability_config = create_collection_mutability_config(&mutate_setting);
        let collection = CollectionData {
            description,
            name,
            uri,
            supply: 0,
            maximum,
            mutability_config
        };

        collection_data.add(name, collection);
        let collection_handle = &mut Collections[account_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                CreateCollection {
                    creator: account_addr,
                    collection_name: name,
                    uri,
                    description,
                    maximum,
                }
            );
        } else {
            event::emit_event<CreateCollectionEvent>(
                &mut collection_handle.create_collection_events,
                CreateCollectionEvent {
                    creator: account_addr,
                    collection_name: name,
                    uri,
                    description,
                    maximum,
                }
            );
        };
    }

    public fun check_collection_exists(creator: address, name: String): bool acquires Collections {
        assert!(
            exists<Collections>(creator),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let collection_data = &Collections[creator].collection_data;
        collection_data.contains(name)
    }

    public fun check_tokendata_exists(creator: address, collection_name: String, token_name: String): bool acquires Collections {
        assert!(
            exists<Collections>(creator),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );

        let token_data = &Collections[creator].token_data;
        let token_data_id = create_token_data_id(creator, collection_name, token_name);
        token_data.contains(token_data_id)
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
        assert!(name.length() <= MAX_NFT_NAME_LENGTH, error::invalid_argument(ENFT_NAME_TOO_LONG));
        assert!(collection.length() <= MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));
        assert!(uri.length() <= MAX_URI_LENGTH, error::invalid_argument(EURI_TOO_LONG));
        assert!(royalty_points_numerator <= royalty_points_denominator, error::invalid_argument(EINVALID_ROYALTY_NUMERATOR_DENOMINATOR));

        let account_addr = signer::address_of(account);
        assert!(
            exists<Collections>(account_addr),
            error::not_found(ECOLLECTIONS_NOT_PUBLISHED),
        );
        let collections = &mut Collections[account_addr];

        let token_data_id = create_token_data_id(account_addr, collection, name);

        assert!(
            collections.collection_data.contains(token_data_id.collection),
            error::not_found(ECOLLECTION_NOT_PUBLISHED),
        );
        assert!(
            !collections.token_data.contains(token_data_id),
            error::already_exists(ETOKEN_DATA_ALREADY_EXISTS),
        );

        let collection = collections.collection_data.borrow_mut(token_data_id.collection);

        // if collection maximum == 0, user don't want to enforce supply constraint.
        // we don't track supply to make token creation parallelizable
        if (collection.maximum > 0) {
            collection.supply += 1;
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
            royalty: create_royalty(royalty_points_numerator, royalty_points_denominator, royalty_payee_address),
            name,
            description,
            default_properties: property_map::new(property_keys, property_values, property_types),
            mutability_config: token_mutate_config,
        };

        collections.token_data.add(token_data_id, token_data);
        if (std::features::module_event_migration_enabled()) {
            event::emit(
                TokenDataCreation {
                    creator: account_addr,
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
                }
            );
        } else {
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
        };

        token_data_id
    }

    /// return the number of distinct token_data_id created under this collection
    public fun get_collection_supply(creator_address: address, collection_name: String): Option<u64> acquires Collections {
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );

        if (collection_data.maximum > 0) {
            option::some(collection_data.supply)
        } else {
            option::none()
        }
    }

    public fun get_collection_description(creator_address: address, collection_name: String): String acquires Collections {
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );
        collection_data.description
    }

    public fun get_collection_uri(creator_address: address, collection_name: String): String acquires Collections {
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );
        collection_data.uri
    }

    public fun get_collection_maximum(creator_address: address, collection_name: String): u64 acquires Collections {
        assert_collection_exists(creator_address, collection_name);
        let collection_data = Collections[creator_address].collection_data.borrow_mut(
            collection_name
        );
        collection_data.maximum
    }

    /// return the number of distinct token_id created under this TokenData
    public fun get_token_supply(creator_address: address, token_data_id: TokenDataId): Option<u64> acquires Collections {
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_address].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        let token_data = all_token_data.borrow(token_data_id);

        if (token_data.maximum > 0) {
            option::some(token_data.supply)
        } else {
            option::none<u64>()
        }
    }

    /// return the largest_property_version of this TokenData
    public fun get_tokendata_largest_property_version(creator_address: address, token_data_id: TokenDataId): u64 acquires Collections {
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_address].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        all_token_data.borrow(token_data_id).largest_property_version
    }

    /// return the TokenId for a given Token
    public fun get_token_id(token: &Token): TokenId {
        token.id
    }

    public fun get_direct_transfer(receiver: address): bool acquires TokenStore {
        if (!exists<TokenStore>(receiver)) {
            return false
        };

        TokenStore[receiver].direct_transfer
    }

    public fun create_token_mutability_config(mutate_setting: &vector<bool>): TokenMutabilityConfig {
        TokenMutabilityConfig {
            maximum: mutate_setting[TOKEN_MAX_MUTABLE_IND],
            uri: mutate_setting[TOKEN_URI_MUTABLE_IND],
            royalty: mutate_setting[TOKEN_ROYALTY_MUTABLE_IND],
            description: mutate_setting[TOKEN_DESCRIPTION_MUTABLE_IND],
            properties: mutate_setting[TOKEN_PROPERTY_MUTABLE_IND],
        }
    }

    public fun create_collection_mutability_config(mutate_setting: &vector<bool>): CollectionMutabilityConfig {
        CollectionMutabilityConfig {
            description: mutate_setting[COLLECTION_DESCRIPTION_MUTABLE_IND],
            uri: mutate_setting[COLLECTION_URI_MUTABLE_IND],
            maximum: mutate_setting[COLLECTION_MAX_MUTABLE_IND],
        }
    }

    public fun mint_token(
        account: &signer,
        token_data_id: TokenDataId,
        amount: u64,
    ): TokenId acquires Collections, TokenStore {
        assert!(token_data_id.creator == signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
        let creator_addr = token_data_id.creator;
        let all_token_data = &mut Collections[creator_addr].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        let token_data = all_token_data.borrow_mut(token_data_id);

        if (token_data.maximum > 0) {
            assert!(token_data.supply + amount <= token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));
            token_data.supply += amount;
        };

        // we add more tokens with property_version 0
        let token_id = create_token_id(token_data_id, 0);
        if (std::features::module_event_migration_enabled()) {
            event::emit(Mint { creator: creator_addr, id: token_data_id, amount })
        } else {
            event::emit_event<MintTokenEvent>(
                &mut Collections[creator_addr].mint_token_events,
                MintTokenEvent {
                    id: token_data_id,
                    amount,
                }
            );
        };

        deposit_token(account,
            Token {
                id: token_id,
                amount,
                token_properties: property_map::empty(), // same as default properties no need to store
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
        let opt_in_transfer = TokenStore[receiver].direct_transfer;
        assert!(opt_in_transfer, error::permission_denied(EUSER_NOT_OPT_IN_DIRECT_TRANSFER));

        assert!(token_data_id.creator == signer::address_of(account), error::permission_denied(ENO_MINT_CAPABILITY));
        let creator_addr = token_data_id.creator;
        let all_token_data = &mut Collections[creator_addr].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        let token_data = all_token_data.borrow_mut(token_data_id);

        if (token_data.maximum > 0) {
            assert!(token_data.supply + amount <= token_data.maximum, error::invalid_argument(EMINT_WOULD_EXCEED_TOKEN_MAXIMUM));
            token_data.supply += amount;
        };

        // we add more tokens with property_version 0
        let token_id = create_token_id(token_data_id, 0);

        if (std::features::module_event_migration_enabled()) {
            event::emit(Mint { creator: creator_addr, id: token_data_id, amount })
        } else {
            event::emit_event<MintTokenEvent>(
                &mut Collections[creator_addr].mint_token_events,
                MintTokenEvent {
                    id: token_data_id,
                    amount,
                }
            );
        };

        direct_deposit(receiver,
            Token {
                id: token_id,
                amount,
                token_properties: property_map::empty(), // same as default properties no need to store
            }
        );
    }

    public fun create_token_id(token_data_id: TokenDataId, property_version: u64): TokenId {
        TokenId {
            token_data_id,
            property_version,
        }
    }

    public fun create_token_data_id(
        creator: address,
        collection: String,
        name: String,
    ): TokenDataId {
        assert!(collection.length() <= MAX_COLLECTION_NAME_LENGTH, error::invalid_argument(ECOLLECTION_NAME_TOO_LONG));
        assert!(name.length() <= MAX_NFT_NAME_LENGTH, error::invalid_argument(ENFT_NAME_TOO_LONG));
        TokenDataId { creator, collection, name }
    }

    public fun create_token_id_raw(
        creator: address,
        collection: String,
        name: String,
        property_version: u64,
    ): TokenId {
        TokenId {
            token_data_id: create_token_data_id(creator, collection, name),
            property_version,
        }
    }

    public fun balance_of(owner: address, id: TokenId): u64 acquires TokenStore {
        if (!exists<TokenStore>(owner)) {
            return 0
        };
        let token_store = &TokenStore[owner];
        if (token_store.tokens.contains(id)) {
            token_store.tokens.borrow(id).amount
        } else {
            0
        }
    }

    public fun has_token_store(owner: address): bool {
        exists<TokenStore>(owner)
    }

    public fun get_royalty(token_id: TokenId): Royalty acquires Collections {
        let token_data_id = token_id.token_data_id;
        get_tokendata_royalty(token_data_id)
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
            let all_token_data = &Collections[creator_addr].token_data;
            assert!(all_token_data.contains(token_id.token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
            let token_data = all_token_data.borrow(token_id.token_data_id);
            token_data.default_properties
        } else {
            let tokens = &TokenStore[owner].tokens;
            tokens.borrow(token_id).token_properties
        }
    }

    public fun get_tokendata_maximum(token_data_id: TokenDataId): u64 acquires Collections {
        let creator_address = token_data_id.creator;
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_address].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

        let token_data = all_token_data.borrow(token_data_id);
        token_data.maximum
    }

    public fun get_tokendata_uri(creator: address, token_data_id: TokenDataId): String acquires Collections {
        assert!(exists<Collections>(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

        let token_data = all_token_data.borrow(token_data_id);
        token_data.uri
    }

    public fun get_tokendata_description(token_data_id: TokenDataId): String acquires Collections {
        let creator_address = token_data_id.creator;
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_address].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

        let token_data = all_token_data.borrow(token_data_id);
        token_data.description
    }

    public fun get_tokendata_royalty(token_data_id: TokenDataId): Royalty acquires Collections {
        let creator_address = token_data_id.creator;
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_address].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));

        let token_data = all_token_data.borrow(token_data_id);
        token_data.royalty
    }

    /// return the token_data_id from the token_id
    public fun get_tokendata_id(token_id: TokenId): TokenDataId {
        token_id.token_data_id
    }

    /// return the mutation setting of the token
    public fun get_tokendata_mutability_config(token_data_id: TokenDataId): TokenMutabilityConfig acquires Collections {
        let creator_addr = token_data_id.creator;
        assert!(exists<Collections>(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_addr].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
        all_token_data.borrow(token_data_id).mutability_config
    }

    /// return if the token's maximum is mutable
    public fun get_token_mutability_maximum(config: &TokenMutabilityConfig): bool {
        config.maximum
    }

    /// return if the token royalty is mutable with a token mutability config
    public fun get_token_mutability_royalty(config: &TokenMutabilityConfig): bool {
        config.royalty
    }

    /// return if the token uri is mutable with a token mutability config
    public fun get_token_mutability_uri(config: &TokenMutabilityConfig): bool {
        config.uri
    }

    /// return if the token description is mutable with a token mutability config
    public fun get_token_mutability_description(config: &TokenMutabilityConfig): bool {
        config.description
    }

    /// return if the tokendata's default properties is mutable with a token mutability config
    public fun get_token_mutability_default_properties(config: &TokenMutabilityConfig): bool {
        config.properties
    }

    #[view]
    /// return the collection mutation setting
    public fun get_collection_mutability_config(
        creator: address,
        collection_name: String
    ): CollectionMutabilityConfig acquires Collections {
        assert!(exists<Collections>(creator), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_collection_data = &Collections[creator].collection_data;
        assert!(all_collection_data.contains(collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));
        all_collection_data.borrow(collection_name).mutability_config
    }

    /// return if the collection description is mutable with a collection mutability config
    public fun get_collection_mutability_description(config: &CollectionMutabilityConfig): bool {
        config.description
    }

    /// return if the collection uri is mutable with a collection mutability config
    public fun get_collection_mutability_uri(config: &CollectionMutabilityConfig): bool {
        config.uri
    }
    /// return if the collection maximum is mutable with collection mutability config
    public fun get_collection_mutability_maximum(config: &CollectionMutabilityConfig): bool {
        config.maximum
    }

    //
    // Private functions
    //
    fun destroy_token_data(token_data: TokenData) {
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
        } = token_data;
    }

    fun destroy_collection_data(collection_data: CollectionData) {
        let CollectionData {
            description: _,
            name: _,
            uri: _,
            supply: _,
            maximum: _,
            mutability_config: _,
        } = collection_data;
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

        let token_store = &mut TokenStore[account_addr];
        if (std::features::module_event_migration_enabled()) {
            event::emit(TokenWithdraw { account: account_addr, id, amount })
        } else {
            event::emit_event<WithdrawEvent>(
                &mut token_store.withdraw_events,
                WithdrawEvent { id, amount }
            );
        };

        let tokens = &mut TokenStore[account_addr].tokens;
        assert!(
            tokens.contains(id),
            error::not_found(ENO_TOKEN_IN_TOKEN_STORE),
        );
        // balance > amount and amount > 0 indirectly asserted that balance > 0.
        let balance = &mut tokens.borrow_mut(id).amount;
        if (*balance > amount) {
            *balance -= amount;
            Token { id, amount, token_properties: property_map::empty() }
        } else {
            tokens.remove(id)
        }
    }

    fun update_token_property_internal(
        token_owner: address,
        token_id: TokenId,
        keys: vector<String>,
        values: vector<vector<u8>>,
        types: vector<String>,
    ) acquires TokenStore {
        let tokens = &mut TokenStore[token_owner].tokens;
        assert!(tokens.contains(token_id), error::not_found(ENO_TOKEN_IN_TOKEN_STORE));

        let value = &mut tokens.borrow_mut(token_id).token_properties;
        assert_non_standard_reserved_property(&keys);
        value.update_property_map(keys, values, types);
    }

    /// Deposit the token balance into the recipients account and emit an event.
    fun direct_deposit(account_addr: address, token: Token) acquires TokenStore {
        assert!(token.amount > 0, error::invalid_argument(ETOKEN_CANNOT_HAVE_ZERO_AMOUNT));
        let token_store = &mut TokenStore[account_addr];

        if (std::features::module_event_migration_enabled()) {
            event::emit(TokenDeposit { account: account_addr, id: token.id, amount: token.amount });
        } else {
            event::emit_event<DepositEvent>(
                &mut token_store.deposit_events,
                DepositEvent { id: token.id, amount: token.amount },
            );
        };

        assert!(
            exists<TokenStore>(account_addr),
            error::not_found(ETOKEN_STORE_NOT_PUBLISHED),
        );

        if (!token_store.tokens.contains(token.id)) {
            token_store.tokens.add(token.id, token);
        } else {
            let recipient_token = token_store.tokens.borrow_mut(token.id);
            merge(recipient_token, token);
        };
    }

    fun assert_collection_exists(creator_address: address, collection_name: String) acquires Collections {
        assert!(exists<Collections>(creator_address), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_collection_data = &Collections[creator_address].collection_data;
        assert!(all_collection_data.contains(collection_name), error::not_found(ECOLLECTION_NOT_PUBLISHED));
    }

    fun assert_tokendata_exists(creator: &signer, token_data_id: TokenDataId) acquires Collections {
        let creator_addr = token_data_id.creator;
        assert!(signer::address_of(creator) == creator_addr, error::permission_denied(ENO_MUTATE_CAPABILITY));
        assert!(exists<Collections>(creator_addr), error::not_found(ECOLLECTIONS_NOT_PUBLISHED));
        let all_token_data = &Collections[creator_addr].token_data;
        assert!(all_token_data.contains(token_data_id), error::not_found(ETOKEN_DATA_NOT_PUBLISHED));
    }

    fun assert_non_standard_reserved_property(keys: &vector<String>) {
        keys.for_each_ref(|key| {
            let key: &String = key;
            let length = key.length();
            if (length >= 6) {
                let prefix = key.sub_string(0, 6);
                assert!(prefix != string::utf8(b"TOKEN_"), error::permission_denied(EPROPERTY_RESERVED_BY_STANDARD));
            };
        });
    }

    // ****************** TEST-ONLY FUNCTIONS **************

    #[test(creator = @0x1, owner = @0x2)]
    public fun create_withdraw_deposit_token(
        creator: signer,
        owner: signer
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(&creator));
        account::create_account_for_test(signer::address_of(&owner));
        let token_id = create_collection_and_token(
            &creator,
            1,
            1,
            1,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

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
        let token_id = create_collection_and_token(
            &creator,
            2,
            5,
            5,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

        let token_0 = withdraw_token(&creator, token_id, 1);
        let token_1 = withdraw_token(&creator, token_id, 1);
        deposit_token(&owner, token_0);
        deposit_token(&creator, token_1);
        let token_2 = withdraw_token(&creator, token_id, 1);
        deposit_token(&owner, token_2);
    }

    #[test(creator = @0x1)]
    #[expected_failure]
    public entry fun test_collection_maximum(creator: signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(&creator));
        let token_id = create_collection_and_token(
            &creator,
            2,
            2,
            1,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
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
        let token_id = create_collection_and_token(
            &creator,
            2,
            2,
            2,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        assert!(balance_of(signer::address_of(&owner), token_id) == 0, 1);

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
    public fun create_collection_and_token(
        creator: &signer,
        amount: u64,
        collection_max: u64,
        token_max: u64,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>,
        collection_mutate_setting: vector<bool>,
        token_mutate_setting: vector<bool>,
    ): TokenId acquires Collections, TokenStore {
        use std::string;
        use std::bcs;
        let mutate_setting = collection_mutate_setting;

        create_collection(
            creator,
            get_collection_name(),
            string::utf8(b"Collection: Hello, World"),
            string::utf8(b"https://aptos.dev"),
            collection_max,
            mutate_setting
        );

        let default_keys = if (property_keys.length<String>() == 0) { vector<String>[string::utf8(b"attack"), string::utf8(b"num_of_use")] } else { property_keys };
        let default_vals = if (property_values.length<vector<u8>>() == 0) { vector<vector<u8>>[bcs::to_bytes<u64>(&10), bcs::to_bytes<u64>(&5)] } else { property_values };
        let default_types = if (property_types.length<String>() == 0) { vector<String>[string::utf8(b"u64"), string::utf8(b"u64")] } else { property_types };
        let mutate_setting = token_mutate_setting;
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
        create_collection_and_token(
            &creator,
            1,
            2,
            1,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        assert!(
            event::emitted_events<CreateCollection>().length() == 1 || event::emitted_events<CreateCollectionEvent>().length(
            ) == 1,
            1
        );
    }

    #[test(creator = @0xAF)]
    fun test_mint_token_from_tokendata(creator: &signer) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));

        create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
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

    #[test(creator = @0xAF)]
    fun test_mutate_token_property_upsert(creator: &signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[string::utf8(TOKEN_PROPERTY_MUTABLE)],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        assert!(token_id.property_version == 0, 1);
        // only be able to mutate the attributed defined when creating the token
        let new_keys = vector<String>[
            string::utf8(b"attack"), string::utf8(b"num_of_use"), string::utf8(b"new_attribute")
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

    #[test(creator = @0xAF)]
    fun test_get_property_map_should_not_update_source_value(creator: &signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );
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
        assert!(pm.length() == 2, 1);
        let new_token_id = mutate_one_token(
            creator,
            signer::address_of(creator),
            token_id,
            new_keys,
            new_vals,
            new_types,
        );
        let updated_pm = get_property_map(signer::address_of(creator), new_token_id);
        assert!(updated_pm.length() == 2, 1);
        updated_pm.update_property_value(&string::utf8(b"attack"), property_map::create_property_value<u64>(&2));

        assert!(updated_pm.read_u64(&string::utf8(b"attack")) == 2, 1);
        let og_pm = get_property_map(signer::address_of(creator), new_token_id);
        assert!(og_pm.read_u64(&string::utf8(b"attack")) == 1, 1);
    }

    #[test(framework = @0x1, creator = @0xcafe)]
    fun test_withdraw_with_proof(creator: &signer, framework: &signer): Token acquires TokenStore, Collections {
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(signer::address_of(creator));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

        timestamp::update_global_time_for_test(1000000);

        // provide the proof to the account
        let cap = create_withdraw_capability(
            creator, // ask user to provide address to avoid ambiguity from rotated keys
            token_id,
            1,
            2000000,
        );

        withdraw_with_capability(cap)
    }

    #[test(creator = @0xcafe, another_creator = @0xde)]
    fun test_burn_token_from_both_limited_and_unlimited(
        creator: &signer,
        another_creator: &signer,
    )acquires Collections, TokenStore {
        // create limited token and collection
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(another_creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[string::utf8(BURNABLE_BY_CREATOR)],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        // burn token from limited token
        let creator_addr = signer::address_of(creator);
        let pre_amount = &mut get_token_supply(creator_addr, token_id.token_data_id);
        burn_by_creator(creator, creator_addr, get_collection_name(), get_token_name(), 0, 1);
        let aft_amount = &mut get_token_supply(creator_addr, token_id.token_data_id);
        assert!((pre_amount.extract<u64>() - aft_amount.extract<u64>()) == 1, 1);

        // create unlimited token and collection
        let new_addr = signer::address_of(another_creator);
        let new_token_id = create_collection_and_token(
            another_creator,
            2,
            0,
            0,
            vector<String>[string::utf8(BURNABLE_BY_OWNER)],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let pre = balance_of(new_addr, new_token_id);
        // burn token from unlimited token and collection
        burn(another_creator, new_addr, get_collection_name(), get_token_name(), 0, 1);
        let aft = balance_of(new_addr, new_token_id);
        assert!(pre - aft == 1, 1);
    }

    #[test(creator = @0xcafe, owner = @0xafe)]
    fun test_mint_token_to_different_address(
        creator: &signer,
        owner: &signer,
    )acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let owner_addr = signer::address_of(owner);
        opt_in_direct_transfer(owner, true);
        mint_token_to(creator, owner_addr, token_id.token_data_id, 1);
        assert!(balance_of(owner_addr, token_id) == 1, 1);
    }

    #[test(creator = @0xcafe, owner = @0xafe)]
    #[expected_failure(abort_code = 327696, location = Self)]
    fun test_opt_in_direct_transfer_fail(
        creator: &signer,
        owner: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let owner_addr = signer::address_of(owner);
        initialize_token_store(owner);
        transfer(creator, token_id, owner_addr, 1);
    }

    #[test(creator = @0xcafe, owner = @0xafe)]
    #[expected_failure(abort_code = 327696, location = Self)]
    fun test_opt_in_direct_deposit_fail(
        creator: &signer,
        owner: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        let owner_addr = signer::address_of(owner);
        let token = withdraw_token(creator, token_id, 2);
        initialize_token_store(owner);
        direct_deposit_with_opt_in(owner_addr, token);
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        opt_in_direct_transfer(owner, true);
        initialize_token_store(owner);
        transfer(creator, token_id, signer::address_of(owner), 2);
        burn_by_creator(creator, signer::address_of(owner), get_collection_name(), get_token_name(), 0, 1);
    }

    #[test(creator = @0xcafe, owner = @0x456)]
    #[expected_failure(abort_code = 327710, location = Self)]
    fun test_burn_token_by_owner_without_burnable_config(
        creator: &signer,
        owner: &signer,
    )acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

        opt_in_direct_transfer(owner, true);
        initialize_token_store(owner);
        transfer(creator, token_id, signer::address_of(owner), 2);

        burn(owner, signer::address_of(creator), get_collection_name(), get_token_name(), 0, 1);
    }

    #[test(creator = @0xcafe, owner = @0x456)]
    fun test_burn_token_by_owner_and_creator(
        creator: &signer,
        owner: &signer,
    ) acquires TokenStore, Collections {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[string::utf8(BURNABLE_BY_CREATOR), string::utf8(BURNABLE_BY_OWNER)],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true), bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool"), string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );
        opt_in_direct_transfer(owner, true);
        initialize_token_store(owner);
        transfer(creator, token_id, signer::address_of(owner), 2);
        burn_by_creator(creator, signer::address_of(owner), get_collection_name(), get_token_name(), 0, 1);
        burn(owner, signer::address_of(creator), get_collection_name(), get_token_name(), 0, 1);
        assert!(balance_of(signer::address_of(owner), token_id) == 0, 1);

        // The corresponding token_data and collection_data should be deleted
        let collections = &Collections[signer::address_of(creator)];
        assert!(!collections.collection_data.contains(token_id.token_data_id.name), 1);
        assert!(!collections.token_data.contains(token_id.token_data_id), 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_collection_description(
        creator: &signer,
    ) acquires Collections, TokenStore {
        let creator_address = signer::address_of(creator);
        account::create_account_for_test(creator_address);
        create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[true, false, false],
            vector<bool>[false, false, false, false, false],
        );

        let description = string::utf8(b"test");
        let collection_name = get_collection_name();
        mutate_collection_description(creator, collection_name, description);
        assert!(get_collection_description(creator_address, collection_name) == description, 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_collection_uri(
        creator: &signer,
    ) acquires Collections, TokenStore {
        let creator_address = signer::address_of(creator);
        account::create_account_for_test(creator_address);
        create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, true, false],
            vector<bool>[false, false, false, false, false],
        );

        let uri = string::utf8(b"");
        let collection_name = get_collection_name();
        mutate_collection_uri(creator, collection_name, uri);
        assert!(get_collection_uri(creator_address, collection_name) == uri, 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_collection_maximum(
        creator: &signer,
    ) acquires Collections, TokenStore {
        let creator_address = signer::address_of(creator);
        account::create_account_for_test(creator_address);
        create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, true],
            vector<bool>[false, false, false, false, false],
        );

        let collection_name = get_collection_name();
        mutate_collection_maximum(creator, collection_name, 10);
        assert!(get_collection_maximum(creator_address, collection_name) == 10, 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_default_token_properties(
        creator: &signer,
    ) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );
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

        mutate_tokendata_property(
            creator,
            token_id.token_data_id,
            new_keys,
            new_vals,
            new_types,
        );

        let all_token_data = &Collections[signer::address_of(creator)].token_data;
        assert!(all_token_data.contains(token_id.token_data_id), 1);
        let props = &all_token_data.borrow(token_id.token_data_id).default_properties;
        assert!(props.read_u64(&string::utf8(b"attack")) == 1, 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_tokendata_maximum(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[true, false, false, false, false],
        );
        mutate_tokendata_maximum(creator, token_id.token_data_id, 10);
        assert!(get_tokendata_maximum(token_id.token_data_id) == 10, 1);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 65572, location = Self)]
    fun test_mutate_tokendata_maximum_from_zero(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[true, false, false, false, false],
        );
        mutate_tokendata_maximum(creator, token_id.token_data_id, 0);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_tokendata_uri(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, true, false, false, false],
        );
        mutate_tokendata_uri(creator, token_id.token_data_id, string::utf8(b""));
        assert!(get_tokendata_uri(signer::address_of(creator), token_id.token_data_id) == string::utf8(b""), 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_tokendata_royalty(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, true, false, false],
        );

        let royalty = create_royalty(1, 3, signer::address_of(creator));
        mutate_tokendata_royalty(creator, token_id.token_data_id, royalty);
        assert!(get_tokendata_royalty(token_id.token_data_id) == royalty, 1);
    }

    #[test(creator = @0xcafe)]
    fun test_mutate_tokendata_description(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, true, false],
        );

        let description = string::utf8(b"test");
        mutate_tokendata_description(creator, token_id.token_data_id, description);
        assert!(get_tokendata_description(token_id.token_data_id) == description, 1);
    }

    #[test(creator = @0xAF, owner = @0xBB)]
    fun test_mutate_token_property(creator: &signer, owner: &signer) acquires Collections, TokenStore {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );
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
        let largest_property_version = get_tokendata_largest_property_version(signer::address_of(creator), token_id.token_data_id);
        assert!(largest_property_version == 2, largest_property_version);

        let new_id_1 = create_token_id(token_id.token_data_id, 1);
        let new_id_2 = create_token_id(token_id.token_data_id, 2);
        let new_id_3 = create_token_id(token_id.token_data_id, 3);

        assert!(balance_of(signer::address_of(creator), new_id_1) == 1, 1);
        assert!(balance_of(signer::address_of(creator), new_id_2) == 1, 1);
        assert!(balance_of(signer::address_of(creator), token_id) == 0, 1);

        let creator_props = &TokenStore[signer::address_of(creator)].tokens;
        let token = creator_props.borrow(new_id_1);

        assert!(token.token_properties.length() == 2, token.token_properties.length());
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
        direct_transfer(creator, owner, new_id_1, 1);

        let props = &TokenStore[signer::address_of(owner)].tokens;
        assert!(props.contains(new_id_1), 1);
        let token = props.borrow(new_id_1);
        assert!(token.token_properties.length() == 2, token.token_properties.length());
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 65569, location = Self)]
    fun test_no_zero_balance_token_deposit(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        create_collection_and_token(
            creator,
            0,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, true, false, false, false],
        );
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 65548, location = Self)]
    fun test_split_out_zero_token(
        creator: &signer,
    ) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));
        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            1,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, true, false, false, false],
        );
        let token = withdraw_token(creator, token_id, 1);
        let split_token = split(&mut token, 1);
        let Token {
            id: _,
            amount: _,
            token_properties: _,
        } = split_token;
        let Token {
            id: _,
            amount: _,
            token_properties: _,
        } = token;
    }

    #[test]
    #[expected_failure(abort_code = 65570, location = Self)]
    public fun test_enter_illegal_royalty(){
        create_royalty(101, 100, @0xcafe);
    }

    #[test(framework = @0x1, creator = @0xcafe)]
    fun test_partial_withdraw_with_proof(creator: &signer, framework: &signer): Token acquires TokenStore, Collections {
        timestamp::set_time_has_started_for_testing(framework);
        account::create_account_for_test(signer::address_of(creator));
        let token_id = create_collection_and_token(
            creator,
            4,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, false],
        );

        timestamp::update_global_time_for_test(1000000);

        // provide the proof to the account
        let cap = create_withdraw_capability(
            creator, // ask user to provide address to avoid ambiguity from rotated keys
            token_id,
            3,
            2000000,
        );

        let (token, capability) = partial_withdraw_with_capability(cap, 1);
        assert!(capability.borrow<WithdrawCapability>().amount == 2, 1);
        let (token_1, cap) = partial_withdraw_with_capability(capability.extract(), 2);
        assert!(cap.is_none(), 1);
        merge(&mut token, token_1);
        token
    }

    #[test(creator = @0xcafe)]
    fun test_get_collection_mutability_config(creator: &signer) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );

        get_collection_mutability_config(@0xcafe, get_collection_name());
    }

    #[test(creator = @0xcafe)]
    fun test_get_tokendata_mutability_config(creator: &signer) acquires Collections, TokenStore {
        account::create_account_for_test(signer::address_of(creator));

        // token owner mutate the token property
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[],
            vector<vector<u8>>[],
            vector<String>[],
            vector<bool>[false, false, false],
            vector<bool>[false, false, false, false, true],
        );

        get_tokendata_mutability_config(token_id.token_data_id);
    }

    #[test(creator = @0xcafe, owner = @0x456)]
    #[expected_failure(abort_code = 327720, location = Self)]
    fun test_fail_to_add_burn_flag(
        creator: &signer,
        owner: &signer,
    ) acquires TokenStore, Collections {
        use std::bcs;
        account::create_account_for_test(signer::address_of(creator));
        account::create_account_for_test(signer::address_of(owner));
        let token_id = create_collection_and_token(
            creator,
            2,
            4,
            4,
            vector<String>[string::utf8(BURNABLE_BY_OWNER)],
            vector<vector<u8>>[bcs::to_bytes<bool>(&true)],
            vector<String>[string::utf8(b"bool")],
            vector<bool>[false, false, false],
            vector<bool>[true, true, true, true, true],
        );
        let new_keys = vector<String>[
            string::utf8(BURNABLE_BY_CREATOR),
        ];
        let new_vals = vector<vector<u8>>[
            bcs::to_bytes<bool>(&true),
        ];
        let new_types = vector<String>[
            string::utf8(b"bool"),
        ];
        mutate_tokendata_property(
            creator,
            token_id.token_data_id,
            new_keys,
            new_vals,
            new_types,
        );
    }

    //
    // Deprecated functions
    //

    public entry fun initialize_token_script(_account: &signer) {
        abort 0
    }

    public fun initialize_token(_account: &signer, _token_id: TokenId) {
        abort 0
    }
}
