/// This defines an object-based Token. The key differentiating features from the Aptos standard
/// token are:
/// * Decoupled token ownership from token data.
/// * Explicit data model for token metadata via adjacent resources
/// * Extensible framework for tokens
///
module aptos_token_objects::token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::signer;
    use std::vector;
    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, Object};
    use aptos_token_objects::collection::{Self, Collection};
    use aptos_token_objects::royalty::{Self, Royalty};

    /// The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 1;
    /// The provided signer is not the creator
    const ENOT_CREATOR: u64 = 2;
    /// The field being changed is not mutable
    const EFIELD_NOT_MUTABLE: u64 = 3;
    /// The token name is over the maximum length
    const ETOKEN_NAME_TOO_LONG: u64 = 4;
    /// The URI is over the maximum length
    const EURI_TOO_LONG: u64 = 5;
    /// The description is over the maximum length
    const EDESCRIPTION_TOO_LONG: u64 = 6;

    const MAX_TOKEN_NAME_LENGTH: u64 = 128;
    const MAX_URI_LENGTH: u64 = 512;
    const MAX_DESCRIPTION_LENGTH: u64 = 2048;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct Token has key {
        /// The collection from which this token resides.
        collection: Object<Collection>,
        /// Unique identifier within the collection, optional, 0 means unassigned
        index: u64,
        /// A brief description of the token.
        description: String,
        /// The name of the token, which should be unique within the collection; the length of name
        /// should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
        /// Emitted upon any mutation of the token.
        mutation_events: event::EventHandle<MutationEvent>,
    }

    /// This enables burning an NFT, if possible, it will also delete the object. Note, the data
    /// in inner and self occupies 32-bytes each, rather than have both, this data structure makes
    /// a small optimization to support either and take a fixed amount of 34-bytes.
    struct BurnRef has drop, store {
        inner: Option<object::DeleteRef>,
        self: Option<address>,
    }

    /// This enables mutating descritpion and URI by higher level services.
    struct MutatorRef has drop, store {
        self: address,
    }

    /// Contains the mutated fields name. This makes the life of indexers easier, so that they can
    /// directly understand the behavior in a writeset.
    struct MutationEvent has drop, store {
        mutated_field_name: String,
        old_value: String,
        new_value: String
    }

    inline fun create_common(
        constructor_ref: &ConstructorRef,
        creator_address: address,
        collection_name: String,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ) {
        assert!(string::length(&name) <= MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));
        assert!(string::length(&description) <= MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));
        assert!(string::length(&uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));

        let object_signer = object::generate_signer(constructor_ref);

        let collection_addr = collection::create_collection_address(&creator_address, &collection_name);
        let collection = object::address_to_object<Collection>(collection_addr);
        let id = collection::increment_supply(&collection, signer::address_of(&object_signer));

        let token = Token {
            collection,
            index: option::get_with_default(&mut id, 0),
            description,
            name,
            uri,
            mutation_events: object::new_event_handle(&object_signer),
        };
        move_to(&object_signer, token);

        if (option::is_some(&royalty)) {
            royalty::init(constructor_ref, option::extract(&mut royalty))
        };
    }

    /// Creates a new token object with a unique address and returns the ConstructorRef
    /// for additional specialization.
    public fun create(
        creator: &signer,
        collection_name: String,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object(creator_address);
        create_common(&constructor_ref, creator_address, collection_name, description, name, royalty, uri);
        constructor_ref
    }

    /// Creates a new token object from a token name and returns the ConstructorRef for
    /// additional specialization.
    public fun create_named_token(
        creator: &signer,
        collection_name: String,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let seed = create_token_seed(&collection_name, &name);

        let constructor_ref = object::create_named_object(creator, seed);
        create_common(&constructor_ref, creator_address, collection_name, description, name, royalty, uri);
        constructor_ref
    }

    #[deprecated]
    /// Creates a new token object from an account GUID and returns the ConstructorRef for
    /// additional specialization.
    public fun create_from_account(
        creator: &signer,
        collection_name: String,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object_from_account(creator);
        create_common(&constructor_ref, creator_address, collection_name, description, name, royalty, uri);
        constructor_ref
    }

    /// Generates the collections address based upon the creators address and the collection's name
    public fun create_token_address(creator: &address, collection: &String, name: &String): address {
        object::create_object_address(creator, create_token_seed(collection, name))
    }

    /// Named objects are derived from a seed, the collection's seed is its name.
    public fun create_token_seed(collection: &String, name: &String): vector<u8> {
        assert!(string::length(name) <= MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));
        let seed = *string::bytes(collection);
        vector::append(&mut seed, b"::");
        vector::append(&mut seed, *string::bytes(name));
        seed
    }

    /// Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.
    public fun generate_mutator_ref(ref: &ConstructorRef): MutatorRef {
        let object = object::object_from_constructor_ref<Token>(ref);
        MutatorRef { self: object::object_address(&object) }
    }

    /// Creates a BurnRef, which gates the ability to burn the given token.
    public fun generate_burn_ref(ref: &ConstructorRef): BurnRef {
        let (inner, self) = if (object::can_generate_delete_ref(ref)) {
            let delete_ref = object::generate_delete_ref(ref);
            (option::some(delete_ref), option::none())
        } else {
            let addr = object::address_from_constructor_ref(ref);
            (option::none(), option::some(addr))
        };
        BurnRef { self, inner }
    }

    /// Extracts the tokens address from a BurnRef.
    public fun address_from_burn_ref(ref: &BurnRef): address {
        if (option::is_some(&ref.inner)) {
            object::address_from_delete_ref(option::borrow(&ref.inner))
        } else {
            *option::borrow(&ref.self)
        }
    }

    // Accessors

    inline fun borrow<T: key>(token: &Object<T>): &Token acquires Token {
        let token_address = object::object_address(token);
        assert!(
            exists<Token>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(token_address)
    }

    #[view]
    public fun creator<T: key>(token: Object<T>): address acquires Token {
        collection::creator(borrow(&token).collection)
    }

    #[view]
    public fun collection_name<T: key>(token: Object<T>): String acquires Token {
        collection::name(borrow(&token).collection)
    }

    #[view]
    public fun collection_object<T: key>(token: Object<T>): Object<Collection> acquires Token {
        borrow(&token).collection
    }

    #[view]
    public fun description<T: key>(token: Object<T>): String acquires Token {
        borrow(&token).description
    }

    #[view]
    public fun name<T: key>(token: Object<T>): String acquires Token {
        borrow(&token).name
    }

    #[view]
    public fun uri<T: key>(token: Object<T>): String acquires Token {
        borrow(&token).uri
    }

    #[view]
    public fun royalty<T: key>(token: Object<T>): Option<Royalty> acquires Token {
        borrow(&token);
        let royalty = royalty::get(token);
        if (option::is_some(&royalty)) {
            royalty
        } else {
            let creator = creator(token);
            let collection_name = collection_name(token);
            let collection_address = collection::create_collection_address(&creator, &collection_name);
            let collection = object::address_to_object<collection::Collection>(collection_address);
            royalty::get(collection)
        }
    }

    // Mutators

    inline fun borrow_mut(mutator_ref: &MutatorRef): &mut Token acquires Token {
        assert!(
            exists<Token>(mutator_ref.self),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global_mut<Token>(mutator_ref.self)
    }

    public fun burn(burn_ref: BurnRef) acquires Token {
        let addr = if (option::is_some(&burn_ref.inner)) {
            let delete_ref = option::extract(&mut burn_ref.inner);
            let addr = object::address_from_delete_ref(&delete_ref);
            object::delete(delete_ref);
            addr
        } else {
            option::extract(&mut burn_ref.self)
        };

        if (royalty::exists_at(addr)) {
            royalty::delete(addr)
        };

        let Token {
            collection,
            index,
            description: _,
            name: _,
            uri: _,
            mutation_events,
        } = move_from<Token>(addr);

        event::destroy_handle(mutation_events);
        collection::decrement_supply(&collection, addr, option::some(index));
    }

    public fun set_description(mutator_ref: &MutatorRef, description: String) acquires Token {
        assert!(string::length(&description) <= MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));
        let token = borrow_mut(mutator_ref);
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent {
                mutated_field_name: string::utf8(b"description"),
                old_value: token.description,
                new_value: description
            },
        );
        token.description = description;
    }

    public fun set_name(mutator_ref: &MutatorRef, name: String) acquires Token {
        assert!(string::length(&name) <= MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));
        let token = borrow_mut(mutator_ref);
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent {
                mutated_field_name: string::utf8(b"name"),
                old_value: token.name,
                new_value: name
            },
        );
        token.name = name;
    }

    public fun set_uri(mutator_ref: &MutatorRef, uri: String) acquires Token {
        assert!(string::length(&uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
        let token = borrow_mut(mutator_ref);
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent {
                mutated_field_name: string::utf8(b"uri"),
                old_value: token.uri,
                new_value: uri,
            },
        );
        token.uri = uri;
    }

    #[test(creator = @0x123, trader = @0x456)]
    fun test_create_and_transfer(creator: &signer, trader: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        create_token_helper(creator, collection_name, token_name);

        let creator_address = signer::address_of(creator);
        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        assert!(object::owner(token) == creator_address, 1);
        object::transfer(creator, token, signer::address_of(trader));
        assert!(object::owner(token) == signer::address_of(trader), 1);

        let expected_royalty = royalty::create(25, 10000, creator_address);
        assert!(option::some(expected_royalty) == royalty(token), 2);
    }

    #[test(creator = @0x123)]
    fun test_collection_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let creator_address = signer::address_of(creator);
        let expected_royalty = royalty::create(10, 1000, creator_address);
        collection::create_fixed_collection(
            creator,
            string::utf8(b"collection description"),
            5,
            collection_name,
            option::some(expected_royalty),
            string::utf8(b"collection uri"),
        );

        create_named_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );

        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        assert!(option::some(expected_royalty) == royalty(token), 0);
    }

    #[test(creator = @0x123)]
    fun test_no_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        collection::create_unlimited_collection(
            creator,
            string::utf8(b"collection description"),
            collection_name,
            option::none(),
            string::utf8(b"collection uri"),
        );

        create_named_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );

        let creator_address = signer::address_of(creator);
        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        assert!(option::none() == royalty(token), 0);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20002, location = aptos_token_objects::collection)]
    fun test_too_many_tokens(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        create_token_helper(creator, collection_name, token_name);
        create_token_helper(creator, collection_name, string::utf8(b"bad"));
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x80001, location = aptos_framework::object)]
    fun test_duplicate_tokens(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 2);
        create_token_helper(creator, collection_name, token_name);
        create_token_helper(creator, collection_name, token_name);
    }

    #[test(creator = @0x123)]
    fun test_set_description(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        let mutator_ref = create_token_with_mutation_ref(creator, collection_name, token_name);
        let token = object::address_to_object<Token>(
            create_token_address(&signer::address_of(creator), &collection_name, &token_name),
        );

        let description = string::utf8(b"no fail");
        assert!(description != description(token), 0);
        set_description(&mutator_ref, description);
        assert!(description == description(token), 1);
    }

    #[test(creator = @0x123)]
    fun test_set_name(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        let mutator_ref = create_token_with_mutation_ref(creator, collection_name, token_name);
        let token = object::address_to_object<Token>(
            create_token_address(&signer::address_of(creator), &collection_name, &token_name),
        );

        let name = string::utf8(b"no fail");
        assert!(name != name(token), 0);
        set_name(&mutator_ref, name);
        assert!(name == name(token), 2);
    }

    #[test(creator = @0x123)]
    fun test_set_uri(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        let mutator_ref = create_token_with_mutation_ref(creator, collection_name, token_name);
        let token = object::address_to_object<Token>(
            create_token_address(&signer::address_of(creator), &collection_name, &token_name),
        );

        let uri = string::utf8(b"no fail");
        assert!(uri != uri(token), 0);
        set_uri(&mutator_ref, uri);
        assert!(uri == uri(token), 1);
    }

    #[test(creator = @0x123)]
    fun test_burn_without_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        let constructor_ref = create_named_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );
        let burn_ref = generate_burn_ref(&constructor_ref);
        let token_addr = object::address_from_constructor_ref(&constructor_ref);
        assert!(exists<Token>(token_addr), 0);
        assert!(!royalty::exists_at(token_addr), 3);
        burn(burn_ref);
        assert!(!exists<Token>(token_addr), 2);
        assert!(!royalty::exists_at(token_addr), 3);
    }

    #[test(creator = @0x123)]
    fun test_burn_with_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        let constructor_ref = create_named_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::some(royalty::create(1, 1, signer::address_of(creator))),
            string::utf8(b"token uri"),
        );
        let burn_ref = generate_burn_ref(&constructor_ref);
        let token_addr = object::address_from_constructor_ref(&constructor_ref);
        assert!(exists<Token>(token_addr), 0);
        assert!(royalty::exists_at(token_addr), 1);
        burn(burn_ref);
        assert!(!exists<Token>(token_addr), 2);
        assert!(!royalty::exists_at(token_addr), 3);
        assert!(object::is_object(token_addr), 4);
    }

    #[test(creator = @0x123)]
    fun test_create_from_account_burn_and_delete(creator: &signer) acquires Token {
        use aptos_framework::account;

        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        account::create_account_for_test(signer::address_of(creator));
        let constructor_ref = create_from_account(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );
        let burn_ref = generate_burn_ref(&constructor_ref);
        let token_addr = object::address_from_constructor_ref(&constructor_ref);
        assert!(exists<Token>(token_addr), 0);
        burn(burn_ref);
        assert!(!exists<Token>(token_addr), 1);
        assert!(!object::is_object(token_addr), 2);
    }

    #[test(creator = @0x123,fx = @std)]
    fun test_create_burn_and_delete(creator: &signer, fx: signer) acquires Token {
        use aptos_framework::account;
        use std::features;

        let feature = features::get_auids();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name, 1);
        account::create_account_for_test(signer::address_of(creator));
        let constructor_ref = create(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::none(),
            string::utf8(b"token uri"),
        );
        let burn_ref = generate_burn_ref(&constructor_ref);
        let token_addr = object::address_from_constructor_ref(&constructor_ref);
        assert!(exists<Token>(token_addr), 0);
        burn(burn_ref);
        assert!(!exists<Token>(token_addr), 1);
        assert!(!object::is_object(token_addr), 2);
    }

    #[test_only]
    fun create_collection_helper(creator: &signer, collection_name: String, max_supply: u64) {
        collection::create_fixed_collection(
            creator,
            string::utf8(b"collection description"),
            max_supply,
            collection_name,
            option::none(),
            string::utf8(b"collection uri"),
        );
    }

    #[test_only]
    fun create_token_helper(creator: &signer, collection_name: String, token_name: String): ConstructorRef {
        create_named_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            option::some(royalty::create(25, 10000, signer::address_of(creator))),
            string::utf8(b"uri"),
        )
    }

    #[test_only]
    fun create_token_with_mutation_ref(
        creator: &signer,
        collection_name: String,
        token_name: String,
    ): MutatorRef {
        let constructor_ref = create_token_helper(creator, collection_name, token_name);
        generate_mutator_ref(&constructor_ref)
    }
}
