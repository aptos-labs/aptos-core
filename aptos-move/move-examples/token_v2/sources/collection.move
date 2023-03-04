/// This defines an object-based Collection. A collection acts as a set organizer for a group of
/// tokens. This includes aspects such as a general description, project URI, name, and may contain
/// other useful generalizations across this set of tokens.
///
/// Being built upon objects enables collections to be relatively flexible. As core primitives it
/// supports:
/// * Common fields: name, uri, description, creator
/// * A mutability config for uri and description
/// * Optional support for collection-wide royalties
/// * Optional support for tracking of supply
///
/// This collection does not directly support:
/// * Events on mint or burn -- that's left to the collection creator.
///
/// TODO:
/// * Add Royalty reading and consider mutation
/// * Consider supporting changing the name of the collection.
/// * Consider supporting changing the aspects of supply
/// * Add aggregator support when added to framework
/// * Update ObjectId to be an acceptable param to move
module token_v2::collection {
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::String;

    use aptos_framework::object::{Self, Object, generate_transfer_ref, object_address, disable_ungated_transfer};
    use aptos_std::smart_table::SmartTable;
    use aptos_std::smart_table;
    use token_v2::common::{Royalty, royalty_new, royalty_exists, remove_royalty, assert_valid_name};
    use token_v2::refs::{Refs, new_refs_from_constructor_ref, generate_object_from_refs, get_delete_from_refs, address_of_refs};
    #[test_only]
    use std::string;
    #[test_only]
    use std::signer::address_of;
    use token_v2::common;

    friend token_v2::token;

    /// The collections supply is at its maximum amount.
    const EEXCEEDS_MAX_SUPPLY: u64 = 1;
    /// The error of collection refs.
    const ECOLLECTION_REFS: u64 = 2;
    /// The provided signer is not the creator.
    const ENOT_CREATOR: u64 = 3;
    /// Attempted to mutate an immutable field.
    const EFIELD_NOT_MUTABLE: u64 = 4;
    /// The error of collection resource.
    const ECOLLECTION: u64 = 5;
    /// The error of collection index.
    const ECOLLECTION_INDEX: u64 = 6;
    /// The error of collection name.
    const ECOLLECTION_NAME: u64 = 7;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields for a collection.
    struct Collection has key {
        /// The creator of this collection.
        creator: address,
        /// A brief description of the collection.
        description: String,
        /// Determines which fields are mutable.
        mutability_config: MutabilityConfig,
        /// An optional categorization of similar token.
        name: String,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
    }

    /// This config specifies which fields in the TokenData are mutable
    struct MutabilityConfig has copy, drop, store {
        description: bool,
        uri: bool,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Aggregable supply tracker, this is can be used for maximum parallel minting but only for
    /// for uncapped mints. Currently disabled until this library is in the framework.
    struct AggregableSupply has key {}

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.
    struct FixedSupply has key, drop {
        current_supply: u64,
        max_supply: u64,
    }

    struct CollectionIndex has key {
        index: SmartTable<String, Refs>
    }

    public fun ensure_collection_index(creator: &signer) {
        if (!exists<CollectionIndex>(signer::address_of(creator))) {
            move_to(creator, CollectionIndex {
                index: smart_table::new()
            })
        }
    }

    fun create_collection_object_signer(
        creator: &signer,
        name: String
    ): (signer, Object<Collection>) acquires CollectionIndex {
        ensure_collection_index(creator);
        let collection_index = &mut borrow_global_mut<CollectionIndex>(signer::address_of(creator)).index;
        // Ensure the collection index does not have an index for `name`.
        assert!(!smart_table::contains(collection_index, name), error::already_exists(ECOLLECTION_NAME));
        let creator_ref = object::create_object_from_account(creator);
        // For collection we only allow extend and delete. So we disable transfer_ref and ungated_transfer.
        // Therefore, the creator and owner of a collection are always the same.
        smart_table::add(
            collection_index,
            name,
            new_refs_from_constructor_ref((&creator_ref), vector[true, false, true])
        );
        disable_ungated_transfer(&generate_transfer_ref(&creator_ref));
        (object::generate_signer(&creator_ref), object::object_from_constructor_ref<Collection>(&creator_ref))
    }

    fun collection_new(
        creator: address,
        name: String,
        description: String,
        uri: String,
        mutability_config: MutabilityConfig
    ): Collection {
        assert_valid_name(&name);
        Collection {
            creator,
            name,
            description,
            uri,
            mutability_config,
        }
    }

    public fun create_fixed_collection(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        mutability_config: MutabilityConfig,
        max_supply: u64,
        royalty: Option<Royalty>,
    ): Object<Collection> acquires CollectionIndex {
        let (object_signer, collection_obj) = create_collection_object_signer(creator, name);

        let collection = collection_new(signer::address_of(creator), name, description, uri, mutability_config);
        move_to(&object_signer, collection);

        let supply = FixedSupply {
            current_supply: 0,
            max_supply,
        };
        move_to(&object_signer, supply);

        if (option::is_some(&royalty)) {
            common::init_royalty(&object_signer, option::extract(&mut royalty))
        };
        collection_obj
    }

