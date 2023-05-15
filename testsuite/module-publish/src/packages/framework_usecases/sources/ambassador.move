module 0xABCD::ambassador {
    use std::error;
    use std::option;
    use std::string::{String};
    use std::signer;

    use aptos_framework::object::{Self, Object};
    use aptos_token_objects::collection;
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
    const ECOLLECTION_DOES_NOT_EXIST: u64 = 6;

    struct AmbassadorCollection has key {
        /// Used to mutate collection fields
        mutator_ref: collection::MutatorRef,
    }

    struct AmbassadorToken has key {
        /// Used to burn.
        burn_ref: token::BurnRef,
        /// Used to control freeze.
        transfer_ref: object::TransferRef,
        /// Used to mutate fields
        mutator_ref: token::MutatorRef,
    }

    struct AmbassadorLevel has key {
        ambassador_level: u64,
    }

    public entry fun create_ambassador_collection(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
    ) {
        create_ambassador_collection_internal(
            creator,
            description,
            name,
            uri,
        );
    }

    fun create_ambassador_collection_internal(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
    ) {
        let constructor_ref = collection::create_unlimited_collection(
            creator,
            description,
            name,
            option::none(),
            uri,
        );
        let object_signer = object::generate_signer(&constructor_ref);
        let mutator_ref = collection::generate_mutator_ref(&constructor_ref);
        let ambassador_collection = AmbassadorCollection {
            mutator_ref,
        };
        move_to(&object_signer, ambassador_collection);
    }

    public entry fun mint_ambassador_token(
        user: &signer,
        creator: &signer,
        collection_name: String,
        description: String,
        name: String,
        uri: String,
    ) {
        mint_ambassador_token_internal(
            creator,
            collection_name,
            description,
            name,
            uri,
            signer::address_of(user),
        );
    }

    fun mint_ambassador_token_internal(
        creator: &signer,
        collection_name: String,
        description: String,
        name: String,
        uri: String,
        soul_bound_to: address,
    ): Object<AmbassadorToken> {
        let constructor_ref = token::create_named_token(
            creator,
            collection_name,
            description,
            name,
            option::none(),
            uri,
        );
        let object_signer = object::generate_signer(&constructor_ref);
        let mutator_ref = token::generate_mutator_ref(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let burn_ref = token::generate_burn_ref(&constructor_ref);

        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, soul_bound_to);
        object::disable_ungated_transfer(&transfer_ref);

        let ambassador_token = AmbassadorToken {
            burn_ref,
            transfer_ref,
            mutator_ref,
        };
        move_to(&object_signer, ambassador_token);
        move_to(&object_signer, AmbassadorLevel { ambassador_level: 1 });

        object::object_from_constructor_ref<AmbassadorToken>(&constructor_ref)
    }

    public entry fun burn(creator: &signer, token: Object<AmbassadorToken>) acquires AmbassadorToken {
        authorize_creator(&token, creator);
        let ambassador_token = move_from<AmbassadorToken>(object::object_address(&token));
        let AmbassadorToken {
            burn_ref,
            transfer_ref: _,
            mutator_ref: _,
        } = ambassador_token;
        token::burn(burn_ref);
    }

    #[view]
    public fun ambassador_level(token: Object<AmbassadorToken>): u64 acquires AmbassadorLevel {
        let ambassador_level = borrow_global<AmbassadorLevel>(object::object_address(&token));
        ambassador_level.ambassador_level
    }

    public entry fun set_ambassador_level(
        token: Object<AmbassadorToken>,
        creator: &signer,
        new_ambassador_level: u64
    ) acquires AmbassadorLevel {
        authorize_creator(&token, creator);
        let ambassador_level = borrow_global_mut<AmbassadorLevel>(object::object_address(&token));
        ambassador_level.ambassador_level = new_ambassador_level;
    }

    inline fun authorize_creator<T: key>(token: &Object<T>, creator: &signer) {
        let token_address = object::object_address(token);
        assert!(
            exists<T>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        assert!(
            token::creator(*token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
    }

    inline fun authorized_borrow<T: key>(token: &Object<T>, creator: &signer): &AmbassadorToken {
        authorize_creator(token, creator);
        borrow_global<AmbassadorToken>(object::object_address(token))
    }

    #[test_only]
    use std::string;

    #[test(creator = @0x123, user1 = @0x456)]
    fun test_mint_burn(creator: &signer, user1: &signer) acquires AmbassadorToken, AmbassadorLevel {
        // -------------------------------------
        // Creator creates the Ambassador Collection.
        // -------------------------------------
        let collection_name = string::utf8(b"Ambassador Collection Name");
        let collection_description = string::utf8(b"Ambassador Collection Description");
        let collection_uri = string::utf8(b"Ambassador Collection URI");
        create_ambassador_collection_internal(creator, collection_description, collection_name, collection_uri);

        // -------------------------------------------
        // Creator mints a Ambassador token for User1.
        // -------------------------------------------
        let token_name = string::utf8(b"Ambassador Token 1");
        let token_description = string::utf8(b"description for Ambassador Token 1");
        let token_uri = string::utf8(b"uri for Ambassador Token 1");
        let user1_addr = signer::address_of(user1);
        let token = mint_ambassador_token_internal(
            creator,
            collection_name,
            token_description,
            token_name,
            token_uri,
            user1_addr,
        );
        assert!(object::owner(token) == user1_addr, 1);

        // ------------------------
        // Creator sets the level.
        // ------------------------
        assert!(ambassador_level(token) == 1, 2);
        set_ambassador_level(token, creator, 2);
        assert!(ambassador_level(token) == 2, 3);

        // ------------------------
        // Creator burns the token.
        // ------------------------
        let token_addr = object::object_address(&token);
        assert!(exists<AmbassadorToken>(token_addr), 4);
        burn(creator, token);
        assert!(!exists<AmbassadorToken>(token_addr), 5);
    }
}
