/// This defines an object-based Token. The key differentiating features from the Aptos standard
module token_v2::token {
    use std::error;
    use std::option::{Self, Option};
    use std::string::{Self, String};
    use std::signer;
    use token_v2::refs;

    /// The token does or does not exist
    const ETOKEN: u64 = 1;
    /// The provided signer is not the creator
    const ENOT_CREATOR: u64 = 2;
    /// Attempted to mutate an immutable field
    const EFIELD_NOT_MUTABLE: u64 = 3;
    /// The token indexer existence.
    const ETOKEN_INDEXER: u64 = 4;
    /// The existence of a specific token index in the token indexer.
    const ETOKEN_INDEX: u64 = 5;
    /// The provided signer is not the owner
    const ENOT_OWNER: u64 = 6;
    /// The existence of extend_ref
    const EEXTEND_REF: u64 = 9;
    /// The existence of transfer_ref
    const ETRANSFER_REF: u64 = 10;
    /// The existence of extend_ref
    const EDELETE_REF: u64 = 11;
    /// The token is not an NFT
    const ENOT_NFT: u64 = 12;

    use aptos_framework::object::{Self, Object, create_object_from_account, TransferRef, object_address, ExtendRef, generate_signer_for_extending, object_from_constructor_ref, address_to_object};
    use aptos_std::smart_table::{Self, SmartTable};
    use token_v2::common::{Royalty, assert_valid_name, init_fungible_asset_metadata, supply_new, assert_owner, assert_flags_length};
    use token_v2::collection::{Collection, get_collection_object};
    use token_v2::common;
    use token_v2::collection;
    use token_v2::refs::{Refs, refs_contain_extend, get_extend_from_refs, put_extend_to_refs, refs_contain_transfer, get_transfer_from_refs, put_transfer_to_refs, refs_contain_delete, get_delete_from_refs, borrow_extend_from_refs};
    use token_v2::coin_v2::mint_by_asset_owner;
    use std::signer::address_of;
    use std::vector;
    #[test_only]
    use token_v2::coin_v2::{balance_of, burn_by_asset_owner, transfer};
    #[test_only]
    use std::option::destroy_some;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields to all tokens.
    struct Token has key, drop {
        /// An optional categorization of similar token, there are no constraints on collections.
        collection: Option<Object<Collection>>,
        /// The original creator of this token.
        creator: address,
        /// A brief description of the token.
        description: String,
        /// Determines which fields are mutable.
        mutability_config: MutabilityConfig,
        /// The name of the token, which should be unique within the collection; the length of name
        /// should be smaller than 128, characters, eg: "Aptos Animal #1234"
        name: String,
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

    struct TokenIndexer has key {
        index: SmartTable<String, Refs>
    }

    struct OwnerRefs has key, drop {
        refs: Refs,
    }

