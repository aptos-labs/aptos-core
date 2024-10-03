/// This defines an object-based Token. The key differentiating features from the Aptos standard
/// token are:
/// * Decoupled token ownership from token data.
/// * Explicit data model for token metadata via adjacent resources
/// * Extensible framework for tokens
///
module aptos_token_objects::token {
    use std::error;
    use std::features;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::signer;
    use std::vector;
    use aptos_framework::aggregator_v2::{Self, AggregatorSnapshot, DerivedStringSnapshot};
    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, Object};
    use aptos_token_objects::collection::{Self, Collection};
    use aptos_token_objects::royalty::{Self, Royalty};

    #[test_only]
    use aptos_framework::object::ExtendRef;

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
    /// The seed is over the maximum length
    const ESEED_TOO_LONG: u64 = 7;
    /// The calling signer is not the owner
    const ENOT_OWNER: u64 = 8;
    /// The collection owner feature is not supported
    const ECOLLECTION_OWNER_NOT_SUPPORTED: u64 = 9;

    const MAX_TOKEN_NAME_LENGTH: u64 = 128;
    const MAX_TOKEN_SEED_LENGTH: u64 = 128;
    const MAX_URI_LENGTH: u64 = 512;
    const MAX_DESCRIPTION_LENGTH: u64 = 2048;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct Token has key {
        /// The collection from which this token resides.
        collection: Object<Collection>,
        /// Deprecated in favor of `index` inside TokenIdentifiers.
        /// Was populated until concurrent_token_v2_enabled feature flag was enabled.
        ///
        /// Unique identifier within the collection, optional, 0 means unassigned
        index: u64,
        // DEPRECATED
        /// A brief description of the token.
        description: String,
        /// Deprecated in favor of `name` inside TokenIdentifiers.
        /// Was populated until concurrent_token_v2_enabled feature flag was enabled.
        ///
        /// The name of the token, which should be unique within the collection; the length of name
        /// should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
        // DEPRECATED
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
        /// Emitted upon any mutation of the token.
        mutation_events: event::EventHandle<MutationEvent>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents first addition to the common fields for all tokens
    /// Started being populated once aggregator_v2_api_enabled was enabled.
    struct TokenIdentifiers has key {
        /// Unique identifier within the collection, optional, 0 means unassigned
        index: AggregatorSnapshot<u64>,
        /// The name of the token, which should be unique within the collection; the length of name
        /// should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: DerivedStringSnapshot,
    }

    // DEPRECATED, NEVER USED
    #[deprecated]
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct ConcurrentTokenIdentifiers has key {
        index: AggregatorSnapshot<u64>,
        name: AggregatorSnapshot<String>,
    }

    /// This enables burning an NFT, if possible, it will also delete the object. Note, the data
    /// in inner and self occupies 32-bytes each, rather than have both, this data structure makes
    /// a small optimization to support either and take a fixed amount of 34-bytes.
    struct BurnRef has drop, store {
        inner: Option<object::DeleteRef>,
        self: Option<address>,
    }

    /// This enables mutating description and URI by higher level services.
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

    #[event]
    struct Mutation has drop, store {
        token_address: address,
        mutated_field_name: String,
        old_value: String,
        new_value: String
    }

    inline fun create_common(
        creator: &signer,
        constructor_ref: &ConstructorRef,
        collection_name: String,
        description: String,
        name_prefix: String,
        // If option::some, numbered token is created - i.e. index is appended to the name.
        // If option::none, name_prefix is the full name of the token.
        name_with_index_suffix: Option<String>,
        royalty: Option<Royalty>,
        uri: String,
    ) {
        let creator_address = signer::address_of(creator);
        let collection_addr = collection::create_collection_address(&creator_address, &collection_name);
        let collection = object::address_to_object<Collection>(collection_addr);

        create_common_with_collection(
            creator,
            constructor_ref,
            collection,
            description,
            name_prefix,
            name_with_index_suffix,
            royalty,
            uri
        )
    }

    inline fun create_common_with_collection(
        creator: &signer,
        constructor_ref: &ConstructorRef,
        collection: Object<Collection>,
        description: String,
        name_prefix: String,
        // If option::some, numbered token is created - i.e. index is appended to the name.
        // If option::none, name_prefix is the full name of the token.
        name_with_index_suffix: Option<String>,
        royalty: Option<Royalty>,
        uri: String,
    ) {
        assert!(collection::creator(collection) == signer::address_of(creator), error::unauthenticated(ENOT_CREATOR));

        create_common_with_collection_internal(
            constructor_ref,
            collection,
            description,
            name_prefix,
            name_with_index_suffix,
            royalty,
            uri
        );
    }

    inline fun create_common_with_collection_as_owner(
        owner: &signer,
        constructor_ref: &ConstructorRef,
        collection: Object<Collection>,
        description: String,
        name_prefix: String,
        // If option::some, numbered token is created - i.e. index is appended to the name.
        // If option::none, name_prefix is the full name of the token.
        name_with_index_suffix: Option<String>,
        royalty: Option<Royalty>,
        uri: String,
    ) {
        assert!(features::is_collection_owner_enabled(), error::unavailable(ECOLLECTION_OWNER_NOT_SUPPORTED));
        assert!(object::owner(collection) == signer::address_of(owner), error::unauthenticated(ENOT_OWNER));

        create_common_with_collection_internal(
            constructor_ref,
            collection,
            description,
            name_prefix,
            name_with_index_suffix,
            royalty,
            uri
        );
    }

    inline fun create_common_with_collection_internal(
        constructor_ref: &ConstructorRef,
        collection: Object<Collection>,
        description: String,
        name_prefix: String,
        // If option::some, numbered token is created - i.e. index is appended to the name.
        // If option::none, name_prefix is the full name of the token.
        name_with_index_suffix: Option<String>,
        royalty: Option<Royalty>,
        uri: String,
    ) {
        if (option::is_some(&name_with_index_suffix)) {
            // Be conservative, as we don't know what length the index will be, and assume worst case (20 chars in MAX_U64)
            assert!(
                string::length(&name_prefix) + 20 + string::length(
                    option::borrow(&name_with_index_suffix)
                ) <= MAX_TOKEN_NAME_LENGTH,
                error::out_of_range(ETOKEN_NAME_TOO_LONG)
            );
        } else {
            assert!(string::length(&name_prefix) <= MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));
        };
        assert!(string::length(&description) <= MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));
        assert!(string::length(&uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));

        let object_signer = object::generate_signer(constructor_ref);

        let index = option::destroy_with_default(
            collection::increment_supply(&collection, signer::address_of(&object_signer)),
            aggregator_v2::create_snapshot<u64>(0)
        );

        // If create_numbered_token called us, add index to the name.
        let name = if (option::is_some(&name_with_index_suffix)) {
            aggregator_v2::derive_string_concat(name_prefix, &index, option::extract(&mut name_with_index_suffix))
        } else {
            aggregator_v2::create_derived_string(name_prefix)
        };

        let deprecated_index = 0;
        let deprecated_name = string::utf8(b"");

        let token_concurrent = TokenIdentifiers {
            index,
            name,
        };
        move_to(&object_signer, token_concurrent);

        let token = Token {
            collection,
            index: deprecated_index,
            description,
            name: deprecated_name,
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
    /// This takes in the collection object instead of the collection name.
    /// This function must be called if the collection name has been previously changed.
    public fun create_token(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object(creator_address);
        create_common_with_collection(
            creator,
            &constructor_ref,
            collection,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
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
        create_common(
            creator,
            &constructor_ref,
            collection_name,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Same functionality as `create_token`, but the token can only be created by the collection owner.
    public fun create_token_as_collection_owner(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object(creator_address);
        create_common_with_collection_as_owner(
            creator,
            &constructor_ref,
            collection,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Creates a new token object with a unique address and returns the ConstructorRef
    /// for additional specialization.
    /// The name is created by concatenating the (name_prefix, index, name_suffix).
    /// This function allows creating tokens in parallel, from the same collection,
    /// while providing sequential names.
    ///
    /// This takes in the collection object instead of the collection name.
    /// This function must be called if the collection name has been previously changed.
    public fun create_numbered_token_object(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name_with_index_prefix: String,
        name_with_index_suffix: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object(creator_address);
        create_common_with_collection(
            creator,
            &constructor_ref,
            collection,
            description,
            name_with_index_prefix,
            option::some(name_with_index_suffix),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Creates a new token object with a unique address and returns the ConstructorRef
    /// for additional specialization.
    /// The name is created by concatenating the (name_prefix, index, name_suffix).
    /// This function will allow creating tokens in parallel, from the same collection,
    /// while providing sequential names.
    public fun create_numbered_token(
        creator: &signer,
        collection_name: String,
        description: String,
        name_with_index_prefix: String,
        name_with_index_suffix: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object(creator_address);
        create_common(
            creator,
            &constructor_ref,
            collection_name,
            description,
            name_with_index_prefix,
            option::some(name_with_index_suffix),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Same functionality as `create_numbered_token_object`, but the token can only be created by the collection owner.
    public fun create_numbered_token_as_collection_owner(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name_with_index_prefix: String,
        name_with_index_suffix: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let creator_address = signer::address_of(creator);
        let constructor_ref = object::create_object(creator_address);
        create_common_with_collection_as_owner(
            creator,
            &constructor_ref,
            collection,
            description,
            name_with_index_prefix,
            option::some(name_with_index_suffix),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Creates a new token object from a token name and returns the ConstructorRef for
    /// additional specialization.
    /// This function must be called if the collection name has been previously changed.
    public fun create_named_token_object(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let seed = create_token_seed(&collection::name(collection), &name);
        let constructor_ref = object::create_named_object(creator, seed);
        create_common_with_collection(
            creator,
            &constructor_ref,
            collection,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
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
        let seed = create_token_seed(&collection_name, &name);

        let constructor_ref = object::create_named_object(creator, seed);
        create_common(
            creator,
            &constructor_ref,
            collection_name,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Same functionality as `create_named_token_object`, but the token can only be created by the collection owner.
    public fun create_named_token_as_collection_owner(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let seed = create_token_seed(&collection::name(collection), &name);
        let constructor_ref = object::create_named_object(creator, seed);
        create_common_with_collection_as_owner(
            creator,
            &constructor_ref,
            collection,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Creates a new token object from a token name and seed.
    /// Returns the ConstructorRef for additional specialization.
    /// This function must be called if the collection name has been previously changed.
    public fun create_named_token_from_seed(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        seed: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let seed = create_token_name_with_seed(&collection::name(collection), &name, &seed);
        let constructor_ref = object::create_named_object(creator, seed);
        create_common_with_collection(creator, &constructor_ref, collection, description, name, option::none(), royalty, uri);
        constructor_ref
    }

    /// Same functionality as `create_named_token_from_seed`, but the token can only be created by the collection owner.
    public fun create_named_token_from_seed_as_collection_owner(
        creator: &signer,
        collection: Object<Collection>,
        description: String,
        name: String,
        seed: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let seed = create_token_name_with_seed(&collection::name(collection), &name, &seed);
        let constructor_ref = object::create_named_object(creator, seed);
        create_common_with_collection_as_owner(
            creator,
            &constructor_ref,
            collection,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
    }

    #[deprecated]
    /// DEPRECATED: Use `create` instead for identical behavior.
    ///
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
        let constructor_ref = object::create_object_from_account(creator);
        create_common(
            creator,
            &constructor_ref,
            collection_name,
            description,
            name,
            option::none(),
            royalty,
            uri
        );
        constructor_ref
    }

    /// Generates the token's address based upon the creator's address, the collection's name and the token's name.
    public fun create_token_address(creator: &address, collection: &String, name: &String): address {
        object::create_object_address(creator, create_token_seed(collection, name))
    }

    #[view]
    /// Generates the token's address based upon the creator's address, the collection object and the token's name and seed.
    public fun create_token_address_with_seed(creator: address, collection: String, name: String, seed: String): address {
        let seed = create_token_name_with_seed(&collection, &name, &seed);
        object::create_object_address(&creator, seed)
    }

    /// Named objects are derived from a seed, the token's seed is its name appended to the collection's name.
    public fun create_token_seed(collection: &String, name: &String): vector<u8> {
        assert!(string::length(name) <= MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));
        let seed = *string::bytes(collection);
        vector::append(&mut seed, b"::");
        vector::append(&mut seed, *string::bytes(name));
        seed
    }

    public fun create_token_name_with_seed(collection: &String, name: &String, seed: &String): vector<u8> {
        assert!(string::length(seed) <= MAX_TOKEN_SEED_LENGTH, error::out_of_range(ESEED_TOO_LONG));
        let seeds = create_token_seed(collection, name);
        vector::append(&mut seeds, *string::bytes(seed));
        seeds
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

    // To be added if/when needed - i.e. if there is a need to access name of the numbered token
    // within the transaction that creates it, to set additional application-specific fields.
    //
    // /// This method allows minting to happen in parallel, making it efficient.
    // fun name_snapshot<T: key>(token: &Object<T>): AggregatorSnapshot<String> acquires Token, TokenIdentifiers {
    //     let token_address = object::object_address(token);
    //     if (exists<TokenIdentifiers>(token_address)) {
    //         aggregator_v2::copy_snapshot(&borrow_global<TokenIdentifiers>(token_address).name)
    //     } else {
    //         aggregator_v2::create_snapshot(borrow(token).name)
    //     }
    // }

    #[view]
    /// Avoid this method in the same transaction as the token is minted
    /// as that would prohibit transactions to be executed in parallel.
    public fun name<T: key>(token: Object<T>): String acquires Token, TokenIdentifiers {
        let token_address = object::object_address(&token);
        if (exists<TokenIdentifiers>(token_address)) {
            aggregator_v2::read_derived_string(&borrow_global<TokenIdentifiers>(token_address).name)
        } else {
            borrow(&token).name
        }
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

    // To be added if/when needed - i.e. if there is a need to access index of the token
    // within the transaction that creates it, to set additional application-specific fields.
    //
    // /// This method allows minting to happen in parallel, making it efficient.
    // fun index_snapshot<T: key>(token: &Object<T>): AggregatorSnapshot<u64> acquires Token, TokenIdentifiers {
    //     let token_address = object::object_address(token);
    //     if (exists<TokenIdentifiers>(token_address)) {
    //         aggregator_v2::copy_snapshot(&borrow_global<TokenIdentifiers>(token_address).index)
    //     } else {
    //         aggregator_v2::create_snapshot(borrow(token).index)
    //     }
    // }

    #[view]
    /// Avoid this method in the same transaction as the token is minted
    /// as that would prohibit transactions to be executed in parallel.
    public fun index<T: key>(token: Object<T>): u64 acquires Token, TokenIdentifiers {
        let token_address = object::object_address(&token);
        if (exists<TokenIdentifiers>(token_address)) {
            aggregator_v2::read_snapshot(&borrow_global<TokenIdentifiers>(token_address).index)
        } else {
            borrow(&token).index
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

    public fun burn(burn_ref: BurnRef) acquires Token, TokenIdentifiers {
        let (addr, previous_owner) = if (option::is_some(&burn_ref.inner)) {
            let delete_ref = option::extract(&mut burn_ref.inner);
            let addr = object::address_from_delete_ref(&delete_ref);
            let previous_owner = object::owner(object::address_to_object<Token>(addr));
            object::delete(delete_ref);
            (addr, previous_owner)
        } else {
            let addr = option::extract(&mut burn_ref.self);
            let previous_owner = object::owner(object::address_to_object<Token>(addr));
            (addr, previous_owner)
        };

        if (royalty::exists_at(addr)) {
            royalty::delete(addr)
        };

        let Token {
            collection,
            index: deprecated_index,
            description: _,
            name: _,
            uri: _,
            mutation_events,
        } = move_from<Token>(addr);

        let index = if (exists<TokenIdentifiers>(addr)) {
            let TokenIdentifiers {
                index,
                name: _,
            } = move_from<TokenIdentifiers>(addr);
            aggregator_v2::read_snapshot(&index)
        } else {
            deprecated_index
        };

        event::destroy_handle(mutation_events);
        collection::decrement_supply(&collection, addr, option::some(index), previous_owner);
    }

    public fun set_description(mutator_ref: &MutatorRef, description: String) acquires Token {
        assert!(string::length(&description) <= MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));
        let token = borrow_mut(mutator_ref);
        if (std::features::module_event_migration_enabled()) {
            event::emit(Mutation {
                token_address: mutator_ref.self,
                mutated_field_name: string::utf8(b"description"),
                old_value: token.description,
                new_value: description
            })
        };
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

    public fun set_name(mutator_ref: &MutatorRef, name: String) acquires Token, TokenIdentifiers {
        assert!(string::length(&name) <= MAX_TOKEN_NAME_LENGTH, error::out_of_range(ETOKEN_NAME_TOO_LONG));

        let token = borrow_mut(mutator_ref);

        let old_name = if (exists<TokenIdentifiers>(mutator_ref.self)) {
            let token_concurrent = borrow_global_mut<TokenIdentifiers>(mutator_ref.self);
            let old_name = aggregator_v2::read_derived_string(&token_concurrent.name);
            token_concurrent.name = aggregator_v2::create_derived_string(name);
            old_name
        } else {
            let old_name = token.name;
            token.name = name;
            old_name
        };

        if (std::features::module_event_migration_enabled()) {
            event::emit(Mutation {
                token_address: mutator_ref.self,
                mutated_field_name: string::utf8(b"name"),
                old_value: old_name,
                new_value: name
            })
        };
        event::emit_event(
            &mut token.mutation_events,
            MutationEvent {
                mutated_field_name: string::utf8(b"name"),
                old_value: old_name,
                new_value: name
            },
        );
    }

    public fun set_uri(mutator_ref: &MutatorRef, uri: String) acquires Token {
        assert!(string::length(&uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
        let token = borrow_mut(mutator_ref);
        if (std::features::module_event_migration_enabled()) {
            event::emit(Mutation {
                token_address: mutator_ref.self,
                mutated_field_name: string::utf8(b"uri"),
                old_value: token.uri,
                new_value: uri,
            })
        };
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

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    fun test_create_and_transfer_token_as_collection_owner(creator: &signer, trader: &signer, aptos_framework: &signer) acquires Token {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let extend_ref = create_collection_as_collection_owner_helper(creator, collection_name, 1);
        let collection = get_collection_from_ref(&extend_ref);
        create_named_token_as_collection_owner_helper(creator, collection, token_name);

        let creator_address = signer::address_of(creator);
        let token_addr = create_token_address(&creator_address, &collection_name, &token_name);
        let token = object::address_to_object<Token>(token_addr);
        assert!(object::owner(token) == creator_address, 1);
        object::transfer(creator, token, signer::address_of(trader));
        assert!(object::owner(token) == signer::address_of(trader), 1);

        let expected_royalty = royalty::create(25, 10000, creator_address);
        assert!(option::some(expected_royalty) == royalty(token), 2);
    }

    #[test(creator = @0x123, trader = @0x456)]
    #[expected_failure(abort_code = 0x40002, location = aptos_token_objects::token)]
    fun test_create_token_non_creator(creator: &signer, trader: &signer) {
        let constructor_ref = &create_fixed_collection(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_token(
            trader, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x40008, location = aptos_token_objects::token)]
    fun test_create_token_non_collection_owner(creator: &signer, trader: &signer, aptos_framework: &signer) {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let constructor_ref = &create_fixed_collection_as_collection_owner(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_token_as_collection_owner(
            trader, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123, trader = @0x456)]
    #[expected_failure(abort_code = 0x40002, location = aptos_token_objects::token)]
    fun test_create_named_token_non_creator(creator: &signer, trader: &signer) {
        let constructor_ref = &create_fixed_collection(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_token_with_collection_helper(trader, collection, string::utf8(b"token name"));
    }

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x40008, location = aptos_token_objects::token)]
    fun test_create_named_token_non_collection_owner(creator: &signer, trader: &signer, aptos_framework: &signer) {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let constructor_ref = &create_fixed_collection_as_collection_owner(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_named_token_as_collection_owner_helper(trader, collection, string::utf8(b"token name"));
    }

    #[test(creator = @0x123, trader = @0x456)]
    #[expected_failure(abort_code = 0x40002, location = aptos_token_objects::token)]
    fun test_create_named_token_object_non_creator(creator: &signer, trader: &signer) {
        let constructor_ref = &create_fixed_collection(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_named_token_object(
            trader, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123, trader = @0x456)]
    #[expected_failure(abort_code = 0x40002, location = aptos_token_objects::token)]
    fun test_create_named_token_from_seed_non_creator(creator: &signer, trader: &signer) {
        let constructor_ref = &create_fixed_collection(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_named_token_object(
            trader, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x40008, location = aptos_token_objects::token)]
    fun test_create_named_token_from_seed_non_collection_owner(creator: &signer, trader: &signer, aptos_framework: &signer) {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let constructor_ref = &create_fixed_collection_as_collection_owner(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_named_token_as_collection_owner(
            trader, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123, trader = @0x456)]
    fun test_create_and_transfer_token_with_seed(creator: &signer, trader: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let extend_ref = create_collection_helper(creator, collection_name, 1);
        let collection = get_collection_from_ref(&extend_ref);
        let seed = string::utf8(b"seed");
        create_token_object_with_seed_helper(creator, collection, token_name, seed);

        let creator_address = signer::address_of(creator);
        // Calculate the token address with collection, token name and seed.
        let token_addr = create_token_address_with_seed(creator_address, collection_name, token_name, seed);
        let token = object::address_to_object<Token>(token_addr);
        assert!(object::owner(token) == creator_address, 1);
        object::transfer(creator, token, signer::address_of(trader));
        assert!(object::owner(token) == signer::address_of(trader), 1);

        let expected_royalty = royalty::create(25, 10000, creator_address);
        assert!(option::some(expected_royalty) == royalty(token), 2);
    }

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    #[expected_failure(abort_code = 0x40008, location = aptos_token_objects::token)]
    fun test_create_token_after_transferring_collection(creator: &signer, trader: &signer, aptos_framework: &signer) {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let constructor_ref = &create_fixed_collection_as_collection_owner(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_token_as_collection_owner(
            creator, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );

        object::transfer(creator, collection, signer::address_of(trader));

        // This should fail as the collection is no longer owned by the creator.
        create_token_as_collection_owner(
            creator, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    fun create_token_works_with_new_collection_owner(creator: &signer, trader: &signer, aptos_framework: &signer) {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let constructor_ref = &create_fixed_collection_as_collection_owner(creator, string::utf8(b"collection name"), 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        create_token_as_collection_owner(
            creator, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );

        object::transfer(creator, collection, signer::address_of(trader));

        // This should pass as `trader` is the new collection owner
        create_token_as_collection_owner(
            trader, collection, string::utf8(b"token description"), string::utf8(b"token name"),
            option::some(royalty::create(25, 10000, signer::address_of(creator))), string::utf8(b"uri"),
        );
    }

    #[test(creator = @0x123)]
    fun test_collection_royalty(creator: &signer) acquires Token {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let creator_address = signer::address_of(creator);
        let expected_royalty = royalty::create(10, 1000, creator_address);
        let constructor_ref = collection::create_fixed_collection(
            creator,
            string::utf8(b"collection description"),
            5,
            collection_name,
            option::some(expected_royalty),
            string::utf8(b"collection uri"),
        );

        let collection = object::object_from_constructor_ref<Collection>(&constructor_ref);
        create_named_token_object(
            creator,
            collection,
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
    fun test_set_name(creator: &signer) acquires Token, TokenIdentifiers {
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
    fun test_burn_without_royalty(creator: &signer) acquires Token, TokenIdentifiers {
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
    fun test_burn_with_royalty(creator: &signer) acquires Token, TokenIdentifiers {
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
    fun test_create_from_account_burn_and_delete(creator: &signer) acquires Token, TokenIdentifiers {
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

    #[test(creator = @0x123)]
    fun test_create_burn_and_delete(creator: &signer) acquires Token, TokenIdentifiers {
        use aptos_framework::account;

        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let extend_ref = create_collection_helper(creator, collection_name, 1);
        let collection = get_collection_from_ref(&extend_ref);
        account::create_account_for_test(signer::address_of(creator));
        let constructor_ref = create_token(
            creator,
            collection,
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

    #[test(creator = @0x123)]
    fun test_upgrade_to_concurrent_and_numbered_tokens(creator: &signer) acquires Token, TokenIdentifiers {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let extend_ref = create_collection_helper(creator, collection_name, 2);
        let collection = get_collection_from_ref(&extend_ref);
        let token_1_ref = create_numbered_token_helper(creator, collection, token_name);
        let token_1_name = name(object::object_from_constructor_ref<Token>(&token_1_ref));
        assert!(token_1_name == std::string::utf8(b"token name1"), 1);

        let token_2_ref = create_numbered_token_helper(creator, collection, token_name);
        assert!(name(object::object_from_constructor_ref<Token>(&token_2_ref)) == std::string::utf8(b"token name2"), 1);
        assert!(vector::length(&event::emitted_events<collection::Mint>()) == 2, 0);

        let burn_ref = generate_burn_ref(&token_2_ref);
        let token_addr = object::address_from_constructor_ref(&token_2_ref);
        assert!(exists<Token>(token_addr), 0);
        burn(burn_ref);
        assert!(vector::length(&event::emitted_events<collection::Burn>()) == 1, 0);
    }

    #[test(creator = @0x123)]
    /// This test verifies that once the collection name can be changed, tokens can still be be minted from the collection.
    fun test_change_collection_name(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let constructor_ref = &create_fixed_collection(creator, collection_name, 5);
        let collection = get_collection_from_ref(&object::generate_extend_ref(constructor_ref));
        let mutator_ref = collection::generate_mutator_ref(constructor_ref);

        create_token_with_collection_helper(creator, collection, token_name);
        collection::set_name(&mutator_ref, string::utf8(b"new collection name"));
        create_token_with_collection_helper(creator, collection, token_name);

        assert!(collection::count(collection) == option::some(2), 0);
    }

    #[test_only]
    fun create_collection_helper(creator: &signer, collection_name: String, max_supply: u64): ExtendRef {
        let constructor_ref = create_fixed_collection(creator, collection_name, max_supply);
        object::generate_extend_ref(&constructor_ref)
    }

    #[test_only]
    fun create_collection_as_collection_owner_helper(creator: &signer, collection_name: String, max_supply: u64): ExtendRef {
        let constructor_ref = create_fixed_collection_as_collection_owner(creator, collection_name, max_supply);
        object::generate_extend_ref(&constructor_ref)
    }

    #[test_only]
    fun create_fixed_collection(creator: &signer, collection_name: String, max_supply: u64): ConstructorRef {
        collection::create_fixed_collection(
            creator,
            string::utf8(b"collection description"),
            max_supply,
            collection_name,
            option::none(),
            string::utf8(b"collection uri"),
        )
    }

    #[test_only]
    fun create_fixed_collection_as_collection_owner(
        creator: &signer,
        collection_name: String,
        max_supply: u64,
    ): ConstructorRef {
        collection::create_fixed_collection_as_owner(
            creator,
            string::utf8(b"collection description as owner"),
            max_supply,
            collection_name,
            option::none(),
            string::utf8(b"collection uri as owner"),
        )
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
    fun create_token_with_collection_helper(
        creator: &signer,
        collection: Object<Collection>,
        token_name: String
    ): ConstructorRef {
        create_named_token_object(
            creator,
            collection,
            string::utf8(b"token description"),
            token_name,
            option::some(royalty::create(25, 10000, signer::address_of(creator))),
            string::utf8(b"uri"),
        )
    }

    #[test_only]
    fun create_named_token_as_collection_owner_helper(
        creator: &signer,
        collection: Object<Collection>,
        token_name: String
    ): ConstructorRef {
        create_named_token_as_collection_owner(
            creator,
            collection,
            string::utf8(b"token description"),
            token_name,
            option::some(royalty::create(25, 10000, signer::address_of(creator))),
            string::utf8(b"uri"),
        )
    }

    #[test_only]
    fun create_token_object_with_seed_helper(
        creator: &signer,
        collection: Object<Collection>,
        token_name: String,
        seed: String
    ): ConstructorRef {
        create_named_token_from_seed(
            creator,
            collection,
            string::utf8(b"token description"),
            token_name,
            seed,
            option::some(royalty::create(25, 10000, signer::address_of(creator))),
            string::utf8(b"uri"),
        )
    }

    #[test_only]
    fun create_numbered_token_helper(
        creator: &signer,
        collection: Object<Collection>,
        token_prefix: String
    ): ConstructorRef {
        create_numbered_token_object(
            creator,
            collection,
            string::utf8(b"token description"),
            token_prefix,
            string::utf8(b""),
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

    #[test_only]
    fun get_collection_from_ref(extend_ref: &ExtendRef): Object<Collection> {
        let collection_address = signer::address_of(&object::generate_signer_for_extending(extend_ref));
        object::address_to_object<Collection>(collection_address)
    }
}
