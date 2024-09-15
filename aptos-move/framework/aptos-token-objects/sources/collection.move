/// This defines an object-based Collection. A collection acts as a set organizer for a group of
/// tokens. This includes aspects such as a general description, project URI, name, and may contain
/// other useful generalizations across this set of tokens.
///
/// Being built upon objects enables collections to be relatively flexible. As core primitives it
/// supports:
/// * Common fields: name, uri, description, creator
/// * MutatorRef leaving mutability configuration to a higher level component
/// * Addressed by a global identifier of creator's address and collection name, thus collections
///   cannot be deleted as a restriction of the object model.
/// * Optional support for collection-wide royalties
/// * Optional support for tracking of supply with events on mint or burn
///
/// TODO:
/// * Consider supporting changing the name of the collection with the MutatorRef. This would
///   require adding the field original_name.
/// * Consider supporting changing the aspects of supply with the MutatorRef.
/// * Add aggregator support when added to framework
module aptos_token_objects::collection {
    use std::error;
    use std::features;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{Self, String};
    use aptos_framework::aggregator_v2::{Self, Aggregator, AggregatorSnapshot};
    use aptos_framework::event;
    use aptos_framework::object::{Self, ConstructorRef, ExtendRef, Object};

    use aptos_token_objects::royalty::{Self, Royalty};

    friend aptos_token_objects::token;

    /// The collection does not exist
    const ECOLLECTION_DOES_NOT_EXIST: u64 = 1;
    /// The collection has reached its supply and no more tokens can be minted, unless some are burned
    const ECOLLECTION_SUPPLY_EXCEEDED: u64 = 2;
    /// The collection name is over the maximum length
    const ECOLLECTION_NAME_TOO_LONG: u64 = 3;
    /// The URI is over the maximum length
    const EURI_TOO_LONG: u64 = 4;
    /// The description is over the maximum length
    const EDESCRIPTION_TOO_LONG: u64 = 5;
    /// The max supply must be positive
    const EMAX_SUPPLY_CANNOT_BE_ZERO: u64 = 6;
    /// Concurrent feature flag is not yet enabled, so the function cannot be performed
    const ECONCURRENT_NOT_ENABLED: u64 = 7;
    /// Tried upgrading collection to concurrent, but collection is already concurrent
    const EALREADY_CONCURRENT: u64 = 8;
    /// The new max supply cannot be less than the current supply
    const EINVALID_MAX_SUPPLY: u64 = 9;
    /// The collection does not have a max supply
    const ENO_MAX_SUPPLY_IN_COLLECTION: u64 = 10;
    /// The collection owner feature is not supported
    const ECOLLECTION_OWNER_NOT_SUPPORTED: u64 = 11;

    const MAX_COLLECTION_NAME_LENGTH: u64 = 128;
    const MAX_URI_LENGTH: u64 = 512;
    const MAX_DESCRIPTION_LENGTH: u64 = 2048;

    const MAX_U64: u64 = 18446744073709551615;

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Represents the common fields for a collection.
    struct Collection has key {
        /// The creator of this collection.
        creator: address,
        /// A brief description of the collection.
        description: String,
        /// An optional categorization of similar token.
        name: String,
        /// The Uniform Resource Identifier (uri) pointing to the JSON file stored in off-chain
        /// storage; the URL length will likely need a maximum any suggestions?
        uri: String,
        /// Emitted upon any mutation of the collection.
        mutation_events: event::EventHandle<MutationEvent>,
    }

    /// This enables mutating description and URI by higher level services.
    struct MutatorRef has drop, store {
        self: address,
    }

    /// Contains the mutated fields name. This makes the life of indexers easier, so that they can
    /// directly understand the behavior in a writeset.
    struct MutationEvent has drop, store {
        mutated_field_name: String,
    }

    #[event]
    /// Contains the mutated fields name. This makes the life of indexers easier, so that they can
    /// directly understand the behavior in a writeset.
    struct Mutation has drop, store {
        mutated_field_name: String,
        collection: Object<Collection>,
        old_value: String,
        new_value: String,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Fixed supply tracker, this is useful for ensuring that a limited number of tokens are minted.
    /// and adding events and supply tracking to a collection.
    struct FixedSupply has key {
        /// Total minted - total burned
        current_supply: u64,
        max_supply: u64,
        total_minted: u64,
        /// Emitted upon burning a Token.
        burn_events: event::EventHandle<BurnEvent>,
        /// Emitted upon minting an Token.
        mint_events: event::EventHandle<MintEvent>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Unlimited supply tracker, this is useful for adding events and supply tracking to a collection.
    struct UnlimitedSupply has key {
        current_supply: u64,
        total_minted: u64,
        /// Emitted upon burning a Token.
        burn_events: event::EventHandle<BurnEvent>,
        /// Emitted upon minting an Token.
        mint_events: event::EventHandle<MintEvent>,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// Supply tracker, useful for tracking amount of issued tokens.
    /// If max_value is not set to U64_MAX, this ensures that a limited number of tokens are minted.
    struct ConcurrentSupply has key {
        /// Total minted - total burned
        current_supply: Aggregator<u64>,
        total_minted: Aggregator<u64>,
    }

