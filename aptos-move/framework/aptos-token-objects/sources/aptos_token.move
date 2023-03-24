/// This defines a minimally viable token for no-code solutions akin the the original token at
/// 0x3::token module.
/// The key features are:
/// * Base token and collection features
/// * Creator definable mutability for tokens
/// * Creator-based freezing of tokens
/// * Standard object-based transfer and events
/// * Metadata property type
module aptos_token_objects::aptos_token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::String;
    use std::signer;

    use aptos_framework::object::{Self, ConstructorRef, Object};

    use aptos_token_objects::collection;
    use aptos_token_objects::property_map;
    use aptos_token_objects::royalty;
    use aptos_token_objects::token;

    // The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 1;
    /// The provided signer is not the creator
    const ENOT_CREATOR: u64 = 2;
    /// Attempted to mutate an immutable field
    const EFIELD_NOT_MUTABLE: u64 = 3;
    /// Attempted to burn a non-burnable token
    const ETOKEN_NOT_BURNABLE: u64 = 4;
    /// Attempted to mutate a property map that is not mutable
    const EPROPERTIES_NOT_MUTABLE: u64 = 5;
    // The collection does not exist
    const ECOLLECTION_DOES_NOT_EXIST: u64 = 1;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Storage state for managing the no-code Collection.
    struct AptosCollection has key {
        /// Used to mutate collection fields
        mutator_ref: Option<collection::MutatorRef>,
        /// Used to mutate royalties
        royalty_mutator_ref: Option<royalty::MutatorRef>,
        /// Determines if the creator can mutate the collection's description
        mutable_description: bool,
        /// Determines if the creator can mutate the collection's uri
        mutable_uri: bool,
        /// Determines if the creator can mutate token descriptions
        mutable_token_description: bool,
        /// Determines if the creator can mutate token names
        mutable_token_name: bool,
        /// Determines if the creator can mutate token properties
        mutable_token_properties: bool,
        /// Determines if the creator can mutate token uris
        mutable_token_uri: bool,
        /// Determines if the creator can burn tokens
        tokens_burnable_by_creator: bool,
        /// Determines if the creator can freeze tokens
        tokens_freezable_by_creator: bool,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Storage state for managing the no-code Token.
    struct AptosToken has key {
        /// Used to burn.
        burn_ref: Option<token::BurnRef>,
        /// Used to control freeze.
        transfer_ref: Option<object::TransferRef>,
        /// Used to mutate fields
        mutator_ref: Option<token::MutatorRef>,
        /// Used to mutate properties
        property_mutator_ref: property_map::MutatorRef,
    }

    /// Create a new collection
    public entry fun create_collection(
        creator: &signer,
        description: String,
        max_supply: u64,
        name: String,
        uri: String,
        mutable_description: bool,
        mutable_royalty: bool,
        mutable_uri: bool,
        mutable_token_description: bool,
        mutable_token_name: bool,
        mutable_token_properties: bool,
        mutable_token_uri: bool,
        tokens_burnable_by_creator: bool,
        tokens_freezable_by_creator: bool,
        royalty_numerator: u64,
        royalty_denominator: u64,
    ) {
        let creator_addr = signer::address_of(creator);
        let royalty = royalty::create(royalty_numerator, royalty_denominator, creator_addr);
        let constructor_ref = collection::create_fixed_collection(
            creator,
            description,
            max_supply,
            name,
            option::some(royalty),
            uri,
        );

        let object_signer = object::generate_signer(&constructor_ref);
        let mutator_ref = if (mutable_description || mutable_uri) {
            option::some(collection::generate_mutator_ref(&constructor_ref))
        } else {
            option::none()
        };

        let royalty_mutator_ref = if (mutable_royalty) {
            option::some(royalty::generate_mutator_ref(object::generate_extend_ref(&constructor_ref)))
        } else {
            option::none()
        };

        let aptos_collection = AptosCollection {
            mutator_ref,
            royalty_mutator_ref,
            mutable_description,
            mutable_uri,
            mutable_token_description,
            mutable_token_name,
            mutable_token_properties,
            mutable_token_uri,
            tokens_burnable_by_creator,
            tokens_freezable_by_creator,
        };
        move_to(&object_signer, aptos_collection);
    }

    /// With an existing collection, directly mint a viable token into the creators account.
    public entry fun mint(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
    ) acquires AptosCollection, AptosToken {
        let constructor_ref = mint_internal(
            creator,
            collection,
            description,
            name,
            uri,
            property_keys,
            property_types,
            property_values,
        );

        let collection = collection_object(creator, &collection);
        let freezable_by_creator = are_collection_tokens_freezable(collection);
        if (!freezable_by_creator) {
            return
        };

        let aptos_token_addr = object::address_from_constructor_ref(&constructor_ref);
        let aptos_token = borrow_global_mut<AptosToken>(aptos_token_addr);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        option::fill(&mut aptos_token.transfer_ref, transfer_ref);
    }

    /// With an existing collection, directly mint a soul bound token into the recipient's account.
    public entry fun mint_soul_bound(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        soul_bound_to: address,
    ) acquires AptosCollection {
        let constructor_ref = mint_internal(
            creator,
            collection,
            description,
            name,
            uri,
            property_keys,
            property_types,
            property_values,
        );

        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, soul_bound_to);
        object::disable_ungated_transfer(&transfer_ref);
    }

    fun mint_internal(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        uri: String,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
    ): ConstructorRef acquires AptosCollection {
        let constructor_ref = token::create(
            creator,
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        let object_signer = object::generate_signer(&constructor_ref);

        let collection_obj = collection_object(creator, &collection);
        let collection = borrow_collection(&collection_obj);

        let mutator_ref = if (
            collection.mutable_token_description
            || collection.mutable_token_name
            || collection.mutable_token_uri
        ) {
            option::some(token::generate_mutator_ref(&constructor_ref))
        } else {
            option::none()
        };

        let burn_ref = if (collection.tokens_burnable_by_creator) {
            option::some(token::generate_burn_ref(&constructor_ref))
        } else {
            option::none()
        };

        let aptos_token = AptosToken {
            burn_ref,
            transfer_ref: option::none(),
            mutator_ref,
            property_mutator_ref: property_map::generate_mutator_ref(&constructor_ref),
        };
        move_to(&object_signer, aptos_token);

        let properties = property_map::prepare_input(property_keys, property_types, property_values);
        property_map::init(&constructor_ref, properties);

        constructor_ref
    }

    // Token accessors

    inline fun borrow<T: key>(token: &Object<T>): &AptosToken {
        let token_address = object::object_address(token);
        assert!(
            exists<AptosToken>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<AptosToken>(token_address)
    }

    public fun are_properties_mutable<T: key>(token: Object<T>): bool acquires AptosCollection {
        let collection = token::collection_object(token);
        borrow_collection(&collection).mutable_token_properties
    }

    public fun is_burnable<T: key>(token: Object<T>): bool acquires AptosToken {
        option::is_some(&borrow(&token).burn_ref)
    }

    public fun is_freezable_by_creator<T: key>(token: Object<T>): bool acquires AptosCollection {
        are_collection_tokens_freezable(token::collection_object(token))
    }

    public fun is_mutable_description<T: key>(token: Object<T>): bool acquires AptosCollection {
        is_mutable_collection_token_description(token::collection_object(token))
    }

    public fun is_mutable_name<T: key>(token: Object<T>): bool acquires AptosCollection {
        is_mutable_collection_token_name(token::collection_object(token))
    }

    public fun is_mutable_uri<T: key>(token: Object<T>): bool acquires AptosCollection {
        is_mutable_collection_token_uri(token::collection_object(token))
    }

    // Token mutators

    inline fun authorized_borrow<T: key>(token: &Object<T>, creator: address): &AptosToken {
        let token_address = object::object_address(token);
        assert!(
            exists<AptosToken>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );

        assert!(
            token::creator(*token) == creator,
            error::permission_denied(ENOT_CREATOR),
        );
        borrow_global<AptosToken>(token_address)
    }

    public fun burn<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_token.burn_ref),
            error::permission_denied(ETOKEN_NOT_BURNABLE),
        );
        move aptos_token;
        let aptos_token = move_from<AptosToken>(object::object_address(&token));
        let AptosToken {
            burn_ref,
            transfer_ref: _,
            mutator_ref: _,
            property_mutator_ref,
        } = aptos_token;
        property_map::burn(property_mutator_ref);
        token::burn(option::extract(&mut burn_ref));
    }

    public fun freeze_transfer<T: key>(creator: &signer, token: Object<T>) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_collection_tokens_freezable(token::collection_object(token))
                && option::is_some(&aptos_token.transfer_ref),
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        object::disable_ungated_transfer(option::borrow(&aptos_token.transfer_ref));
    }

    public fun unfreeze_transfer<T: key>(creator: &signer, token: Object<T>) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_collection_tokens_freezable(token::collection_object(token))
                && option::is_some(&aptos_token.transfer_ref),
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        object::enable_ungated_transfer(option::borrow(&aptos_token.transfer_ref));
    }

    public fun set_description<T: key>(
        creator: &signer,
        token: Object<T>,
        description: String,
    ) acquires AptosCollection, AptosToken {
        assert!(
            is_mutable_description(token),
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        token::set_description(option::borrow(&aptos_token.mutator_ref), description);
    }

    public fun set_name<T: key>(
        creator: &signer,
        token: Object<T>,
        name: String,
    ) acquires AptosCollection, AptosToken {
        assert!(
            is_mutable_name(token),
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        token::set_name(option::borrow(&aptos_token.mutator_ref), name);
    }

    public fun set_uri<T: key>(
        creator: &signer,
        token: Object<T>,
        uri: String,
    ) acquires AptosCollection, AptosToken {
        assert!(
            is_mutable_uri(token),
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        token::set_uri(option::borrow(&aptos_token.mutator_ref), uri);
    }

    public fun add_property<T: key>(
        creator: &signer,
        token: Object<T>,
        key: String,
        type: String,
        value: vector<u8>,
    ) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_properties_mutable(token),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::add(&aptos_token.property_mutator_ref, key, type, value);
    }

    public fun add_typed_property<T: key, V: drop>(
        creator: &signer,
        token: Object<T>,
        key: String,
        value: V,
    ) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_properties_mutable(token),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::add_typed(&aptos_token.property_mutator_ref, key, value);
    }

    public fun remove_property<T: key>(
        creator: &signer,
        token: Object<T>,
        key: &String,
    ) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_properties_mutable(token),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::remove(&aptos_token.property_mutator_ref, key);
    }

    public fun update_property<T: key>(
        creator: &signer,
        token: Object<T>,
        key: &String,
        type: String,
        value: vector<u8>,
    ) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_properties_mutable(token),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::update(&aptos_token.property_mutator_ref, key, type, value);
    }

    public fun update_typed_property<T: key, V: drop>(
        creator: &signer,
        token: Object<T>,
        key: &String,
        value: V,
    ) acquires AptosCollection, AptosToken {
        let aptos_token = authorized_borrow(&token, signer::address_of(creator));
        assert!(
            are_properties_mutable(token),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::update_typed(&aptos_token.property_mutator_ref, key, value);
    }

    // Token entry functions

    inline fun token_object(creator: &signer, collection: &String, name: &String): Object<AptosToken> {
        let token_addr = token::create_token_address(&signer::address_of(creator), collection, name);
        object::address_to_object<AptosToken>(token_addr)
    }

    entry fun burn_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosToken {
        burn(creator, token_object(creator, &collection, &name));
    }

    entry fun freeze_transfer_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosCollection, AptosToken {
        freeze_transfer(creator, token_object(creator, &collection, &name));
    }

    entry fun unfreeze_transfer_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosCollection, AptosToken {
        unfreeze_transfer(creator, token_object(creator, &collection, &name));
    }

    entry fun set_description_call(
        creator: &signer,
        collection: String,
        name: String,
        description: String,
    ) acquires AptosCollection, AptosToken {
        set_description(creator, token_object(creator, &collection, &name), description);
    }

    entry fun set_name_call(
        creator: &signer,
        collection: String,
        original_name: String,
        new_name: String,
    ) acquires AptosCollection, AptosToken {
        set_name(creator, token_object(creator, &collection, &original_name), new_name);
    }

    entry fun set_uri_call(
        creator: &signer,
        collection: String,
        name: String,
        uri: String,
    ) acquires AptosCollection, AptosToken {
        set_uri(creator, token_object(creator, &collection, &name), uri);
    }

    entry fun add_property_call(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        type_: String,
        value: vector<u8>,
    ) acquires AptosCollection, AptosToken {
        let token = token_object(creator, &collection, &name);
        add_property(creator, token, key, type_, value);
    }

    entry fun add_typed_property_call<T: drop>(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        value: T,
    ) acquires AptosCollection, AptosToken {
        let token = token_object(creator, &collection, &name);
        add_typed_property(creator, token, key, value);
    }

    entry fun remove_property_call(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
    ) acquires AptosCollection, AptosToken {
        let token = token_object(creator, &collection, &name);
        remove_property(creator, token, &key);
    }

    entry fun update_property_call(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        type_: String,
        value: vector<u8>,
    ) acquires AptosCollection, AptosToken {
        let token = token_object(creator, &collection, &name);
        update_property(creator, token, &key, type_, value);
    }

    entry fun update_typed_property_call<T: drop>(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        value: T,
    ) acquires AptosCollection, AptosToken {
        let token = token_object(creator, &collection, &name);
        update_typed_property(creator, token, &key, value);
    }

    // Collection accessors

    inline fun borrow_collection<T: key>(token: &Object<T>): &AptosCollection {
        let collection_address = object::object_address(token);
        assert!(
            exists<AptosCollection>(collection_address),
            error::not_found(ECOLLECTION_DOES_NOT_EXIST),
        );
        borrow_global<AptosCollection>(collection_address)
    }

    public fun is_mutable_collection_description<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_description
    }

    public fun is_mutable_collection_royalty<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        option::is_some(&borrow_collection(&collection).royalty_mutator_ref)
    }

    public fun is_mutable_collection_uri<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_uri
    }

    public fun is_mutable_collection_token_description<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_token_description
    }

    public fun is_mutable_collection_token_name<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_token_name
    }

    public fun is_mutable_collection_token_uri<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_token_uri
    }

    public fun is_mutable_collection_token_properties<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_token_properties
    }

    public fun are_collection_tokens_burnable<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).tokens_burnable_by_creator
    }

    public fun are_collection_tokens_freezable<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).tokens_freezable_by_creator
    }

    // Collection mutators

    inline fun authorized_borrow_collection<T: key>(collection: &Object<T>, creator: address): &AptosCollection {
        let collection_address = object::object_address(collection);
        assert!(
            exists<AptosCollection>(collection_address),
            error::not_found(ECOLLECTION_DOES_NOT_EXIST),
        );
        assert!(
            collection::creator(*collection) == creator,
            error::permission_denied(ENOT_CREATOR),
        );
        borrow_global<AptosCollection>(collection_address)
    }

    public fun set_collection_description<T: key>(
        creator: &signer,
        collection: Object<T>,
        description: String,
    ) acquires AptosCollection {
        let aptos_collection = authorized_borrow_collection(&collection, signer::address_of(creator));
        assert!(
            aptos_collection.mutable_description,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        collection::set_description(option::borrow(&aptos_collection.mutator_ref), description);
    }

    public fun set_collection_royalties<T: key>(
        creator: &signer,
        collection: Object<T>,
        royalty: royalty::Royalty,
    ) acquires AptosCollection {
        let aptos_collection = authorized_borrow_collection(&collection, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_collection.royalty_mutator_ref),
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        royalty::update(option::borrow(&aptos_collection.royalty_mutator_ref), royalty);
    }

    public fun set_collection_uri<T: key>(
        creator: &signer,
        collection: Object<T>,
        uri: String,
    ) acquires AptosCollection {
        let aptos_collection = authorized_borrow_collection(&collection, signer::address_of(creator));
        assert!(
            aptos_collection.mutable_uri,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        collection::set_uri(option::borrow(&aptos_collection.mutator_ref), uri);
    }

    // Collection entry functions

    inline fun collection_object(creator: &signer, name: &String): Object<AptosCollection> {
        let collection_addr = collection::create_collection_address(&signer::address_of(creator), name);
        object::address_to_object<AptosCollection>(collection_addr)
    }

    entry fun set_collection_description_call(
        creator: &signer,
        collection: String,
        description: String,
    ) acquires AptosCollection {
        set_collection_description(creator, collection_object(creator, &collection), description);
    }

    entry fun set_collection_royalties_call(
        creator: &signer,
        collection: String,
        royalty_numerator: u64,
        royalty_denominator: u64,
        payee_address: address,
    ) acquires AptosCollection {
        let royalty = royalty::create(royalty_numerator, royalty_denominator, payee_address);
        set_collection_royalties(creator, collection_object(creator, &collection), royalty);
    }

    entry fun set_collection_uri_call(
        creator: &signer,
        collection: String,
        uri: String,
    ) acquires AptosCollection {
        set_collection_uri(creator, collection_object(creator, &collection), uri);
    }

    // Tests

    #[test_only]
    use std::string;

    #[test(creator = @0x123)]
    fun test_create_and_transfer(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        assert!(object::owner(token) == signer::address_of(creator), 1);
        object::transfer(creator, token, @0x345);
        assert!(object::owner(token) == @0x345, 1);
    }

    #[test(creator = @0x123, bob = @0x456)]
    #[expected_failure(abort_code = 0x50003, location = object)]
    fun test_mint_soul_bound(creator: &signer, bob: &signer) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_soul_bound(
            creator,
            collection_name,
            string::utf8(b""),
            token_name,
            string::utf8(b""),
            vector[],
            vector[],
            vector[],
            signer::address_of(bob),
        );

        let token_addr = token::create_token_address(
            &signer::address_of(creator),
            &collection_name,
            &token_name,
        );
        let token = object::address_to_object<AptosToken>(token_addr);
        object::transfer(bob, token, @0x345);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = object)]
    fun test_frozen_transfer(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        freeze_transfer_call(creator, collection_name, token_name);
        object::transfer(creator, token, @0x345);
    }

    #[test(creator = @0x123)]
    fun test_unfrozen_transfer(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        freeze_transfer_call(creator, collection_name, token_name);
        unfreeze_transfer_call(creator, collection_name, token_name);
        object::transfer(creator, token, @0x345);
    }

    #[test(creator = @0x123, another = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_noncreator_freeze(creator: &signer, another: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        freeze_transfer(another, token);
    }

    #[test(creator = @0x123, another = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_noncreator_unfreeze(creator: &signer, another: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        freeze_transfer_call(creator, collection_name, token_name);
        unfreeze_transfer(another, token);
    }

    #[test(creator = @0x123)]
    fun test_set_description(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let description = string::utf8(b"not");
        assert!(token::description(token) != description, 0);
        set_description_call(creator, collection_name, token_name, description);
        assert!(token::description(token) == description, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_description(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_helper(creator, collection_name, token_name);

        set_description_call(creator, collection_name, token_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_description_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let description = string::utf8(b"not");
        set_description(noncreator, token, description);
    }

    #[test(creator = @0x123)]
    fun test_set_name(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let name = string::utf8(b"not");
        assert!(token::name(token) != name, 0);
        set_name_call(creator, collection_name, token_name, name);
        assert!(token::name(token) == name, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_name(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_helper(creator, collection_name, token_name);

        set_name_call(creator, collection_name, token_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_name_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let name = string::utf8(b"not");
        set_name(noncreator, token, name);
    }

    #[test(creator = @0x123)]
    fun test_set_uri(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let uri = string::utf8(b"not");
        assert!(token::uri(token) != uri, 0);
        set_uri_call(creator, collection_name, token_name, uri);
        assert!(token::uri(token) == uri, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_uri(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_helper(creator, collection_name, token_name);

        set_uri_call(creator, collection_name, token_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_uri_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let uri = string::utf8(b"not");
        set_uri(noncreator, token, uri);
    }

    #[test(creator = @0x123)]
    entry fun test_burnable(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let token_addr = token::create_token_address(&signer::address_of(creator), &collection_name, &token_name);
        assert!(exists<AptosToken>(token_addr), 0);
        burn(creator, token);
        assert!(!exists<AptosToken>(token_addr), 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    entry fun test_not_burnable(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name);

        burn(creator, token);
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    entry fun test_burn_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        burn(noncreator, token);
    }

    #[test(creator = @0x123)]
    fun test_set_collection_description(creator: &signer) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        let collection = create_collection_helper(creator, collection_name, true);
        let value = string::utf8(b"not");
        assert!(collection::description(collection) != value, 0);
        set_collection_description_call(creator, collection_name, value);
        assert!(collection::description(collection) == value, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_collection_description(creator: &signer) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        create_collection_helper(creator, collection_name, false);
        set_collection_description_call(creator, collection_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_collection_description_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        let collection = create_collection_helper(creator, collection_name, true);
        set_collection_description(noncreator, collection, string::utf8(b""));
    }

    #[test(creator = @0x123)]
    fun test_set_collection_uri(creator: &signer) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        let collection = create_collection_helper(creator, collection_name, true);
        let value = string::utf8(b"not");
        assert!(collection::uri(collection) != value, 0);
        set_collection_uri_call(creator, collection_name, value);
        assert!(collection::uri(collection) == value, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_collection_uri(creator: &signer) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        create_collection_helper(creator, collection_name, false);
        set_collection_uri_call(creator, collection_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_collection_uri_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosCollection {
        let collection_name = string::utf8(b"collection name");
        let collection = create_collection_helper(creator, collection_name, true);
        set_collection_uri(noncreator, collection, string::utf8(b""));
    }

    #[test(creator = @0x123)]
    entry fun test_property_add(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");
        let property_name = string::utf8(b"u8");
        let property_type = string::utf8(b"u8");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        add_property_call(creator, collection_name, token_name, property_name, property_type, vector [ 0x08 ]);

        assert!(property_map::read_u8(&token, &property_name) == 0x8, 0);
    }

    #[test(creator = @0x123)]
    entry fun test_property_typed_add(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");
        let property_name = string::utf8(b"u8");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        add_typed_property_call<u8>(creator, collection_name, token_name, property_name, 0x8);

        assert!(property_map::read_u8(&token, &property_name) == 0x8, 0);
    }

    #[test(creator = @0x123)]
    entry fun test_property_update(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");
        let property_name = string::utf8(b"bool");
        let property_type = string::utf8(b"bool");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        update_property_call(creator, collection_name, token_name, property_name, property_type, vector [ 0x00 ]);

        assert!(!property_map::read_bool(&token, &property_name), 0);
    }

    #[test(creator = @0x123)]
    entry fun test_property_update_typed(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");
        let property_name = string::utf8(b"bool");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);
        update_typed_property_call<bool>(creator, collection_name, token_name, property_name, false);

        assert!(!property_map::read_bool(&token, &property_name), 0);
    }

    #[test(creator = @0x123)]
    entry fun test_property_remove(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");
        let property_name = string::utf8(b"bool");

        create_collection_helper(creator, collection_name, true);
        mint_helper(creator, collection_name, token_name);
        remove_property_call(creator, collection_name, token_name, property_name);
    }

    #[test(creator = @0x123)]
    entry fun test_royalties(creator: &signer) acquires AptosCollection, AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, true);
        let token = mint_helper(creator, collection_name, token_name);

        let royalty_before = option::extract(&mut token::royalty(token));
        set_collection_royalties_call(creator, collection_name, 2, 3, @0x444);
        let royalty_after = option::extract(&mut token::royalty(token));
        assert!(royalty_before != royalty_after, 0);
    }

    #[test_only]
    fun create_collection_helper(
        creator: &signer,
        collection_name: String,
        flag: bool,
    ): Object<AptosCollection> {
        create_collection(
            creator,
            string::utf8(b"collection description"),
            1,
            collection_name,
            string::utf8(b"collection uri"),
            flag,
            flag,
            flag,
            flag,
            flag,
            flag,
            flag,
            flag,
            flag,
            1,
            100,
        );

        collection_object(creator, &collection_name)
    }

    #[test_only]
    fun mint_helper(
        creator: &signer,
        collection_name: String,
        token_name: String,
    ): Object<AptosToken> acquires AptosCollection, AptosToken {
        mint(
            creator,
            collection_name,
            string::utf8(b"description"),
            token_name,
            string::utf8(b"uri"),
            vector[string::utf8(b"bool")],
            vector[string::utf8(b"bool")],
            vector[vector[0x01]],
        );

        token_object(creator, &collection_name, &token_name)
    }
}
