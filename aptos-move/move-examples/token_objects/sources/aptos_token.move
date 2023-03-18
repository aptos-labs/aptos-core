/// This defines a minimally viable token for no-code solutions akin the the original token at
/// 0x3::token module.
/// The key features are:
/// * Base token and collection features
/// * Creator definable mutability for tokens
/// * Creator-based freezing of tokens
/// * Standard object-based transfer and events
/// * Metadata property type
///
/// TODO:
/// * Fungible of tokens
module token_objects::aptos_token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::String;
    use std::signer;

    use aptos_framework::object::{Self, ConstructorRef, Object};

    use token_objects::collection;
    use token_objects::property_map;
    use token_objects::royalty;
    use token_objects::token;

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
        /// Determines if the creator can mutate the collection's description
        mutable_description: bool,
        /// Determines if the creator can mutate the collection's uri
        mutable_uri: bool,
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
        property_mutator_ref: Option<property_map::MutatorRef>,
        /// Determines if the creator can mutate the description
        mutable_description: bool,
        /// Determines if the creator can mutate the name
        mutable_name: bool,
        /// Determines if the creator can mutate the uri
        mutable_uri: bool,
        /// Determines if the creator can freeze this NFT
        freezable_by_creator: bool,
    }

    /// Create a new collection
    public entry fun create_collection(
        creator: &signer,
        description: String,
        max_supply: u64,
        name: String,
        uri: String,
        mutable_description: bool,
        mutable_uri: bool,
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

        let aptos_collection = AptosCollection {
            mutator_ref,
            mutable_description,
            mutable_uri,
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
        mutable_description: bool,
        mutable_name: bool,
        mutable_uri: bool,
        burnable_by_creator: bool,
        freezable_by_creator: bool,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        mutable_properties: bool,
    ) acquires AptosToken {
        let constructor_ref = mint_internal(
            creator,
            collection,
            description,
            name,
            uri,
            mutable_description,
            mutable_name,
            mutable_uri,
            burnable_by_creator,
            property_keys,
            property_types,
            property_values,
            mutable_properties,
        );

        if (!freezable_by_creator) {
            return
        };

        let aptos_token_obj = object::object_from_constructor_ref<AptosToken>(&constructor_ref);
        let aptos_token = borrow_global_mut<AptosToken>(object::object_address(&aptos_token_obj));
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        aptos_token.freezable_by_creator = true;
        option::fill(&mut aptos_token.transfer_ref, transfer_ref);
    }

    /// With an existing collection, directly mint a soul bound token into the recipient's account.
    public entry fun mint_soul_bound(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        uri: String,
        mutable_description: bool,
        mutable_name: bool,
        mutable_uri: bool,
        burnable_by_creator: bool,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        mutable_properties: bool,
        soul_bound_to: address,
    ) {
        let constructor_ref = mint_internal(
            creator,
            collection,
            description,
            name,
            uri,
            mutable_description,
            mutable_name,
            mutable_uri,
            burnable_by_creator,
            property_keys,
            property_types,
            property_values,
            mutable_properties,
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
        mutable_description: bool,
        mutable_name: bool,
        mutable_uri: bool,
        burnable_by_creator: bool,
        property_keys: vector<String>,
        property_types: vector<String>,
        property_values: vector<vector<u8>>,
        mutable_properties: bool,
    ): ConstructorRef {
        let constructor_ref = token::create(
            creator,
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        let object_signer = object::generate_signer(&constructor_ref);
        let mutator_ref = if (mutable_description || mutable_name || mutable_uri) {
            option::some(token::generate_mutator_ref(&constructor_ref))
        } else {
            option::none()
        };

        let burn_ref = if (burnable_by_creator) {
            option::some(token::generate_burn_ref(&constructor_ref))
        } else {
            option::none()
        };

        let property_mutator_ref = if (mutable_properties) {
            option::some(property_map::generate_mutator_ref(&constructor_ref))
        } else {
            option::none()
        };

        let aptos_token = AptosToken {
            burn_ref,
            transfer_ref: option::none(),
            mutator_ref,
            property_mutator_ref,
            mutable_description,
            mutable_name,
            mutable_uri,
            freezable_by_creator: false,
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

    public fun are_properties_mutable<T: key>(token: Object<T>): bool acquires AptosToken {
        option::is_some(&borrow(&token).property_mutator_ref)
    }

    public fun is_burnable<T: key>(token: Object<T>): bool acquires AptosToken {
        option::is_some(&borrow(&token).burn_ref)
    }

    public fun is_freezable_by_creator<T: key>(token: Object<T>): bool acquires AptosToken {
        borrow(&token).freezable_by_creator
    }

    public fun is_mutable_description<T: key>(token: Object<T>): bool acquires AptosToken {
        borrow(&token).mutable_description
    }

    public fun is_mutable_name<T: key>(token: Object<T>): bool acquires AptosToken {
        borrow(&token).mutable_name
    }

    public fun is_mutable_uri<T: key>(token: Object<T>): bool acquires AptosToken {
        borrow(&token).mutable_uri
    }

    // Token mutators

    inline fun borrow_mut<T: key>(token: &Object<T>, creator: address): &mut AptosToken {
        let token_address = object::object_address(token);
        assert!(
            exists<AptosToken>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );

        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(*token) == creator,
            error::permission_denied(ENOT_CREATOR),
        );
        aptos_token
    }

    public fun burn<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
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
            property_mutator_ref: _,
            mutable_description: _,
            mutable_name: _,
            mutable_uri: _,
            freezable_by_creator: _,
        } = aptos_token;
        token::burn(option::extract(&mut burn_ref));
    }

    public fun freeze_transfer<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            aptos_token.freezable_by_creator,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        object::disable_ungated_transfer(option::borrow(&aptos_token.transfer_ref));
    }

    public fun unfreeze_transfer<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            aptos_token.freezable_by_creator,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        object::enable_ungated_transfer(option::borrow(&aptos_token.transfer_ref));
    }

    public fun set_description<T: key>(
        creator: &signer,
        token: Object<T>,
        description: String,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            aptos_token.mutable_description,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        token::set_description(option::borrow(&aptos_token.mutator_ref), description);
    }

    public fun set_name<T: key>(
        creator: &signer,
        token: Object<T>,
        name: String,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            aptos_token.mutable_name,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        token::set_name(option::borrow(&aptos_token.mutator_ref), name);
    }

    public fun set_uri<T: key>(
        creator: &signer,
        token: Object<T>,
        uri: String,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            aptos_token.mutable_uri,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        token::set_uri(option::borrow(&aptos_token.mutator_ref), uri);
    }

    public fun add_property<T: key>(
        creator: &signer,
        token: Object<T>,
        key: String,
        type: String,
        value: vector<u8>,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_token.property_mutator_ref),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::add(
            option::borrow(&aptos_token.property_mutator_ref),
            key,
            type,
            value,
        );
    }

    public fun add_typed_property<T: key, V: drop>(
        creator: &signer,
        token: Object<T>,
        key: String,
        value: V,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_token.property_mutator_ref),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::add_typed(
            option::borrow(&aptos_token.property_mutator_ref),
            key,
            value,
        );
    }

    public fun remove_property<T: key>(creator: &signer, token: Object<T>, key: &String) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_token.property_mutator_ref),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::remove(option::borrow(&aptos_token.property_mutator_ref), key);
    }

    public fun update_property<T: key>(
        creator: &signer,
        token: Object<T>,
        key: &String,
        type: String,
        value: vector<u8>,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_token.property_mutator_ref),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::update(
            option::borrow(&aptos_token.property_mutator_ref),
            key,
            type,
            value,
        );
    }

    public fun update_typed_property<T: key, V: drop>(
        creator: &signer,
        token: Object<T>,
        key: &String,
        value: V,
    ) acquires AptosToken {
        let aptos_token = borrow_mut(&token, signer::address_of(creator));
        assert!(
            option::is_some(&aptos_token.property_mutator_ref),
            error::permission_denied(EPROPERTIES_NOT_MUTABLE),
        );

        property_map::update_typed(
            option::borrow(&aptos_token.property_mutator_ref),
            key,
            value,
        );
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
    ) acquires AptosToken {
        freeze_transfer(creator, token_object(creator, &collection, &name));
    }

    entry fun unfreeze_transfer_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosToken {
        unfreeze_transfer(creator, token_object(creator, &collection, &name));
    }

    entry fun set_description_call(
        creator: &signer,
        collection: String,
        name: String,
        description: String,
    ) acquires AptosToken {
        set_description(creator, token_object(creator, &collection, &name), description);
    }

    entry fun set_name_call(
        creator: &signer,
        collection: String,
        original_name: String,
        new_name: String,
    ) acquires AptosToken {
        set_name(creator, token_object(creator, &collection, &original_name), new_name);
    }

    entry fun set_uri_call(
        creator: &signer,
        collection: String,
        name: String,
        uri: String,
    ) acquires AptosToken {
        set_uri(creator, token_object(creator, &collection, &name), uri);
    }

    entry fun add_property_call(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        type: String,
        value: vector<u8>,
    ) acquires AptosToken {
        let token = token_object(creator, &collection, &name);
        add_property(creator, token, key, type, value);
    }

    entry fun add_typed_property_call<T: drop>(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        value: T,
    ) acquires AptosToken {
        let token = token_object(creator, &collection, &name);
        add_typed_property(creator, token, key, value);
    }

    entry fun remove_property_call(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
    ) acquires AptosToken {
        let token = token_object(creator, &collection, &name);
        remove_property(creator, token, &key);
    }

    entry fun update_property_call(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        type: String,
        value: vector<u8>,
    ) acquires AptosToken {
        let token = token_object(creator, &collection, &name);
        update_property(creator, token, &key, type, value);
    }

    entry fun update_typed_property_call<T: drop>(
        creator: &signer,
        collection: String,
        name: String,
        key: String,
        value: T,
    ) acquires AptosToken {
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

    public fun is_mutable_collection_uri<T: key>(
        collection: Object<T>,
    ): bool acquires AptosCollection {
        borrow_collection(&collection).mutable_uri
    }

    // Collection mutators

    inline fun authorized_borrow<T: key>(collection: &Object<T>, creator: address): &AptosCollection {
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
        let aptos_collection = authorized_borrow(&collection, signer::address_of(creator));
        assert!(
            aptos_collection.mutable_description,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        collection::set_description(option::borrow(&aptos_collection.mutator_ref), description);
    }

    public fun set_collection_uri<T: key>(
        creator: &signer,
        collection: Object<T>,
        uri: String,
    ) acquires AptosCollection {
        let aptos_collection = authorized_borrow(&collection, signer::address_of(creator));
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
    fun test_create_and_transfer(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        assert!(object::owner(token) == signer::address_of(creator), 1);
        object::transfer(creator, token, @0x345);
        assert!(object::owner(token) == @0x345, 1);
    }

    #[test(creator = @0x123, bob = @0x456)]
    #[expected_failure(abort_code = 0x50003, location = object)]
    fun test_mint_soul_bound(creator: &signer, bob: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_soul_bound(
            creator,
            collection_name,
            string::utf8(b""),
            token_name,
            string::utf8(b""),
            false,
            false,
            false,
            false,
            vector[],
            vector[],
            vector[],
            false,
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
    fun test_frozen_transfer(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer_call(creator, collection_name, token_name);
        object::transfer(creator, token, @0x345);
    }

    #[test(creator = @0x123)]
    fun test_unfrozen_transfer(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer_call(creator, collection_name, token_name);
        unfreeze_transfer_call(creator, collection_name, token_name);
        object::transfer(creator, token, @0x345);
    }

    #[test(creator = @0x123, another = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_noncreator_freeze(creator: &signer, another: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer(another, token);
    }

    #[test(creator = @0x123, another = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_noncreator_unfreeze(creator: &signer, another: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer_call(creator, collection_name, token_name);
        unfreeze_transfer(another, token);
    }

    #[test(creator = @0x123)]
    fun test_set_description(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let description = string::utf8(b"not");
        assert!(token::description(token) != description, 0);
        set_description_call(creator, collection_name, token_name, description);
        assert!(token::description(token) == description, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_description(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_helper(creator, collection_name, token_name, false);

        set_description_call(creator, collection_name, token_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_description_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let description = string::utf8(b"not");
        set_description(noncreator, token, description);
    }

    #[test(creator = @0x123)]
    fun test_set_name(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let name = string::utf8(b"not");
        assert!(token::name(token) != name, 0);
        set_name_call(creator, collection_name, token_name, name);
        assert!(token::name(token) == name, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_name(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_helper(creator, collection_name, token_name, false);

        set_name_call(creator, collection_name, token_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_name_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let name = string::utf8(b"not");
        set_name(noncreator, token, name);
    }

    #[test(creator = @0x123)]
    fun test_set_uri(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let uri = string::utf8(b"not");
        assert!(token::uri(token) != uri, 0);
        set_uri_call(creator, collection_name, token_name, uri);
        assert!(token::uri(token) == uri, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    fun test_set_immutable_uri(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        mint_helper(creator, collection_name, token_name, false);

        set_uri_call(creator, collection_name, token_name, string::utf8(b""));
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_set_uri_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let uri = string::utf8(b"not");
        set_uri(noncreator, token, uri);
    }

    #[test(creator = @0x123)]
    entry fun test_burnable(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

        let token_addr = token::create_token_address(&signer::address_of(creator), &collection_name, &token_name);
        assert!(exists<AptosToken>(token_addr), 0);
        burn(creator, token);
        assert!(!exists<AptosToken>(token_addr), 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    entry fun test_not_burnable(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, false);

        burn(creator, token);
    }

    #[test(creator = @0x123, noncreator = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    entry fun test_burn_non_creator(
        creator: &signer,
        noncreator: &signer,
    ) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, false);
        let token = mint_helper(creator, collection_name, token_name, true);

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
        flag: bool,
    ): Object<AptosToken> acquires AptosToken {
        mint(
            creator,
            collection_name,
            string::utf8(b"description"),
            token_name,
            string::utf8(b"uri"),
            flag,
            flag,
            flag,
            flag,
            flag,
            vector[string::utf8(b"bool")],
            vector[string::utf8(b"bool")],
            vector[vector[0x01]],
            flag,
        );

        let token_addr = token::create_token_address(
            &signer::address_of(creator),
            &collection_name,
            &token_name,
        );
        object::address_to_object<AptosToken>(token_addr)
    }
}