    /// Create a token object and return its `ConstructurRef, which could be used to generate 3 storable object Refs for
    /// customized object control logic. It is not for general use cases.
    /// Drop the returned `ConstructorRef` unless you know what you are doing.
    public fun create_token(
        creator: &signer,
        collection_name: Option<String>,
        name: String,
        description: String,
        uri: String,
        mutability_config: MutabilityConfig,
        royalty: Option<Royalty>,
        creator_enabled_refs: vector<bool>, // extend, transfer, delete
        allow_owner_issue_fungible_coin: bool,
        allow_owner_burn: bool,
    ): Object<Token> acquires TokenIndexer {
        assert_valid_name(&name);

        let creator_ref = create_object_from_account(creator);
        let object_signer = object::generate_signer(&creator_ref);

        let creator_address = signer::address_of(creator);
        let token_index_key = generate_token_index_key(collection_name, name);
        let collection = if (option::is_some(&collection_name)) {
            let col_name = option::destroy_some(collection_name);
            collection::increment_supply(creator_address, col_name);
            option::some(get_collection_object(creator_address, col_name))
        } else {
            option::none()
        };

        // Put the refs into creator
        let creator_refs = refs::new_refs_from_constructor_ref(&creator_ref, creator_enabled_refs);
        assert!(exists<TokenIndexer>(creator_address), error::not_found(ETOKEN_INDEXER));
        let token_indexer = &mut borrow_global_mut<TokenIndexer>(creator_address).index;
        assert!(!smart_table::contains(token_indexer, name), error::already_exists(ETOKEN_INDEX));
        smart_table::add(token_indexer, token_index_key, creator_refs);

        let owner_refs = OwnerRefs {
            refs: refs::new_refs_from_constructor_ref(
                &creator_ref,
                vector[allow_owner_issue_fungible_coin, false, allow_owner_burn]
            )
        };

        let token = Token {
            collection,
            creator: creator_address,
            description,
            mutability_config,
            name,
            uri,
        };

        // All the resources in this object at token layer.
        move_to(&object_signer, token);
        move_to(&object_signer, owner_refs);
        if (option::is_some(&royalty)) {
            common::init_royalty(&object_signer, option::extract(&mut royalty))
        };
        object_from_constructor_ref<Token>(&creator_ref)
    }

    /// Indicate whether this `Token` object is an NFT or the asset of fungible tokens.
    public fun is_nft(token_obj: &Object<Token>): bool {
        !common::fungible_asset_metadata_exists(token_obj)
    }

    public fun token_exists(
        creator: address,
        collection_name: Option<String>,
        token_name: String
    ): bool acquires TokenIndexer {
        let token_index_key = generate_token_index_key(collection_name, token_name);
        let token_index = &borrow_global<TokenIndexer>(creator).index;
        smart_table::contains(token_index, token_index_key)
    }

    public fun get_token(
        creator: address,
        collection_name: Option<String>,
        token_name: String
    ): Object<Token> acquires TokenIndexer {
        let token_index_key = generate_token_index_key(collection_name, token_name);
        let token_index = &borrow_global<TokenIndexer>(creator).index;
        assert!(smart_table::contains(token_index, token_index_key), error::not_found(ETOKEN_INDEX));
        refs::generate_object_from_refs<Token>(smart_table::borrow(token_index, token_index_key))
    }

    inline fun generate_token_index_key(collection_name: Option<String>, token_name: String): String {
        assert_valid_name(&token_name);
        if (option::is_some(&collection_name)) {
            let name = option::destroy_some(collection_name);
            assert_valid_name(&name);
            string::append_utf8(&mut name, vector[0x0]);
            string::append(&mut name, token_name);
            name
        } else {
            token_name
        }
    }

    inline fun borrow_creator_refs(token_obj: &Object<Token>): &Refs acquires Token, TokenIndexer {
        let token_addr = verify(token_obj);
        let token = borrow_global<Token>(token_addr);
        let collection_name = option::map(token.collection, |obj| collection::name(obj));
        let token_index_key = generate_token_index_key(collection_name, token.name);
        let token_index = &borrow_global<TokenIndexer>(token.creator).index;
        assert!(smart_table::contains(token_index, token_index_key), error::not_found(ETOKEN_INDEX));
        smart_table::borrow(token_index, token_index_key)
    }

    inline fun borrow_creator_refs_mut(token_obj: &Object<Token>): &mut Refs acquires Token, TokenIndexer {
        let token_addr = verify(token_obj);
        let token = borrow_global<Token>(token_addr);
        let collection_name = option::map(token.collection, |obj| collection::name(obj));
        let token_index_key = generate_token_index_key(collection_name, token.name);
        let token_index = &mut borrow_global_mut<TokenIndexer>(token.creator).index;
        assert!(smart_table::contains(token_index, token_index_key), error::not_found(ETOKEN_INDEX));
        smart_table::borrow_mut(token_index, token_index_key)
    }