    public fun create_aggregable_collection(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        mutability_config: MutabilityConfig,
        royalty: Option<Royalty>,
    ): Object<Collection> acquires CollectionIndex {
        assert_valid_name(&name);
        let (object_signer, collection_obj) = create_collection_object_signer(creator, name);

        let collection = collection_new(signer::address_of(creator), name, description, uri, mutability_config);
        move_to(&object_signer, collection);

        let supply = AggregableSupply {};
        move_to(&object_signer, supply);

        if (option::is_some(&royalty)) {
            common::init_royalty(&object_signer, option::extract(&mut royalty))
        };
        collection_obj
    }

    inline fun borrow_collection_index_mut(creator: address): &mut SmartTable<String, Refs> acquires CollectionIndex {
        assert!(exists<CollectionIndex>(creator), error::not_found(ECOLLECTION_INDEX));
        &mut borrow_global_mut<CollectionIndex>(creator).index
    }

    public fun has_collection(creator: address, name: String): bool acquires CollectionIndex {
        let collection_index = borrow_collection_index_mut(creator);
        smart_table::contains(collection_index, name)
    }

    fun remove_collection_refs(creator: address, name: String): Refs acquires CollectionIndex {
        let collection_index = borrow_collection_index_mut(creator);
        assert!(smart_table::contains(collection_index, *&name), error::not_found(ECOLLECTION_REFS));
        smart_table::remove(collection_index, name)
    }

    public fun get_collection_object(creator: address, name: String): Object<Collection> acquires CollectionIndex {
        let collection_index = borrow_collection_index_mut(creator);
        assert!(smart_table::contains(collection_index, *&name), error::not_found(ECOLLECTION_REFS));
        generate_object_from_refs<Collection>(smart_table::borrow(collection_index, name))
    }

    public fun create_mutability_config(description: bool, uri: bool): MutabilityConfig {
        MutabilityConfig { description, uri }
    }

    public(friend) fun increment_supply(creator: address, name: String) acquires FixedSupply, CollectionIndex {
        let collection_addr = object_address(&get_collection_object(creator, name));
        if (exists<FixedSupply>(collection_addr)) {
            let supply = borrow_global_mut<FixedSupply>(collection_addr);
            supply.current_supply = supply.current_supply + 1;
            assert!(
                supply.current_supply <= supply.max_supply,
                error::out_of_range(EEXCEEDS_MAX_SUPPLY),
            );
        }
    }

    public(friend) fun decrement_supply(creator: address, name: String) acquires FixedSupply, CollectionIndex {
        let collection_addr = object_address(&get_collection_object(creator, name));
        if (exists<FixedSupply>(collection_addr)) {
            let supply = borrow_global_mut<FixedSupply>(collection_addr);
            supply.current_supply = supply.current_supply - 1;
        }
    }

    /// Entry function for creating a collection
    public entry fun create_collection(
        creator: &signer,
        name: String,
        description: String,
        uri: String,
        mutable_description: bool,
        mutable_uri: bool,
        max_supply: u64,
        enable_royalty: bool,
        royalty_bps: u32,
        royalty_payee_address: address,
    ) acquires CollectionIndex {
        let mutability_config = create_mutability_config(mutable_description, mutable_uri);
        let royalty = if (enable_royalty) {
            option::some(royalty_new(royalty_bps, royalty_payee_address))
        } else {
            option::none()
        };

        if (max_supply == 0) {
            create_aggregable_collection(
                creator,
                name,
                description,
                uri,
                mutability_config,
                royalty,
            )
        } else {
            create_fixed_collection(
                creator,
                name,
                description,
                uri,
                mutability_config,
                max_supply,
                royalty,
            )
        };
    }

    public fun delete_collection(
        creator: &signer,
        name: String,
    ) acquires CollectionIndex, FixedSupply {
        let collection_refs = remove_collection_refs(signer::address_of(creator), name);
        let collection_addr = address_of_refs(&collection_refs);
        if (exists<FixedSupply>(collection_addr)) {
            let supply = borrow_global<FixedSupply>(collection_addr);
            assert!(supply.current_supply == 0, 0);
            move_from<FixedSupply>(collection_addr);
        } else {
            // delete aggregatable supply
        };
        if (royalty_exists(collection_addr)) {
            remove_royalty(collection_addr);
        };
        object::delete(get_delete_from_refs(&mut collection_refs));
    }

    // Accessors
    inline fun verify<T: key>(collection: &Object<T>): address {
        let collection_address = object::object_address(collection);
        assert!(
            exists<Collection>(collection_address),
            error::not_found(ECOLLECTION),
        );
        collection_address
    }

    public fun creator<T: key>(collection: Object<T>): address acquires Collection {
        let collection_address = verify(&collection);
        borrow_global<Collection>(collection_address).creator
    }