    struct BurnEvent has drop, store {
        index: u64,
        token: address,
    }

    struct MintEvent has drop, store {
        index: u64,
        token: address,
    }

    #[event]
    struct Burn has drop, store {
        collection: address,
        index: u64,
        token: address,
        previous_owner: address,
    }

    #[event]
    struct Mint has drop, store {
        collection: address,
        index: AggregatorSnapshot<u64>,
        token: address,
    }

    // DEPRECATED, NEVER USED
    #[deprecated]
    #[event]
    struct ConcurrentBurnEvent has drop, store {
        collection_addr: address,
        index: u64,
        token: address,
    }

    // DEPRECATED, NEVER USED
    #[deprecated]
    #[event]
    struct ConcurrentMintEvent has drop, store {
        collection_addr: address,
        index: AggregatorSnapshot<u64>,
        token: address,
    }

    #[event]
    struct SetMaxSupply has drop, store {
        collection: Object<Collection>,
        old_max_supply: u64,
        new_max_supply: u64,
    }

    /// Creates a fixed-sized collection, or a collection that supports a fixed amount of tokens.
    /// This is useful to create a guaranteed, limited supply on-chain digital asset. For example,
    /// a collection 1111 vicious vipers. Note, creating restrictions such as upward limits results
    /// in data structures that prevent Aptos from parallelizing mints of this collection type.
    /// Beyond that, it adds supply tracking with events.
    public fun create_fixed_collection(
        creator: &signer,
        description: String,
        max_supply: u64,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        assert!(max_supply != 0, error::invalid_argument(EMAX_SUPPLY_CANNOT_BE_ZERO));
        let collection_seed = create_collection_seed(&name);
        let constructor_ref = object::create_named_object(creator, collection_seed);

        let supply = ConcurrentSupply {
            current_supply: aggregator_v2::create_aggregator(max_supply),
            total_minted: aggregator_v2::create_unbounded_aggregator(),
        };

        create_collection_internal(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::some(supply),
        )
    }

    /// Same functionality as `create_fixed_collection`, but the caller is the owner of the collection.
    /// This means that the caller can transfer the collection to another address.
    /// This transfers ownership and minting permissions to the new address.
    public fun create_fixed_collection_as_owner(
        creator: &signer,
        description: String,
        max_supply: u64,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        assert!(features::is_collection_owner_enabled(), error::unavailable(ECOLLECTION_OWNER_NOT_SUPPORTED));

        let constructor_ref = create_fixed_collection(
            creator,
            description,
            max_supply,
            name,
            royalty,
            uri,
        );
        enable_ungated_transfer(&constructor_ref);
        constructor_ref
    }

    /// Creates an unlimited collection. This has support for supply tracking but does not limit
    /// the supply of tokens.
    public fun create_unlimited_collection(
        creator: &signer,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let collection_seed = create_collection_seed(&name);
        let constructor_ref = object::create_named_object(creator, collection_seed);

        let supply = ConcurrentSupply {
            current_supply: aggregator_v2::create_unbounded_aggregator(),
            total_minted: aggregator_v2::create_unbounded_aggregator(),
        };

        create_collection_internal(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::some(supply),
        )
    }

    /// Same functionality as `create_unlimited_collection`, but the caller is the owner of the collection.
    /// This means that the caller can transfer the collection to another address.
    /// This transfers ownership and minting permissions to the new address.
    public fun create_unlimited_collection_as_owner(
        creator: &signer,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        assert!(features::is_collection_owner_enabled(), error::unavailable(ECOLLECTION_OWNER_NOT_SUPPORTED));

        let constructor_ref = create_unlimited_collection(
            creator,
            description,
            name,
            royalty,
            uri,
        );
        enable_ungated_transfer(&constructor_ref);
        constructor_ref
    }

    /// Creates an untracked collection, or a collection that supports an arbitrary amount of
    /// tokens. This is useful for mass airdrops that fully leverage Aptos parallelization.
    /// TODO: Hide this until we bring back meaningful way to enforce burns
    fun create_untracked_collection(
        creator: &signer,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
    ): ConstructorRef {
        let collection_seed = create_collection_seed(&name);
        let constructor_ref = object::create_named_object(creator, collection_seed);

        create_collection_internal<FixedSupply>(
            creator,
            constructor_ref,
            description,
            name,
            royalty,
            uri,
            option::none(),
        )
    }