    fun remove_creator_refs(token_obj: &Object<Token>): Refs acquires Token, TokenIndexer {
        let token_addr = verify(token_obj);
        let token = borrow_global<Token>(token_addr);
        let collection_name = option::map(token.collection, |obj| {
            let name = collection::name(obj);
            // Removing creator_refs means deleting the token so if collection exists, we have to decrement collection
            // supply.
            collection::decrement_supply(token.creator, name);
            name
        });
        let token_index_key = generate_token_index_key(collection_name, token.name);
        let token_index = &mut borrow_global_mut<TokenIndexer>(token.creator).index;
        assert!(smart_table::contains(token_index, token_index_key), error::not_found(ETOKEN_INDEX));
        smart_table::remove(token_index, token_index_key)
    }

    inline fun assert_creator(creator: &signer, token_obj: &Object<Token>) acquires Token {
        let token_addr = verify(token_obj);
        let token = borrow_global<Token>(token_addr);
        assert!(token.creator == signer::address_of(creator), error::permission_denied(ENOT_CREATOR));
    }

    inline fun assert_nft(token_obj: &Object<Token>) {
        assert!(is_nft(token_obj), error::permission_denied(ENOT_NFT));
    }

    // ================================================================================================================
    // Owner control by OwnerRefs
    // ================================================================================================================
    /// Whether owner can convert a token(NFT) into an asset of fungible token.
    public fun owner_can_convert_to_ft(token_obj: &Object<Token>): bool acquires OwnerRefs {
        let addr = verify<Token>(token_obj);
        let owner_refs = &borrow_global<OwnerRefs>(addr).refs;
        refs_contain_extend(owner_refs)
    }

    /// Convert an NFT into an asset of fungible token. After calling this, `mint_fungible_coin` is allowed to call.
    /// After this call, owner will have 1 unit of coin in her account and she can also call `coin::mint_by_asset_owner`
    /// to mint more coins.
    /// Note: Owner can only do this when `OwnerRefs` has `ExtendRef` and for now it is irreversible.
    public fun convert_to_ft(
        owner: &signer,
        token_obj: &Object<Token>,
        max_supply: Option<u64>,
    ) acquires OwnerRefs {
        common::assert_fungible_asset_metadata_not_exists(token_obj);
        assert_owner(owner, token_obj);
        let owner_refs = &borrow_global<OwnerRefs>(object_address(token_obj)).refs;
        assert!(refs_contain_extend(owner_refs), error::not_found(EEXTEND_REF));
        let extend_ref = borrow_extend_from_refs(owner_refs);
        let token_obj_signer = generate_signer_for_extending(extend_ref);
        let supply = supply_new(max_supply);
        init_fungible_asset_metadata(&token_obj_signer, supply);
        // Mint one fungible coin to the `owner` as she owns the nft before.
        mint_by_asset_owner(owner, token_obj, 1, address_of(owner));
    }

    public fun owner_can_burn(token_obj: &Object<Token>): bool acquires OwnerRefs {
        let owner_refs = &borrow_global<OwnerRefs>(object_address(token_obj)).refs;
        refs_contain_delete(owner_refs)
    }

    public entry fun burn_by_owner(
        owner: &signer,
        token_address: address
    ) acquires OwnerRefs, Token, TokenIndexer {
        let token_obj = address_to_object<Token>(token_address);
        verify(&token_obj);
        // Deleting a token as a base of fungible asset is not allowed.
        assert_nft(&token_obj);
        assert_owner(owner, &token_obj);

        let token_addr = verify(&token_obj);
        let owner_refs = &mut borrow_global_mut<OwnerRefs>(token_addr).refs;
        assert!(refs_contain_delete(owner_refs), error::not_found(EDELETE_REF));
        let delete_ref = get_delete_from_refs(owner_refs);

        remove_creator_refs(&token_obj);

        // Remove token resources
        move_from<Token>(token_addr);
        move_from<OwnerRefs>(token_addr);
        if (common::royalty_exists(token_addr)) {
            common::remove_royalty(token_addr);
        };
        object::delete(delete_ref);
    }

