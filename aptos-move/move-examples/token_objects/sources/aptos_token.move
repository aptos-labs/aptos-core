/// This defines a minimally viable token for no-code solutions akin the the original token at
/// 0x3::token module.
/// The key features are:
/// * Base token and collection features
/// * Creator definable mutability for tokens
/// * Creator-based freezing of tokens
/// * Standard object-based transfer and events
///
/// TODO:
/// * Fungible of tokens
module token_objects::aptos_token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::signer;

    use aptos_framework::object::{Self, ConstructorRef, Object};

    use token_objects::collection;
    use token_objects::token;

    // The token does not exist
    const ETOKEN_DOES_NOT_EXIST: u64 = 1;
    /// The provided signer is not the creator
    const ENOT_CREATOR: u64 = 2;
    /// Attempted to mutate an immutable field
    const EFIELD_NOT_MUTABLE: u64 = 3;
    /// Attempted to burn a non-burnable token
    const ETOKEN_NOT_BURNABLE: u64 = 4;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Storage state for managing the no-code Token.
    struct AptosToken has key {
        /// Used to burn.
        burn_ref: Option<token::BurnRef>,
        /// Used to control freeze.
        transfer_ref: Option<object::TransferRef>,
        /// Used to mutate fields
        mutator_ref: Option<token::MutatorRef>,
        /// Determines if the creator can mutate the description
        mutable_description: bool,
        /// Determines if the creator can mutate the name
        mutable_name: bool,
        /// Determines if the creator can mutate the uri
        mutable_uri: bool,
        /// Determines if the creator can freeze this NFT
        freezable_by_creator: bool,
    }

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
        );

        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
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

        let aptos_token = AptosToken {
            burn_ref,
            transfer_ref: option::none(),
            mutator_ref,
            mutable_description,
            mutable_name,
            mutable_uri,
            freezable_by_creator: false,
        };
        move_to(&object_signer, aptos_token);

        constructor_ref
    }

    // Accessors

    inline fun verify<T: key>(token: &Object<T>): address {
        let token_address = object::object_address(token);
        assert!(
            exists<AptosToken>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );
        token_address
    }

    public fun is_burnable_by_creator<T: key>(token: Object<T>): bool acquires AptosToken {
        let token_address = verify(&token);
        option::is_some(&borrow_global<AptosToken>(token_address).burn_ref)
    }

    public fun is_freezable_by_creator<T: key>(token: Object<T>): bool acquires AptosToken {
        let token_address = verify(&token);
        borrow_global<AptosToken>(token_address).freezable_by_creator
    }

    public fun is_mutable_description<T: key>(token: Object<T>): bool acquires AptosToken {
        let token_address = verify(&token);
        borrow_global<AptosToken>(token_address).mutable_description
    }

    public fun is_mutable_name<T: key>(token: Object<T>): bool acquires AptosToken {
        let token_address = verify(&token);
        borrow_global<AptosToken>(token_address).mutable_name
    }

    public fun is_mutable_uri<T: key>(token: Object<T>): bool acquires AptosToken {
        let token_address = verify(&token);
        borrow_global<AptosToken>(token_address).mutable_uri
    }

    // Mutators

    public fun burn<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let token_address = verify(&token);
        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            option::is_some(&aptos_token.burn_ref),
            error::permission_denied(ETOKEN_NOT_BURNABLE),
        );
    }

    public fun freeze_transfer<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let token_address = verify(&token);
        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            aptos_token.freezable_by_creator,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        object::disable_ungated_transfer(option::borrow(&aptos_token.transfer_ref));
    }

    public fun unfreeze_transfer<T: key>(creator: &signer, token: Object<T>) acquires AptosToken {
        let token_address = verify(&token);
        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
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
        let token_address = verify(&token);
        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
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
        let token_address = verify(&token);
        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
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
        let token_address = verify(&token);
        let aptos_token = borrow_global_mut<AptosToken>(token_address);
        assert!(
            token::creator(token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            aptos_token.mutable_uri,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );
        token::set_uri(option::borrow(&aptos_token.mutator_ref), uri);
    }

    // Entry functions

    entry fun burn_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosToken {
        let token_addr = token::create_token_address(&signer::address_of(creator), &collection, &name);
        let token = object::address_to_object<AptosToken>(token_addr);
        burn(creator, token);
    }

    entry fun freeze_transfer_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosToken {
        let token_addr = token::create_token_address(&signer::address_of(creator), &collection, &name);
        let token = object::address_to_object<AptosToken>(token_addr);
        freeze_transfer(creator, token);
    }

    entry fun unfreeze_transfer_call(
        creator: &signer,
        collection: String,
        name: String,
    ) acquires AptosToken {
        let token_addr = token::create_token_address(&signer::address_of(creator), &collection, &name);
        let token = object::address_to_object<AptosToken>(token_addr);
        unfreeze_transfer(creator, token);
    }

    entry fun set_description_call(
        creator: &signer,
        collection: String,
        name: String,
        description: String
    )  acquires AptosToken {
        let token_addr = token::create_token_address(&signer::address_of(creator), &collection, &name);
        let token = object::address_to_object<AptosToken>(token_addr);
        set_description(creator, token, description);
    }

    entry fun set_name_call(
        creator: &signer,
        collection: String,
        original_name: String,
        new_name: String
    )  acquires AptosToken {
        let token_addr = token::create_token_address(&signer::address_of(creator), &collection, &original_name);
        let token = object::address_to_object<AptosToken>(token_addr);
        set_name(creator, token, new_name);
    }

    entry fun set_uri_call(
        creator: &signer,
        collection: String,
        name: String,
        uri: String
    )  acquires AptosToken {
        let token_addr = token::create_token_address(&signer::address_of(creator), &collection, &name);
        let token = object::address_to_object<AptosToken>(token_addr);
        set_uri(creator, token, uri);
    }

    // Tests

    #[test(creator = @0x123)]
    fun test_create_and_transfer(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);

        assert!(object::owner(token) == signer::address_of(creator), 1);
        object::transfer(creator, token, @0x345);
        assert!(object::owner(token) == @0x345, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = object)]
    fun test_mint_soul_bound(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
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
        );

        let token_addr = token::create_token_address(
            &signer::address_of(creator),
            &collection_name,
            &token_name,
        );
        let token = object::address_to_object<AptosToken>(token_addr);
        object::transfer(creator, token, @0x345);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50003, location = object)]
    fun test_frozen_transfer(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer_call(creator, collection_name, token_name);
        object::transfer(creator, token, @0x345);
    }

    #[test(creator = @0x123)]
    fun test_unfrozen_transfer(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer(another, token);
    }

    #[test(creator = @0x123, another = @0x456)]
    #[expected_failure(abort_code = 0x50002, location = Self)]
    fun test_noncreator_unfreeze(creator: &signer, another: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);
        freeze_transfer_call(creator, collection_name, token_name);
        unfreeze_transfer(another, token);
    }

    #[test(creator = @0x123)]
    fun test_set_description(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);

        let description = string::utf8(b"not");
        set_description(noncreator, token, description);
    }

    #[test(creator = @0x123)]
    fun test_set_name(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);

        let name = string::utf8(b"not");
        set_name(noncreator, token, name);
    }

    #[test(creator = @0x123)]
    fun test_set_uri(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);

        let uri = string::utf8(b"not");
        set_uri(noncreator, token, uri);
    }

    #[test(creator = @0x123)]
    entry fun test_burnable(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);

        burn(creator, token);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x50004, location = Self)]
    entry fun test_not_burnable(creator: &signer) acquires AptosToken {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, collection_name);
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

        create_collection_helper(creator, collection_name);
        let token = mint_helper(creator, collection_name, token_name, true);

        burn(noncreator, token);
    }

    #[test_only]
    fun create_collection_helper(creator: &signer, collection_name: String) {
        collection::create_collection(
            creator,
            string::utf8(b"collection description"),
            collection_name,
            string::utf8(b"collection uri"),
            1,
            false,
            0,
            0,
            signer::address_of(creator),
        );
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
        );

        let token_addr = token::create_token_address(
            &signer::address_of(creator),
            &collection_name,
            &token_name,
        );
        object::address_to_object<AptosToken>(token_addr)
    }
}