    inline fun create_collection_internal<Supply: key>(
        creator: &signer,
        constructor_ref: ConstructorRef,
        description: String,
        name: String,
        royalty: Option<Royalty>,
        uri: String,
        supply: Option<Supply>,
    ): ConstructorRef {
        assert!(string::length(&name) <= MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));
        assert!(string::length(&uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
        assert!(string::length(&description) <= MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));

        let object_signer = object::generate_signer(&constructor_ref);

        let collection = Collection {
            creator: signer::address_of(creator),
            description,
            name,
            uri,
            mutation_events: object::new_event_handle(&object_signer),
        };
        move_to(&object_signer, collection);

        if (option::is_some(&supply)) {
            move_to(&object_signer, option::destroy_some(supply))
        } else {
            option::destroy_none(supply)
        };

        if (option::is_some(&royalty)) {
            royalty::init(&constructor_ref, option::extract(&mut royalty))
        };

        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        object::disable_ungated_transfer(&transfer_ref);

        constructor_ref
    }

    inline fun enable_ungated_transfer(constructor_ref: &ConstructorRef) {
        let transfer_ref = object::generate_transfer_ref(constructor_ref);
        object::enable_ungated_transfer(&transfer_ref);
    }

    /// Generates the collections address based upon the creators address and the collection's name
    public fun create_collection_address(creator: &address, name: &String): address {
        object::create_object_address(creator, create_collection_seed(name))
    }

    /// Named objects are derived from a seed, the collection's seed is its name.
    public fun create_collection_seed(name: &String): vector<u8> {
        assert!(string::length(name) <= MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));
        *string::bytes(name)
    }

    /// Called by token on mint to increment supply if there's an appropriate Supply struct.
    public(friend) fun increment_supply(
        collection: &Object<Collection>,
        token: address,
    ): Option<AggregatorSnapshot<u64>> acquires FixedSupply, UnlimitedSupply, ConcurrentSupply {
        let collection_addr = object::object_address(collection);
        if (exists<ConcurrentSupply>(collection_addr)) {
            let supply = borrow_global_mut<ConcurrentSupply>(collection_addr);
            assert!(
                aggregator_v2::try_add(&mut supply.current_supply, 1),
                error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),
            );
            aggregator_v2::add(&mut supply.total_minted, 1);
            event::emit(
                Mint {
                    collection: collection_addr,
                    index: aggregator_v2::snapshot(&supply.total_minted),
                    token,
                },
            );
            option::some(aggregator_v2::snapshot(&supply.total_minted))
        } else if (exists<FixedSupply>(collection_addr)) {
            let supply = borrow_global_mut<FixedSupply>(collection_addr);
            supply.current_supply = supply.current_supply + 1;
            supply.total_minted = supply.total_minted + 1;
            assert!(
                supply.current_supply <= supply.max_supply,
                error::out_of_range(ECOLLECTION_SUPPLY_EXCEEDED),
            );
            if (std::features::module_event_migration_enabled()) {
                event::emit(
                    Mint {
                        collection: collection_addr,
                        index: aggregator_v2::create_snapshot(supply.total_minted),
                        token,
                    },
                );
            };
            event::emit_event(&mut supply.mint_events,
                MintEvent {
                    index: supply.total_minted,
                    token,
                },
            );
            option::some(aggregator_v2::create_snapshot<u64>(supply.total_minted))
        } else if (exists<UnlimitedSupply>(collection_addr)) {
            let supply = borrow_global_mut<UnlimitedSupply>(collection_addr);
            supply.current_supply = supply.current_supply + 1;
            supply.total_minted = supply.total_minted + 1;
            if (std::features::module_event_migration_enabled()) {
                event::emit(
                    Mint {
                        collection: collection_addr,
                        index: aggregator_v2::create_snapshot(supply.total_minted),
                        token,
                    },
                );
            };
            event::emit_event(
                &mut supply.mint_events,
                MintEvent {
                    index: supply.total_minted,
                    token,
                },
            );
            option::some(aggregator_v2::create_snapshot<u64>(supply.total_minted))
        } else {
            option::none()
        }
    }