    // ================================================================================================================
    // Check, Get, Put ExtendRef from CreatorRefs
    // ================================================================================================================
    public fun extend_ref_exists_in_creator_refs(token_obj: &Object<Token>): bool acquires Token, TokenIndexer {
        let creator_refs = borrow_creator_refs(token_obj);
        refs_contain_extend(creator_refs)
    }

    public fun get_extend_ref_by_creator(
        creator: &signer,
        token_obj: &Object<Token>
    ): ExtendRef acquires Token, TokenIndexer {
        assert_creator(creator, token_obj);
        let creator_refs = borrow_creator_refs_mut(token_obj);
        assert!(refs_contain_extend(creator_refs), error::not_found(EEXTEND_REF));
        get_extend_from_refs(creator_refs)
    }

    public fun put_extend_ref_by_creator(
        creator: &signer,
        token_obj: &Object<Token>,
        extend_ref: ExtendRef
    ) acquires Token, TokenIndexer {
        assert_creator(creator, token_obj);
        let creator_refs = borrow_creator_refs_mut(token_obj);
        put_extend_to_refs(creator_refs, extend_ref);
    }

    // ================================================================================================================
    // Check, Get, Put TransferRef from CreatorRefs
    // ================================================================================================================
    public fun transfer_ref_exists_in_creator_refs(token_obj: &Object<Token>): bool acquires Token, TokenIndexer {
        let creator_refs = borrow_creator_refs(token_obj);
        refs_contain_transfer(creator_refs)
    }

    public fun get_transfer_ref_from_creator_refs(
        creator: &signer,
        token_obj: &Object<Token>
    ): TransferRef acquires Token, TokenIndexer {
        assert_creator(creator, token_obj);
        let creator_refs = borrow_creator_refs_mut(token_obj);

        assert!(refs_contain_transfer(creator_refs), error::not_found(ETRANSFER_REF));
        get_transfer_from_refs(creator_refs)
    }

    public fun put_transfer_ref_to_creator_refs(
        creator: &signer,
        token_obj: &Object<Token>,
        transfer_ref: TransferRef
    ) acquires Token, TokenIndexer {
        assert_creator(creator, token_obj);
        let creator_refs = borrow_creator_refs_mut(token_obj);
        put_transfer_to_refs(creator_refs, transfer_ref);
    }


    // ================================================================================================================
    // Check, Use DeleteRef in CreatorRefs. Getting DeleteRef out is not allowed.
    // ================================================================================================================
    public fun creator_can_burn(token_address: address): bool acquires Token, TokenIndexer {
        let token_obj = address_to_object<Token>(token_address);
        verify(&token_obj);
        let creator_refs = borrow_creator_refs(&token_obj);
        refs_contain_delete(creator_refs)
    }

    public entry fun burn_by_creator(
        creator: &signer,
        token_address: address
    ) acquires Token, TokenIndexer, OwnerRefs {
        let token_obj = address_to_object<Token>(token_address);
        verify(&token_obj);

        // Deleting a token as a base of fungible asset is not allowed.
        assert_nft(&token_obj);
        assert_creator(creator, &token_obj);

        let token_addr = verify(&token_obj);
        // remove creator refs from token index
        let creator_refs = remove_creator_refs(&token_obj);
        assert!(refs_contain_delete(&creator_refs), error::not_found(EDELETE_REF));
        let delete_ref = get_delete_from_refs(&mut creator_refs);

        // Remove token resources
        move_from<Token>(token_addr);
        move_from<OwnerRefs>(token_addr);
        if (common::royalty_exists(token_addr)) {
            common::remove_royalty(token_addr);
        };
        object::delete(delete_ref);
    }

