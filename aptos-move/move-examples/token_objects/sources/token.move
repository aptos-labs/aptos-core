/// This defines an object-based Token. The key differentiating features from the Aptos standard
/// token are:
/// * Decouple token ownership from token data.
/// * Explicit data model for token metadata via adjacent resources
/// * Extensible framework for tokens
///
/// TODO:
/// * Provide functions for mutability -- the capability model seems to heavy for mutations, so
///   probably keep the existing model
/// * Consider adding an optional source name if name is mutated, since the objects address depends
///   on the name...
/// * Update ObjectId to be an acceptable param to move
module token_objects::token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::signer;
    use std::vector;

    // The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 0;
    /// The provided signer is not the creator
    const ENOT_CREATOR: u64 = 1;
    /// Attempted to mutate an immutable field
    const EFIELD_NOT_MUTABLE: u64 = 2;

    use aptos_framework::object::{Self, CreatorRef, ObjectId};

    use token_objects::collection::{Self, Royalty};

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct Token has key {
        /// An optional categorization of similar token, there are no constraints on collections.
        collection: String,
        /// The original creator of this token.
        creator: address,
        /// A brief description of the token.
        description: String,
        /// Determines which fields are mutable.
        mutability_config: MutabilityConfig,
        /// The name of the token, which should be unique within the collection; the length of name
        /// should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
        /// The creation name of the token. Since tokens are created with the name as part of the
        /// object id generation.
        creation_name: Option<String>,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
    }

    /// This config specifies which fields in the TokenData are mutable
    struct MutabilityConfig has copy, drop, store {
        description: bool,
        name: bool,
        uri: bool,
    }

    public fun create_token(
        creator: &signer,
        collection: String,
        description: String,
        mutability_config: MutabilityConfig,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): CreatorRef {
        let creator_address = signer::address_of(creator);
        let seed = create_token_id_seed(&collection, &name);
        let creator_ref = object::create_named_object(creator, seed);
        let object_signer = object::generate_signer(&creator_ref);

        collection::increment_supply(&creator_address, &collection);
        let token = Token {
            collection,
            creator: creator_address,
            description,
            mutability_config,
            name,
            creation_name: option::none(),
            uri,
        };
        move_to(&object_signer, token);

        if (option::is_some(&royalty)) {
            collection::init_royalty(&object_signer, option::extract(&mut royalty))
        };
        creator_ref
    }

    public fun create_mutability_config(description: bool, name: bool, uri: bool): MutabilityConfig {
        MutabilityConfig { description, name, uri }
    }

    public fun create_token_id(creator: &address, collection: &String, name: &String): ObjectId {
        object::create_object_id(creator, create_token_id_seed(collection, name))
    }

    public fun create_token_id_seed(collection: &String, name: &String): vector<u8> {
        let seed = *string::bytes(collection);
        vector::append(&mut seed, b"::");
        vector::append(&mut seed, *string::bytes(name));
        seed
    }

    /// Simple token creation that generates a token and deposits it into the creators object store.
    public entry fun mint_token(
        creator: &signer,
        collection: String,
        description: String,
        name: String,
        uri: String,
        mutable_description: bool,
        mutable_name: bool,
        mutable_uri: bool,
        enable_royalty: bool,
        royalty_numerator: u64,
        royalty_denominator: u64,
        royalty_payee_address: address,
    ) {
        let mutability_config = create_mutability_config(
            mutable_description,
            mutable_name,
            mutable_uri,
        );

        let royalty = if (enable_royalty) {
            option::some(collection::create_royalty(
                royalty_numerator,
                royalty_denominator,
                royalty_payee_address,
            ))
        } else {
            option::none()
        };

        create_token(
            creator,
            collection,
            description,
            mutability_config,
            name,
            royalty,
            uri,
        );
    }

    // Accessors

    public fun is_collection(token_id: ObjectId): bool {
        exists<Token>(object::object_id_address(&token_id))
    }

    public fun creator(token_id: ObjectId): address acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).creator
    }

    public fun collection(token_id: ObjectId): String acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).collection
    }

    public fun creation_name(token_id: ObjectId): String acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        let token = borrow_global<Token>(object::object_id_address(&token_id));
        if (option::is_some(&token.creation_name)) {
            *option::borrow(&token.creation_name)
        } else {
            token.name
        }
    }

    public fun description(token_id: ObjectId): String acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).description
    }

    public fun is_description_mutable(token_id: ObjectId): bool acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).mutability_config.description
    }

    public fun is_name_mutable(token_id: ObjectId): bool acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).mutability_config.name
    }

    public fun is_uri_mutable(token_id: ObjectId): bool acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).mutability_config.uri
    }

    public fun name(token_id: ObjectId): String acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).name
    }

    public fun uri(token_id: ObjectId): String acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        borrow_global<Token>(object::object_id_address(&token_id)).uri
    }

    // Mutators

    public fun set_description(
        creator: &signer,
        token_id: ObjectId,
        description: String,
    ) acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        let token = borrow_global_mut<Token>(object::object_id_address(&token_id));
        assert!(
            token.creator == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            token.mutability_config.description,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        token.description = description;
    }

    public fun set_name(
        creator: &signer,
        token_id: ObjectId,
        name: String,
    ) acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        let token = borrow_global_mut<Token>(object::object_id_address(&token_id));
        assert!(
            token.creator == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            token.mutability_config.name,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );

        if (option::is_none(&token.creation_name)) {
            option::fill(&mut token.creation_name, token.name)
        };
        token.name = name;
    }

    public fun set_uri(
        creator: &signer,
        token_id: ObjectId,
        uri: String,
    ) acquires Token {
        assert!(
            exists<Token>(object::object_id_address(&token_id)),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        let token = borrow_global_mut<Token>(object::object_id_address(&token_id));
        assert!(
            token.creator == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            token.mutability_config.uri,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        token.uri = uri;
    }

    // Entry functions

    entry fun set_description_entry(
        creator: &signer,
        collection: String,
        name: String,
        description: String
    )  acquires Token {
        let token_id = create_token_id(&signer::address_of(creator), &collection, &name);
        set_description(creator, token_id, description);
    }

    #[test(creator = @0x123, trader = @0x456)]
    entry fun test_create_and_transfer(creator: &signer, trader: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        create_token_helper(creator, *&collection_name, *&token_name);

        let creator_address = signer::address_of(creator);
        let token_id = create_token_id(&creator_address, &collection_name, &token_name);
        assert!(object::owner(token_id) == creator_address, 1);
        object::transfer(creator, token_id, signer::address_of(trader));
        assert!(object::owner(token_id) == signer::address_of(trader), 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20000, location = token_objects::collection)]
    entry fun test_too_many_tokens(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        create_token_helper(creator, *&collection_name, token_name);
        create_token_helper(creator, collection_name, string::utf8(b"bad"));
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x80001, location = aptos_framework::object)]
    entry fun test_duplicate_tokens(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1);
        create_token_helper(creator, *&collection_name, *&token_name);
        create_token_helper(creator, collection_name, token_name);
    }

    #[test_only]
    entry fun create_collection_helper(creator: &signer, collection_name: String, max_supply: u64) {
        collection::create_collection(
            creator,
            string::utf8(b"collection description"),
            collection_name,
            string::utf8(b"collection uri"),
            false,
            false,
            max_supply,
            false,
            0,
            0,
            signer::address_of(creator),
        );
    }

    #[test_only]
    entry fun create_token_helper(creator: &signer, collection_name: String, token_name: String) {
        mint_token(
            creator,
            collection_name,
            string::utf8(b"token description"),
            token_name,
            string::utf8(b"token uri"),
            false,
            false,
            false,
            true,
            25,
            10000,
            signer::address_of(creator),
        );
    }
}