    /// Called by token on burn to decrement supply if there's an appropriate Supply struct.
    public(friend) fun decrement_supply(
        collection: &Object<Collection>,
        token: address,
        index: Option<u64>,
        previous_owner: address,
    ) acquires FixedSupply, UnlimitedSupply, ConcurrentSupply {
        let collection_addr = object::object_address(collection);
        if (exists<ConcurrentSupply>(collection_addr)) {
            let supply = borrow_global_mut<ConcurrentSupply>(collection_addr);
            aggregator_v2::sub(&mut supply.current_supply, 1);

            event::emit(
                Burn {
                    collection: collection_addr,
                    index: *option::borrow(&index),
                    token,
                    previous_owner,
                },
            );
        } else if (exists<FixedSupply>(collection_addr)) {
            let supply = borrow_global_mut<FixedSupply>(collection_addr);
            supply.current_supply = supply.current_supply - 1;
            if (std::features::module_event_migration_enabled()) {
                event::emit(
                    Burn {
                        collection: collection_addr,
                        index: *option::borrow(&index),
                        token,
                        previous_owner,
                    },
                );
            };
            event::emit_event(
                &mut supply.burn_events,
                BurnEvent {
                    index: *option::borrow(&index),
                    token,
                },
            );
        } else if (exists<UnlimitedSupply>(collection_addr)) {
            let supply = borrow_global_mut<UnlimitedSupply>(collection_addr);
            supply.current_supply = supply.current_supply - 1;
            if (std::features::module_event_migration_enabled()) {
                event::emit(
                    Burn {
                        collection: collection_addr,
                        index: *option::borrow(&index),
                        token,
                        previous_owner,
                    },
                );
            };
            event::emit_event(
                &mut supply.burn_events,
                BurnEvent {
                    index: *option::borrow(&index),
                    token,
                },
            );
        }
    }

    /// Creates a MutatorRef, which gates the ability to mutate any fields that support mutation.
    public fun generate_mutator_ref(ref: &ConstructorRef): MutatorRef {
        let object = object::object_from_constructor_ref<Collection>(ref);
        MutatorRef { self: object::object_address(&object) }
    }

    public fun upgrade_to_concurrent(
        ref: &ExtendRef,
    ) acquires FixedSupply, UnlimitedSupply {
        let metadata_object_address = object::address_from_extend_ref(ref);
        let metadata_object_signer = object::generate_signer_for_extending(ref);

        let (supply, current_supply, total_minted, burn_events, mint_events) = if (exists<FixedSupply>(
            metadata_object_address
        )) {
            let FixedSupply {
                current_supply,
                max_supply,
                total_minted,
                burn_events,
                mint_events,
            } = move_from<FixedSupply>(metadata_object_address);

            let supply = ConcurrentSupply {
                current_supply: aggregator_v2::create_aggregator(max_supply),
                total_minted: aggregator_v2::create_unbounded_aggregator(),
            };
            (supply, current_supply, total_minted, burn_events, mint_events)
        } else if (exists<UnlimitedSupply>(metadata_object_address)) {
            let UnlimitedSupply {
                current_supply,
                total_minted,
                burn_events,
                mint_events,
            } = move_from<UnlimitedSupply>(metadata_object_address);

            let supply = ConcurrentSupply {
                current_supply: aggregator_v2::create_unbounded_aggregator(),
                total_minted: aggregator_v2::create_unbounded_aggregator(),
            };
            (supply, current_supply, total_minted, burn_events, mint_events)
        } else {
            // untracked collection is already concurrent, and other variants too.
            abort error::invalid_argument(EALREADY_CONCURRENT)
        };

        // update current state:
        aggregator_v2::add(&mut supply.current_supply, current_supply);
        aggregator_v2::add(&mut supply.total_minted, total_minted);
        move_to(&metadata_object_signer, supply);

        event::destroy_handle(burn_events);
        event::destroy_handle(mint_events);
    }

    // Accessors

    inline fun check_collection_exists(addr: address) {
        assert!(
            exists<Collection>(addr),
            error::not_found(ECOLLECTION_DOES_NOT_EXIST),
        );
    }

    inline fun borrow<T: key>(collection: &Object<T>): &Collection {
        let collection_address = object::object_address(collection);
        check_collection_exists(collection_address);
        borrow_global<Collection>(collection_address)
    }

    #[view]
    /// Provides the count of the current selection if supply tracking is used
    ///
    /// Note: Calling this method from transaction that also mints/burns, prevents
    /// it from being parallelized.
    public fun count<T: key>(
        collection: Object<T>
    ): Option<u64> acquires FixedSupply, UnlimitedSupply, ConcurrentSupply {
        let collection_address = object::object_address(&collection);
        check_collection_exists(collection_address);

        if (exists<ConcurrentSupply>(collection_address)) {
            let supply = borrow_global_mut<ConcurrentSupply>(collection_address);
            option::some(aggregator_v2::read(&supply.current_supply))
        } else if (exists<FixedSupply>(collection_address)) {
            let supply = borrow_global_mut<FixedSupply>(collection_address);
            option::some(supply.current_supply)
        } else if (exists<UnlimitedSupply>(collection_address)) {
            let supply = borrow_global_mut<UnlimitedSupply>(collection_address);
            option::some(supply.current_supply)
        } else {
            option::none()
        }
    }