    public fun description<T: key>(collection: Object<T>): String acquires Collection {
        let collection_address = verify(&collection);
        borrow_global<Collection>(collection_address).description
    }

    public fun is_description_mutable<T: key>(collection: Object<T>): bool acquires Collection {
        let collection_address = verify(&collection);
        borrow_global<Collection>(collection_address).mutability_config.description
    }

    public fun is_uri_mutable<T: key>(collection: Object<T>): bool acquires Collection {
        let collection_address = verify(&collection);
        borrow_global<Collection>(collection_address).mutability_config.uri
    }

    public fun name<T: key>(collection: Object<T>): String acquires Collection {
        let collection_address = verify(&collection);
        borrow_global<Collection>(collection_address).name
    }

    public fun uri<T: key>(collection: Object<T>): String acquires Collection {
        let collection_address = verify(&collection);
        borrow_global<Collection>(collection_address).uri
    }

    // Mutators

    public fun set_description<T: key>(
        creator: &signer,
        collection: Object<T>,
        description: String,
    ) acquires Collection {
        let collection_address = verify(&collection);
        let collection = borrow_global_mut<Collection>(collection_address);
        assert!(
            collection.creator == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );

        assert!(
            collection.mutability_config.description,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );

        collection.description = description;
    }

    public fun set_uri<T: key>(
        creator: &signer,
        collection: Object<T>,
        uri: String,
    ) acquires Collection {
        let collection_address = verify(&collection);
        let collection = borrow_global_mut<Collection>(collection_address);
        assert!(
            collection.creator == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );

        assert!(
            collection.mutability_config.uri,
            error::permission_denied(EFIELD_NOT_MUTABLE),
        );

        collection.uri = uri;
    }

    #[test(creator = @0xcafe)]
    entry fun test_create_and_deletion(creator: &signer) acquires CollectionIndex, FixedSupply {
        let creator_address = signer::address_of(creator);
        let collection_name = string::utf8(b"collection name");
        create_immutable_collection_helper(creator, *&collection_name);

        let collection = get_collection_object(creator_address, *&collection_name);
        assert!(object::owner(collection) == creator_address, 0);
        delete_collection(creator, collection_name);
        assert!(!has_collection(signer::address_of(creator), collection_name), 0);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x80007, location = aptos_framework::object)]
    entry fun test_duplicate_collection(creator: &signer) acquires CollectionIndex {
        let collection_name = string::utf8(b"collection name");
        create_immutable_collection_helper(creator, *&collection_name);
        create_immutable_collection_helper(creator, collection_name);
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    entry fun test_immutable_set_description(creator: &signer) acquires Collection, CollectionIndex {
        let collection_name = string::utf8(b"collection name");
        create_immutable_collection_helper(creator, *&collection_name);
        let collection = get_collection_object(address_of(creator), collection_name);
        set_description(creator, collection, string::utf8(b"fail"));
    }

    #[test(creator = @0xcafe)]
    #[expected_failure(abort_code = 0x50003, location = Self)]
    entry fun test_immutable_set_uri(creator: &signer) acquires Collection, CollectionIndex {
        let collection_name = string::utf8(b"collection name");
        create_immutable_collection_helper(creator, *&collection_name);
        let collection = get_collection_object(address_of(creator), collection_name);
        set_uri(creator, collection, string::utf8(b"fail"));
    }

    #[test(creator = @0xcafe)]
    entry fun test_mutable_set_description(creator: &signer) acquires Collection, CollectionIndex {
        let collection_name = string::utf8(b"collection name");
        create_mutable_collection_helper(creator, *&collection_name);
        let collection = get_collection_object(address_of(creator), collection_name);
        let description = string::utf8(b"no fail");
        assert!(description != description(collection), 0);
        set_description(creator, collection, *&description);
        assert!(description == description(collection), 1);
    }

    #[test(creator = @0xcafe)]
    entry fun test_mutable_set_uri(creator: &signer) acquires Collection, CollectionIndex {
        let collection_name = string::utf8(b"collection name");
        create_mutable_collection_helper(creator, *&collection_name);
        let collection = get_collection_object(address_of(creator), collection_name);
        let uri = string::utf8(b"no fail");
        assert!(uri != uri(collection), 0);
        set_uri(creator, collection, *&uri);
        assert!(uri == uri(collection), 1);
    }

    #[test_only]
    fun create_immutable_collection_helper(creator: &signer, name: String) acquires CollectionIndex {
        create_collection(
            creator,
            string::utf8(b"collection description"),
            name,
            string::utf8(b"collection uri"),
            false,
            false,
            1,
            true,
            10,
            signer::address_of(creator),
        );
    }

    #[test_only]
    fun create_mutable_collection_helper(creator: &signer, name: String) acquires CollectionIndex {
        create_collection(
            creator,
            string::utf8(b"collection description"),
            name,
            string::utf8(b"collection uri"),
            true,
            true,
            1,
            true,
            10,
            signer::address_of(creator),
        );
    }
}