    public fun create_mutability_config(flags: &vector<bool>): MutabilityConfig {
        assert_flags_length(flags);
        let description = *vector::borrow(flags, 0);
        let name = *vector::borrow(flags, 1);
        let uri = *vector::borrow(flags, 2);
        MutabilityConfig { description, name, uri }
    }

    /// Simple token creation that generates a token and deposits it into the creators object store.
    /// For collection name, empty string means no collection associated with this token.
    public entry fun mint_token(
        creator: &signer,
        collection: String,
        name: String,
        description: String,
        uri: String,
        mutable_config_flags: vector<bool>,
        enable_royalty: bool,
        royalty_bps: u32,
        royalty_payee_address: address,
        creator_enabled_refs: vector<bool>, // extend, transfer, delete
        allow_owner_issue_fungible_coin: bool,
        allow_owner_burn: bool,
    ) acquires TokenIndexer {
        let mutability_config = create_mutability_config(&mutable_config_flags);

        let royalty = if (enable_royalty) {
            option::some(common::royalty_new(
                royalty_bps,
                royalty_payee_address,
            ))
        } else {
            option::none()
        };

        create_token(
            creator,
            if (string::is_empty(&collection)) { option::none() } else { option::some(collection) },
            name,
            description,
            uri,
            mutability_config,
            royalty,
            creator_enabled_refs,
            allow_owner_issue_fungible_coin,
            allow_owner_burn
        );
    }

    // Accessors
    inline fun verify<T: key>(token: &Object<T>): address {
        let token_address = object::object_address(token);
        assert!(
            exists<Token>(token_address),
            error::not_found(ETOKEN),
        );
        token_address
    }