    #[view]
    public fun creator<T: key>(collection: Object<T>): address acquires Collection {
        borrow(&collection).creator
    }

    #[view]
    public fun description<T: key>(collection: Object<T>): String acquires Collection {
        borrow(&collection).description
    }

    #[view]
    public fun name<T: key>(collection: Object<T>): String acquires Collection {
        borrow(&collection).name
    }

    #[view]
    public fun uri<T: key>(collection: Object<T>): String acquires Collection {
        borrow(&collection).uri
    }

    // Mutators

    inline fun borrow_mut(mutator_ref: &MutatorRef): &mut Collection {
        check_collection_exists(mutator_ref.self);
        borrow_global_mut<Collection>(mutator_ref.self)
    }

    /// Callers of this function must be aware that changing the name will change the calculated
    /// collection's address when calling `create_collection_address`.
    /// Once the collection has been created, the collection address should be saved for reference and
    /// `create_collection_address` should not be used to derive the collection's address.
    ///
    /// After changing the collection's name, to create tokens - only call functions that accept the collection object as an argument.
    public fun set_name(mutator_ref: &MutatorRef, name: String) acquires Collection {
        assert!(string::length(&name) <= MAX_COLLECTION_NAME_LENGTH, error::out_of_range(ECOLLECTION_NAME_TOO_LONG));
        let collection = borrow_mut(mutator_ref);
        event::emit(Mutation {
            mutated_field_name: string::utf8(b"name") ,
            collection: object::address_to_object(mutator_ref.self),
            old_value: collection.name,
            new_value: name,
        });
        collection.name = name;
    }

    public fun set_description(mutator_ref: &MutatorRef, description: String) acquires Collection {
        assert!(string::length(&description) <= MAX_DESCRIPTION_LENGTH, error::out_of_range(EDESCRIPTION_TOO_LONG));
        let collection = borrow_mut(mutator_ref);
        if (std::features::module_event_migration_enabled()) {
            event::emit(Mutation {
                mutated_field_name: string::utf8(b"description"),
                collection: object::address_to_object(mutator_ref.self),
                old_value: collection.description,
                new_value: description,
            });
        };
        collection.description = description;
        event::emit_event(
            &mut collection.mutation_events,
            MutationEvent { mutated_field_name: string::utf8(b"description") },
        );
    }

    public fun set_uri(mutator_ref: &MutatorRef, uri: String) acquires Collection {
        assert!(string::length(&uri) <= MAX_URI_LENGTH, error::out_of_range(EURI_TOO_LONG));
        let collection = borrow_mut(mutator_ref);
        if (std::features::module_event_migration_enabled()) {
            event::emit(Mutation {
                mutated_field_name: string::utf8(b"uri"),
                collection: object::address_to_object(mutator_ref.self),
                old_value: collection.uri,
                new_value: uri,
            });
        };
        collection.uri = uri;
        event::emit_event(
            &mut collection.mutation_events,
            MutationEvent { mutated_field_name: string::utf8(b"uri") },
        );
    }

    public fun set_max_supply(mutator_ref: &MutatorRef, max_supply: u64) acquires ConcurrentSupply, FixedSupply {
        let collection = object::address_to_object<Collection>(mutator_ref.self);
        let collection_address = object::object_address(&collection);
        let old_max_supply;

        if (exists<ConcurrentSupply>(collection_address)) {
            let supply = borrow_global_mut<ConcurrentSupply>(collection_address);
            let current_supply = aggregator_v2::read(&supply.current_supply);
            assert!(
                max_supply >= current_supply,
                error::out_of_range(EINVALID_MAX_SUPPLY),
            );
            old_max_supply = aggregator_v2::max_value(&supply.current_supply);
            supply.current_supply = aggregator_v2::create_aggregator(max_supply);
            aggregator_v2::add(&mut supply.current_supply, current_supply);
        } else if (exists<FixedSupply>(collection_address)) {
            let supply = borrow_global_mut<FixedSupply>(collection_address);
            assert!(
                max_supply >= supply.current_supply,
                error::out_of_range(EINVALID_MAX_SUPPLY),
            );
            old_max_supply = supply.max_supply;
            supply.max_supply = max_supply;
        } else {
            abort error::invalid_argument(ENO_MAX_SUPPLY_IN_COLLECTION)
        };

        event::emit(SetMaxSupply { collection, old_max_supply, new_max_supply: max_supply });
    }

