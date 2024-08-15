module unmanaged_launchpad::unmanaged_launchpad {
    use std::error;
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string;
    use std::string::String;
    use std::vector;

    use aptos_framework::object::{Self, Object};
    use aptos_framework::aptos_coin::AptosCoin;
    use aptos_framework::aggregator_v2::{Self, Aggregator};
    use aptos_framework::event;
    use aptos_framework::smart_vector::{Self, SmartVector};
    use aptos_framework::randomness;
    use aptos_framework::timestamp;

    use aptos_token_objects::collection;
    use aptos_token_objects::collection::Collection;
    use aptos_token_objects::royalty;
    use aptos_token_objects::royalty::Royalty;
    use aptos_token_objects::token;
    use aptos_token_objects::token::Token;

    use minter::coin_payment::{Self, CoinPayment};
    use minter::collection_components;
    use minter::collection_properties;
    use minter::token_components;
    use minter::transfer_token;

    /// The provided signer is not the collection owner during pre-minting.
    const ENOT_OWNER: u64 = 1;
    /// The provided collection does not have a UnmanagedLaunchpadConfig resource.
    /// Are you sure this Collection was created with unmanaged_launchpad?
    const EUNMANAGED_LAUNCHPAD_CONFIG_DOES_NOT_EXIST: u64 = 2;
    /// Token Minting has not yet started.
    const EMINTING_HAS_NOT_STARTED_YET: u64 = 3;
    /// No tokens available to mint.
    const ETOKENS_NOT_AVAILABLE: u64 = 4;

    #[event]
    struct CreateUnmanagedCollection has drop, store {
        collection: Object<Collection>,
        creator: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct UnmanagedLaunchpadConfig has key {
        extend_ref: object::ExtendRef, // for permissionless mint
        collection_created_timestamp: u64,
        coin_payments: vector<CoinPayment<AptosCoin>>,
        tokens: SmartVector<TokenVariance>,
        ready_to_mint: bool,
    }

    struct TokenVariance has store {
        name: String,
        description: String,
        uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        minted_token_count: Aggregator<u64>, // using aggregator for more parallel count addition performance
        supply: Option<u64>, // total supply of token variance, if null supply is unlimited
    }

    // ================================= Entry Functions ================================= //

    public entry fun create_collection(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
        mint_fee: Option<u64>,
        royalty_numerator: Option<u64>,
        royalty_denominator: Option<u64>,
    ) acquires UnmanagedLaunchpadConfig {
        create_collection_impl(
            creator,
            description,
            name,
            uri,
            mint_fee,
            royalty_numerator,
            royalty_denominator,
        );
    }

    public entry fun pre_mint_tokens(
        creator: &signer,
        collection: Object<Collection>,
        name: String,
        uri: String,
        description: String,
        num_token: Option<u64>,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
    ) acquires UnmanagedLaunchpadConfig {
        pre_mint_tokens_impl(
            creator,
            collection,
            name,
            uri,
            description,
            num_token,
            property_keys,
            property_types,
            property_values,
        )
    }

    #[randomness]
    entry fun mint(
        user: &signer,
        collection: Object<Collection>,
    ) acquires UnmanagedLaunchpadConfig {
        mint_impl(user, collection);
    }

    public entry fun set_minting_status(creator: &signer, collection: Object<Collection>, ready_to_mint: bool) acquires UnmanagedLaunchpadConfig {
        let unmanaged_launchpad_config = authorized_borrow_mut(creator, collection);

        unmanaged_launchpad_config.ready_to_mint = ready_to_mint;
    }

    public entry fun add_mint_fee(creator: &signer, collection: Object<Collection>, mint_fee: u64, mint_fee_category: String, destination: address) acquires UnmanagedLaunchpadConfig {
        let config = authorized_borrow_mut(creator, collection);
        let fee = coin_payment::create<AptosCoin>(mint_fee, destination, mint_fee_category);
        vector::push_back(&mut config.coin_payments, fee);
    }

    // ================================= Helper  ================================= //

    public fun create_collection_impl(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
        mint_fee: Option<u64>,
        royalty_numerator: Option<u64>,
        royalty_denominator: Option<u64>,
    ): Object<Collection> acquires UnmanagedLaunchpadConfig {
        let creator_addr = signer::address_of(creator);
        let object_constructor_ref = &object::create_object(creator_addr);
        let obj_signer = object::generate_signer(object_constructor_ref);

        let royalty = royalty(&mut royalty_numerator, &mut royalty_denominator, creator_addr);

        let collection_constructor_ref =
            collection::create_unlimited_collection(
                &obj_signer,
                description,
                name,
                royalty,
                uri,
            );
        collection_components::create_refs_and_properties(&collection_constructor_ref);
        let collection = object::object_from_constructor_ref(&collection_constructor_ref);
        configure_collection_and_token_properties(
            &obj_signer,
            collection,
            true, // mutable_collection_metadata
            true, // mutable_token_metadata
            true, // tokens_burnable_by_collection_owner,
            true, // tokens_transferrable_by_collection_owner,
        );

        event::emit(CreateUnmanagedCollection { collection, creator: signer::address_of(creator) });

        move_to(&obj_signer, UnmanagedLaunchpadConfig {
            extend_ref: object::generate_extend_ref(object_constructor_ref),
            tokens: smart_vector::new(), // todo (jill) make bucket_size 10?
            collection_created_timestamp: timestamp::now_seconds(),
            coin_payments: vector[],
            ready_to_mint: false,
        });

        let obj_addr = signer::address_of(&obj_signer);
        let config = borrow_global_mut<UnmanagedLaunchpadConfig>(obj_addr);
        add_mint_fee_internal(&mut mint_fee, creator_addr, config);

        collection
    }

    public fun pre_mint_tokens_impl(
        creator: &signer,
        collection: Object<Collection>,
        name: String,
        uri: String,
        description: String,
        num_token: Option<u64>,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
    ) acquires UnmanagedLaunchpadConfig {
        // not check against the ready_to_mint so that we enable creators to pre_mint anytime
        // even when minting has already started

        let config = authorized_borrow_mut(creator, collection);
        let minted_token_count = if (option::is_some(&num_token)) {
            aggregator_v2::create_aggregator_with_value(0, option::extract(&mut num_token))
        } else {
            aggregator_v2::create_unbounded_aggregator_with_value(0)
        };
        let token_variance = TokenVariance {
            name,
            description,
            uri,
            property_keys,
            property_types,
            property_values,
            minted_token_count,
            supply: num_token, // if null supply is unlimited
        };
        smart_vector::push_back(&mut config.tokens, token_variance);
    }

    #[lint::allow_unsafe_randomness]
    public fun mint_impl(
        user: &signer,
        collection: Object<Collection>,
    ): Object<Token> acquires UnmanagedLaunchpadConfig {
        config_addr(collection);

        let config = borrow_mut(collection);
        assert!(config.ready_to_mint, error::permission_denied(EMINTING_HAS_NOT_STARTED_YET));

        let collection_signer = object::generate_signer_for_extending(&config.extend_ref);

        let token_variances = &mut config.tokens;

        let token_variance = get_random_token(token_variances);

        let description = token_variance.description;
        let name = token_variance.name;
        let uri = token_variance.uri;
        let property_keys = token_variance.property_keys;
        let property_types = token_variance.property_types;
        let property_values = token_variance.property_values;

        execute_coin_payments(user, collection);

        mint_token_impl(&collection_signer, user, collection, description, name, uri, property_keys, property_types, property_values)
    }


    public fun mint_token_impl(
        creator: &signer,
        user: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
    ): Object<Token> {
        let constructor_ref = &token::create_token(
            creator,
            collection,
            description,
            name,
            royalty::get(collection),
            uri,
        );

        token_components::create_refs(constructor_ref);

        let token = object::object_from_constructor_ref(constructor_ref);

        for (i in 0..vector::length(&property_keys)) {
            let key = *vector::borrow(&property_keys, i);
            let type = *vector::borrow(&property_types, i);
            let value = *vector::borrow(&property_values, i);
            token_components::add_property(creator, token, key, type, value);
        };

        transfer_token::transfer(creator, signer::address_of(user), constructor_ref);

        object::object_from_constructor_ref(constructor_ref)
    }

    fun get_random_token(token_variances: &mut SmartVector<TokenVariance>): &mut TokenVariance {
        let total_variances = smart_vector::length(token_variances);
        let multiplier = 3; // Adjust this multiplier as needed
        let tries = total_variances * multiplier;

        while (tries > 0) {
            let random_index = randomness::u64_integer() % total_variances;
            let token_variance = smart_vector::borrow_mut(token_variances, random_index);
            let success = aggregator_v2::try_add(&mut token_variance.minted_token_count, 1);
            if (!success) {
                tries = tries - 1;
            } else {
                return token_variance
            }
        };

        // abort if tries exhausted
        abort(error::aborted(ETOKENS_NOT_AVAILABLE))
    }

    fun assert_owner<T: key>(owner: address, object: Object<T>) {
        // creator is not the direct owner of collection or config, but root owner
        assert!(object::root_owner(object) == owner, error::permission_denied(ENOT_OWNER));
    }

    fun add_mint_fee_internal(
        mint_fee: &mut Option<u64>,
        creator_addr: address,
        config: &mut UnmanagedLaunchpadConfig,
    ) {
        if (option::is_some(mint_fee)) {
            let mint_fee_category = b"Mint Fee";
            let coin_payment = coin_payment::create<AptosCoin>(
                option::extract(mint_fee),
                creator_addr,
                string::utf8(mint_fee_category),
            );
            vector::push_back(&mut config.coin_payments, coin_payment);
        };
    }

    fun execute_coin_payments(
        user: &signer,
        collection: Object<Collection>,
    ) acquires UnmanagedLaunchpadConfig {
        let unmanaged_launchpad_config = borrow_mut(collection);
        vector::for_each_ref(&unmanaged_launchpad_config.coin_payments, |coin_payment| {
            let coin_payment: &CoinPayment<AptosCoin> = coin_payment;
            coin_payment::execute(user, coin_payment);
        });
    }

    fun royalty(
        royalty_numerator: &mut Option<u64>,
        royalty_denominator: &mut Option<u64>,
        creator_addr: address,
    ): Option<Royalty> {
        if (option::is_some(royalty_numerator) && option::is_some(royalty_denominator)) {
            let num = option::extract(royalty_numerator);
            let den = option::extract(royalty_denominator);
            if (num != 0 && den != 0) {
                option::some(royalty::create(num, den, creator_addr));
            };
        };
        option::none()
    }

    fun configure_collection_and_token_properties(
        obj_signer: &signer,
        collection: Object<Collection>,
        mutable_collection_metadata: bool,
        mutable_token_metadata: bool,
        tokens_burnable_by_collection_owner: bool,
        tokens_transferrable_by_collection_owner: bool,
    ) {
        collection_properties::set_mutable_description(obj_signer, collection, mutable_collection_metadata);
        collection_properties::set_mutable_uri(obj_signer, collection, mutable_collection_metadata);
        collection_properties::set_mutable_royalty(obj_signer, collection, mutable_collection_metadata);
        collection_properties::set_mutable_token_name(obj_signer, collection, mutable_token_metadata);
        collection_properties::set_mutable_token_properties(obj_signer, collection, mutable_token_metadata);
        collection_properties::set_mutable_token_description(obj_signer, collection, mutable_token_metadata);
        collection_properties::set_mutable_token_uri(obj_signer, collection, mutable_token_metadata);
        collection_properties::set_tokens_transferable_by_collection_owner(obj_signer, collection, tokens_transferrable_by_collection_owner);
        collection_properties::set_tokens_burnable_by_collection_owner(obj_signer, collection, tokens_burnable_by_collection_owner);
    }

    inline fun borrow(collection: Object<Collection>): &UnmanagedLaunchpadConfig {
        freeze(borrow_mut(collection))
    }

    inline fun borrow_mut(collection: Object<Collection>): &mut UnmanagedLaunchpadConfig acquires UnmanagedLaunchpadConfig {
        let obj_addr = config_addr(collection);
        borrow_global_mut<UnmanagedLaunchpadConfig>(obj_addr)
    }

    inline fun config_addr(collection: Object<Collection>): address {
        let obj_addr = object::owner(collection);
        assert!(exists<UnmanagedLaunchpadConfig>(obj_addr), error::not_found(EUNMANAGED_LAUNCHPAD_CONFIG_DOES_NOT_EXIST));
        obj_addr
    }

    inline fun authorized_borrow_mut(creator: &signer, collection: Object<Collection>): &mut UnmanagedLaunchpadConfig acquires UnmanagedLaunchpadConfig {
        assert_owner(signer::address_of(creator), collection);
        borrow_mut(collection)
    }

    // ================================= View  ================================= //

    #[view]
    public fun collection_created_timestamp(collection: Object<Collection>): u64 acquires UnmanagedLaunchpadConfig {
        borrow(collection).collection_created_timestamp // utc
    }

    #[view]
    public fun mint_fee(collection: Object<Collection>): Option<u64> acquires UnmanagedLaunchpadConfig {
        let config = borrow_mut(collection);
        if (vector::is_empty(&config.coin_payments)) {
            option::none<u64>()
        } else {
            let payment = vector::borrow(&config.coin_payments, 0);
            option::some(coin_payment::amount(payment))
        }
    }

    #[view]
    public fun ready_to_mint(collection: Object<Collection>): bool acquires UnmanagedLaunchpadConfig {
        borrow(collection).ready_to_mint
    }

    #[view]
    public fun collection_owner(collection: Object<Collection>): address {
        object::owner(collection)
    }

    struct TokenVarianceView has copy, drop {
        name: String,
        description: String,
        uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        minted_token_count: u64,
        supply: Option<u64>,
    }

    #[view]
    /// Helper function to get all the token variances of a collection - useful for display.
    public fun tokens(collection: Object<Collection>): vector<TokenVarianceView> acquires UnmanagedLaunchpadConfig {
        let token_variances = &borrow(collection).tokens;
        let token_variances_view = vector[];
        for(i in 0..smart_vector::length(token_variances)) {
            let token_variance = smart_vector::borrow(token_variances, i);
            let token_variance_view = TokenVarianceView {
                name: token_variance.name,
                description: token_variance.description,
                uri: token_variance.uri,
                property_keys: token_variance.property_keys,
                property_types: token_variance.property_types,
                property_values: token_variance.property_values,
                minted_token_count: aggregator_v2::read(&token_variance.minted_token_count),
                supply: token_variance.supply,
            };
            vector::push_back(&mut token_variances_view, token_variance_view);
        };
        token_variances_view
    }
}