    public fun creator<T: key>(token: &Object<T>): address acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).creator
    }

    public fun collection<T: key>(token: &Object<T>): Option<Object<Collection>> acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).collection
    }

    public fun description<T: key>(token: &Object<T>): String acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).description
    }

    public fun is_description_mutable<T: key>(token: &Object<T>): bool acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).mutability_config.description
    }

    public fun is_name_mutable<T: key>(token: &Object<T>): bool acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).mutability_config.name
    }

    public fun is_uri_mutable<T: key>(token: &Object<T>): bool acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).mutability_config.uri
    }

    public fun name<T: key>(token: &Object<T>): String acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).name
    }

    public fun uri<T: key>(token: &Object<T>): String acquires Token {
        let token_address = verify(token);
        borrow_global<Token>(token_address).uri
    }

    // Mutators
    public fun set_description<T: key>(
        creator: &signer,
        token: &Object<T>,
        description: String,
    ) acquires Token {
        let token_address = verify(token);
        let token = borrow_global_mut<Token>(token_address);
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

    public fun set_name<T: key>(
        creator: &signer,
        token: &Object<T>,
        name: String,
    ) acquires Token {
        let token_address = verify(token);
        let token = borrow_global_mut<Token>(token_address);
        assert!(
            token.creator == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        assert!(
            token.mutability_config.name,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );

        token.name = name;
    }

    public fun set_uri<T: key>(
        creator: &signer,
        token: &Object<T>,
        uri: String,
    ) acquires Token {
        let token_address = verify(token);
        let token = borrow_global_mut<Token>(token_address);
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

    #[test_only]
    entry fun create_collection_helper(
        creator: &signer,
        collection_name: String,
        max_supply: u64,
        royalty_bps: Option<u32>
    ) {
        let (enable_royalty, bps) = if (option::is_some(&royalty_bps)) {
            (true, destroy_some(royalty_bps))
        } else {
            (false, 0)
        };
        collection::create_collection(
            creator,
            collection_name,
            string::utf8(b"collection description"),
            string::utf8(b"collection uri"),
            false,
            false,
            max_supply,
            enable_royalty,
            bps,
            signer::address_of(creator),
        )
    }

    #[test_only]
    entry fun create_token_helper(
        creator: &signer,
        collection_name: Option<String>,
        token_name: String,
        royalty: Option<Royalty>,
    ): Object<Token> acquires TokenIndexer {
        create_token(
            creator,
            collection_name,
            token_name,
            string::utf8(b"token description"),
            string::utf8(b"token uri"),
            create_mutability_config(&vector[false, false, false]),
            royalty,
            vector[true, true, true],
            true,
            true
        )
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    entry fun test_create_and_transfer_and_burn(
        creator: &signer,
        aaron: &signer
    ) acquires Token, TokenIndexer, OwnerRefs {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        let creator_address = signer::address_of(creator);
        let expected_royalty = common::royalty_new(25, creator_address);

        create_collection_helper(creator, *&collection_name, 1, option::some(10));
        let token = create_token_helper(
            creator,
            option::some(*&collection_name),
            *&token_name,
            option::some(copy expected_royalty)
        );

        assert!(token == get_token(creator_address, option::some(*&collection_name), token_name), 1);
        assert!(object::owner(token) == creator_address, 2);
        assert!(creator(&token) == creator_address, 3);
        assert!(option::extract(&mut collection(&token)) == get_collection_object(creator_address, collection_name), 4);
        object::transfer(creator, token, signer::address_of(aaron));
        assert!(object::owner(token) == signer::address_of(aaron), 5);
        assert!(expected_royalty == common::get_royalty(object_address(&token)), 6);

        // May assert collection supply once it's ready.
        burn_by_owner(aaron, object_address(&token));
        assert!(!token_exists(creator_address, option::some(collection_name), token_name), 7);
    }

    #[test(creator = @0xcafe, aaron = @0xface)]
    entry fun test_conversion_to_ft(creator: &signer, aaron: &signer) acquires TokenIndexer, OwnerRefs {
        let token_name = string::utf8(b"UDSA");
        let creator_address = signer::address_of(creator);
        let aaron_address = signer::address_of(aaron);
        let token = create_token_helper(
            creator,
            option::none(),
            *&token_name,
            option::none(),
        );
        convert_to_ft(creator, &token, option::none());
        assert!(balance_of(creator_address, &token) == 1, 1);
        object::transfer(creator, token, signer::address_of(aaron));
        mint_by_asset_owner(aaron, &token, 100, aaron_address);
        transfer(aaron, &token, 100, creator_address);
        burn_by_asset_owner(aaron, &token, 101, creator_address);
    }

    #[test(creator = @0xcafe)]
    entry fun test_collection_royalty(creator: &signer) acquires TokenIndexer {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");
        let creator_address = signer::address_of(creator);

        create_collection_helper(creator, *&collection_name, 1, option::some(10));
        let token = create_token_helper(creator, option::some(*&collection_name), *&token_name, option::none());
        let expected_royalty = common::royalty_new(10, creator_address);
        assert!(expected_royalty == common::get_royalty(object_address(&token)), 2);
    }

    #[test(creator = @0xcafe)]
    entry fun test_no_royalty(creator: &signer) acquires TokenIndexer {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1, option::none());
        let token = create_token_helper(creator, option::some(*&collection_name), *&token_name, option::none());

        assert!(!common::royalty_exists(object_address(&token)), 1);
    }


    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x20001, location = token_v2::collection)]
    entry fun test_too_many_tokens(creator: &signer) acquires TokenIndexer {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1, option::none());
        create_token_helper(creator, option::some(*&collection_name), token_name, option::none());
        create_token_helper(creator, option::some(collection_name), string::utf8(b"bad"), option::none());
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x80005, location = aptos_framework::object)]
    entry fun test_duplicate_tokens(creator: &signer) acquires TokenIndexer {
        let collection_name = string::utf8(b"collection name");
        let token_name = string::utf8(b"token name");

        create_collection_helper(creator, *&collection_name, 1, option::none());
        create_token_helper(creator, option::some(*&collection_name), token_name, option::none());
        create_token_helper(creator, option::some(collection_name), *&token_name, option::none());
    }
}