    // Tests

    #[test_only]
    fun downgrade_from_concurrent_for_test(
        ref: &ExtendRef,
    ) acquires ConcurrentSupply {
        let metadata_object_address = object::address_from_extend_ref(ref);
        let metadata_object_signer = object::generate_signer_for_extending(ref);

        let ConcurrentSupply {
            current_supply,
            total_minted,
        } = move_from<ConcurrentSupply>(metadata_object_address);

        if (aggregator_v2::max_value(&current_supply) == MAX_U64) {
            move_to(&metadata_object_signer, UnlimitedSupply {
                current_supply: aggregator_v2::read(&current_supply),
                total_minted: aggregator_v2::read(&total_minted),
                burn_events: object::new_event_handle(&metadata_object_signer),
                mint_events: object::new_event_handle(&metadata_object_signer),
            });
        } else {
            move_to(&metadata_object_signer, FixedSupply {
                current_supply: aggregator_v2::read(&current_supply),
                max_supply: aggregator_v2::max_value(&current_supply),
                total_minted: aggregator_v2::read(&total_minted),
                burn_events: object::new_event_handle(&metadata_object_signer),
                mint_events: object::new_event_handle(&metadata_object_signer),
            });
        }
    }

    #[test(creator = @0x123)]
    fun test_create_mint_burn_for_unlimited(creator: &signer) acquires FixedSupply, UnlimitedSupply, ConcurrentSupply {
        let creator_address = signer::address_of(creator);
        let name = string::utf8(b"collection name");
        let constructor_ref = create_unlimited_collection(creator, string::utf8(b""), name, option::none(), string::utf8(b""));
        downgrade_from_concurrent_for_test(&object::generate_extend_ref(&constructor_ref));

        let collection_address = create_collection_address(&creator_address, &name);
        let collection = object::address_to_object<Collection>(collection_address);
        assert!(count(collection) == option::some(0), 0);
        let cid = aggregator_v2::read_snapshot(&option::destroy_some(increment_supply(&collection, creator_address)));
        assert!(count(collection) == option::some(1), 0);
        assert!(event::counter(&borrow_global<UnlimitedSupply>(collection_address).mint_events) == 1, 0);
        decrement_supply(&collection, creator_address, option::some(cid), creator_address);
        assert!(count(collection) == option::some(0), 0);
        assert!(event::counter(&borrow_global<UnlimitedSupply>(collection_address).burn_events) == 1, 0);
    }

    #[test(creator = @0x123)]
    fun test_create_mint_burn_for_fixed(creator: &signer) acquires FixedSupply, UnlimitedSupply, ConcurrentSupply {
        let creator_address = signer::address_of(creator);
        let name = string::utf8(b"collection name");
        let constructor_ref = create_fixed_collection(creator, string::utf8(b""), 1, name, option::none(), string::utf8(b""));
        downgrade_from_concurrent_for_test(&object::generate_extend_ref(&constructor_ref));

        let collection_address = create_collection_address(&creator_address, &name);
        let collection = object::address_to_object<Collection>(collection_address);
        assert!(count(collection) == option::some(0), 0);
        let cid = aggregator_v2::read_snapshot(&option::destroy_some(increment_supply(&collection, creator_address)));
        assert!(count(collection) == option::some(1), 0);
        assert!(event::counter(&borrow_global<FixedSupply>(collection_address).mint_events) == 1, 0);
        decrement_supply(&collection, creator_address, option::some(cid), creator_address);
        assert!(count(collection) == option::some(0), 0);
        assert!(event::counter(&borrow_global<FixedSupply>(collection_address).burn_events) == 1, 0);
    }

    #[test(creator = @0x123)]
    fun test_create_mint_burn_for_concurrent(
        creator: &signer
    ) acquires FixedSupply, UnlimitedSupply, ConcurrentSupply {
        let creator_address = signer::address_of(creator);
        let name = string::utf8(b"collection name");
        create_fixed_collection(creator, string::utf8(b""), 1, name, option::none(), string::utf8(b""));
        let collection_address = create_collection_address(&creator_address, &name);
        let collection = object::address_to_object<Collection>(collection_address);
        assert!(count(collection) == option::some(0), 0);
        let cid = increment_supply(&collection, creator_address);
        event::was_event_emitted<Mint>(&Mint {
            collection: collection_address,
            index: aggregator_v2::create_snapshot(0),
            token: creator_address,
        });
        assert!(cid == option::some(aggregator_v2::create_snapshot(1)), 1);
        assert!(count(collection) == option::some(1), 0);
        decrement_supply(&collection, creator_address, option::some(1), creator_address);
        event::was_event_emitted<Burn>(&Burn {
            collection: collection_address,
            index: 1,
            token: creator_address,
            previous_owner: creator_address,
        });
        assert!(count(collection) == option::some(0), 0);
    }

    #[test(creator = @0x123, trader = @0x456)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::object)]
    entry fun test_create_and_transfer(creator: &signer, trader: &signer) {
        let creator_address = signer::address_of(creator);
        let trader_address = signer::address_of(trader);
        let collection_name = string::utf8(b"collection name");
        create_collection_helper(creator, collection_name);

        let collection = object::address_to_object<Collection>(
            create_collection_address(&creator_address, &collection_name),
        );
        assert!(object::owner(collection) == creator_address, 1);
        object::transfer(creator, collection, trader_address);
    }

    #[test(creator = @0x123, trader = @0x456, aptos_framework = @aptos_framework)]
    entry fun test_create_and_transfer_as_owner(creator: &signer, trader: &signer, aptos_framework: &signer) {
        features::change_feature_flags_for_testing(aptos_framework, vector[features::get_collection_owner_feature()], vector[]);
        let creator_address = signer::address_of(creator);
        let trader_address = signer::address_of(trader);
        let collection_name = string::utf8(b"collection name");
        create_unlimited_collection_as_owner_helper(creator, collection_name);

        let collection = object::address_to_object<Collection>(
            create_collection_address(&creator_address, &collection_name),
        );
        assert!(object::owner(collection) == creator_address, 1);
        // Transferring owned collections are allowed
        object::transfer(creator, collection, trader_address);
        assert!(object::owner(collection) == trader_address, 1);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x80001, location = aptos_framework::object)]
    entry fun test_duplicate_collection(creator: &signer) {
        let collection_name = string::utf8(b"collection name");
        create_collection_helper(creator, collection_name);
        create_collection_helper(creator, collection_name);
    }

    #[test(creator = @0x123)]
    entry fun test_set_name(creator: &signer) acquires Collection {
        let collection_name = string::utf8(b"collection name");
        let constructor_ref = create_collection_helper(creator, collection_name);
        let mutator_ref = generate_mutator_ref(&constructor_ref);
        let collection = object::address_to_object<Collection>(
            create_collection_address(&signer::address_of(creator), &collection_name),
        );
        let new_collection_name = string::utf8(b"new collection name");
        assert!(new_collection_name != name(collection), 0);
        set_name(&mutator_ref, new_collection_name);
        assert!(new_collection_name == name(collection), 1);
        event::was_event_emitted(&Mutation {
            mutated_field_name: string::utf8(b"name"),
            collection,
            old_value: collection_name,
            new_value: new_collection_name,
        });
    }

    #[test(creator = @0x123)]
    entry fun test_set_description(creator: &signer) acquires Collection {
        let collection_name = string::utf8(b"collection name");
        let constructor_ref = create_collection_helper(creator, collection_name);
        let collection = object::address_to_object<Collection>(
            create_collection_address(&signer::address_of(creator), &collection_name),
        );
        let mutator_ref = generate_mutator_ref(&constructor_ref);
        let description = string::utf8(b"no fail");
        assert!(description != description(collection), 0);
        set_description(&mutator_ref, description);
        assert!(description == description(collection), 1);
    }

    #[test(creator = @0x123)]
    entry fun test_set_uri(creator: &signer) acquires Collection {
        let collection_name = string::utf8(b"collection name");
        let constructor_ref = create_collection_helper(creator, collection_name);
        let mutator_ref = generate_mutator_ref(&constructor_ref);
        let collection = object::address_to_object<Collection>(
            create_collection_address(&signer::address_of(creator), &collection_name),
        );
        let uri = string::utf8(b"no fail");
        assert!(uri != uri(collection), 0);
        set_uri(&mutator_ref, uri);
        assert!(uri == uri(collection), 1);
    }

    #[test(creator = @0x123)]
    entry fun test_set_max_supply_concurrent(creator: &signer) acquires ConcurrentSupply, FixedSupply {
        let collection_name = string::utf8(b"collection name");
        let max_supply = 100;
        let constructor_ref = create_fixed_collection_helper(creator, collection_name, max_supply);
        let mutator_ref = generate_mutator_ref(&constructor_ref);

        let new_max_supply = 200;
        set_max_supply(&mutator_ref, new_max_supply);

        let collection_address = create_collection_address(&signer::address_of(creator), &collection_name);
        let supply = borrow_global<ConcurrentSupply>(collection_address);
        assert!(aggregator_v2::max_value(&supply.current_supply) == new_max_supply, 0);

        event::was_event_emitted<SetMaxSupply>(&SetMaxSupply {
            collection: object::address_to_object<Collection>(collection_address),
            old_max_supply: max_supply,
            new_max_supply,
        });
    }

    #[test(creator = @0x123)]
    entry fun test_set_max_supply_same_as_current_supply_fixed(
        creator: &signer,
    ) acquires ConcurrentSupply, FixedSupply, UnlimitedSupply {
        let collection_name = string::utf8(b"collection name");
        let max_supply = 10;
        let constructor_ref = create_fixed_collection_helper(creator, collection_name, max_supply);
        let collection = object::object_from_constructor_ref<Collection>(&constructor_ref);
        let token_signer = create_token(creator);

        let current_supply = 5;
        let i = 0;
        while (i < current_supply) {
            increment_supply(&collection, signer::address_of(&token_signer));
            i = i + 1;
        };

        let mutator_ref = generate_mutator_ref(&constructor_ref);
        set_max_supply(&mutator_ref, current_supply);

        let collection_address = create_collection_address(&signer::address_of(creator), &collection_name);
        let supply = borrow_global<ConcurrentSupply>(collection_address);
        assert!(aggregator_v2::max_value(&supply.current_supply) == current_supply, EINVALID_MAX_SUPPLY);

        event::was_event_emitted<SetMaxSupply>(&SetMaxSupply {
            collection: object::address_to_object<Collection>(collection_address),
            old_max_supply: current_supply,
            new_max_supply: current_supply,
        });
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x1000A, location = aptos_token_objects::collection)]
    entry fun test_set_max_supply_none(creator: &signer) acquires ConcurrentSupply, FixedSupply {
        let collection_name = string::utf8(b"collection name");
        let constructor_ref = create_collection_helper(creator, collection_name);
        let mutator_ref = generate_mutator_ref(&constructor_ref);
        set_max_supply(&mutator_ref, 200);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20009, location = aptos_token_objects::collection)]
    entry fun test_set_max_supply_too_low_fixed_supply(creator: &signer) acquires ConcurrentSupply, FixedSupply, UnlimitedSupply {
        let max_supply = 3;
        let collection_name = string::utf8(b"Low Supply Collection");
        let constructor_ref = create_fixed_collection_helper(creator, collection_name, max_supply);
        downgrade_from_concurrent_for_test(&object::generate_extend_ref(&constructor_ref));

        let collection = object::object_from_constructor_ref<Collection>(&constructor_ref);
        let token_signer = create_token(creator);

        let i = 0;
        while (i < max_supply) {
            increment_supply(&collection, signer::address_of(&token_signer));
            i = i + 1;
        };

        let mutator_ref = generate_mutator_ref(&constructor_ref);
        let new_max_supply = 2;
        set_max_supply(&mutator_ref, new_max_supply);
    }

    #[test(creator = @0x123)]
    #[expected_failure(abort_code = 0x20009, location = aptos_token_objects::collection)]
    entry fun test_set_max_supply_too_low_concurrent_supply(creator: &signer) acquires ConcurrentSupply, FixedSupply, UnlimitedSupply {
        let collection_name = string::utf8(b"Low Supply Collection");
        let max_supply = 3;
        let constructor_ref = create_fixed_collection_helper(creator, collection_name, max_supply);
        let collection = object::object_from_constructor_ref<Collection>(&constructor_ref);
        let token_signer = create_token(creator);

        let i = 0;
        while (i < max_supply) {
            increment_supply(&collection, signer::address_of(&token_signer));
            i = i + 1;
        };

        let mutator_ref = generate_mutator_ref(&constructor_ref);
        let new_max_supply = 2;
        set_max_supply(&mutator_ref, new_max_supply);
    }

    #[test_only]
    fun create_collection_helper(creator: &signer, name: String): ConstructorRef {
        create_untracked_collection(
            creator,
            string::utf8(b"collection description"),
            name,
            option::none(),
            string::utf8(b"collection uri"),
        )
    }

    #[test_only]
    fun create_fixed_collection_helper(creator: &signer, name: String, max_supply: u64): ConstructorRef {
        create_fixed_collection(
            creator,
            string::utf8(b"description"),
            max_supply,
            name,
            option::none(),
            string::utf8(b"uri"),
        )
    }

    #[test_only]
    fun create_unlimited_collection_as_owner_helper(creator: &signer, name: String): ConstructorRef {
        create_unlimited_collection_as_owner(
            creator,
            string::utf8(b"description"),
            name,
            option::none(),
            string::utf8(b"uri"),
        )
    }

    #[test_only]
    /// Create a token as we cannot create a dependency cycle between collection and token modules.
    fun create_token(creator: &signer): signer {
        let token_constructor_ref = &object::create_object(signer::address_of(creator));
        object::generate_signer(token_constructor_ref)
    }
}
