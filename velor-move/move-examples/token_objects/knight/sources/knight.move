/// This module implements the knight token (non-fungible token) including the
/// functions create the collection and the knight tokens, and the function to feed a
/// knight token with food tokens to increase the knight's health point.
module knight::knight {
    use velor_framework::event;
    use velor_framework::object::{Self, Object};
    use velor_token_objects::collection;
    use velor_token_objects::property_map;
    use velor_token_objects::token;
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use knight::food::{Self, FoodToken};

    /// The token does not exist
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

    /// The knight token collection name
    const KNIGHT_COLLECTION_NAME: vector<u8> = b"Knight Collection Name";
    /// The knight collection description
    const KNIGHT_COLLECTION_DESCRIPTION: vector<u8> = b"Knight Collection Description";
    /// The knight collection URI
    const KNIGHT_COLLECTION_URI: vector<u8> = b"https://knight.collection.uri";

    /// Property names
    const CONDITION_PROPERTY_NAME: vector<u8> = b"Condition";
    const HEALTH_POINT_PROPERTY_NAME: vector<u8> = b"Health Point";

    /// The condition of a knight
    const CONDITION_HUNGRY: vector<u8> = b"Hungry";
    const CONDITION_GOOD: vector<u8> = b"Good";

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Knight token
    struct KnightToken has key {
        /// Used to mutate the token uri
        mutator_ref: token::MutatorRef,
        /// Used to mutate properties
        property_mutator_ref: property_map::MutatorRef,
        /// the base URI of the token
        base_uri: String,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// The knight's health point
    struct HealthPoint has key {
        value: u64,
    }

    #[event]
    /// The health update event
    struct HealthUpdate has drop, store {
        token: address,
        old_health: u64,
        new_health: u64,
    }

    /// Initializes the module, creating the knight token collection.
    fun init_module(sender: &signer) {
        // Create a collection for knight tokens.
        create_knight_collection(sender);
    }

    #[view]
    /// Returns the knight health point of the token
    public fun health_point(token: Object<KnightToken>): u64 acquires HealthPoint {
        let health = borrow_global<HealthPoint>(object::object_address(&token));
        health.value
    }

    #[view]
    /// Returns the knight token address by name
    public fun knight_token_address(knight_token_name: String): address {
        token::create_token_address(&@knight, &string::utf8(KNIGHT_COLLECTION_NAME), &knight_token_name)
    }

    /// Mints an knight token. This function mints a new knight token and transfers it to the
    /// `soul_bound_to` address. The token is minted with health point 0 and condition Hungry.
    public entry fun mint_knight(
        creator: &signer,
        description: String,
        name: String,
        base_uri: String,
        receiver: address,
    ) {
        // The collection name is used to locate the collection object and to create a new token object.
        let collection = string::utf8(KNIGHT_COLLECTION_NAME);
        // Creates the knight token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let uri = base_uri;
        string::append(&mut uri, string::utf8(CONDITION_HUNGRY));
        let constructor_ref = token::create_named_token(
            creator,
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The object signer is used to publish a resource
        // (e.g., HealthPoint) under the token object address. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let mutator_ref = token::generate_mutator_ref(&constructor_ref);
        let property_mutator_ref = property_map::generate_mutator_ref(&constructor_ref);

        // Transfers the token to the `soul_bound_to` address
        if (receiver != signer::address_of(creator)) {
            let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
            object::transfer_with_ref(linear_transfer_ref, receiver);
        };

        // Initializes the knight health point as 0
        move_to(&object_signer, HealthPoint { value: 1 });

        // Initialize the property map and the knight condition as Hungry
        let properties = property_map::prepare_input(vector[], vector[], vector[]);
        property_map::init(&constructor_ref, properties);
        property_map::add_typed(
            &property_mutator_ref,
            string::utf8(CONDITION_PROPERTY_NAME),
            string::utf8(CONDITION_HUNGRY),
        );
        // Although the health point is stored in the HealthPoint resource, it is also duplicated
        // and stored in the property map to be recognized as a property by the wallet.
        property_map::add_typed(
            &property_mutator_ref,
            string::utf8(HEALTH_POINT_PROPERTY_NAME),
            1,
        );

        // Publishes the KnightToken resource with the refs.
        let knight_token = KnightToken {
            mutator_ref,
            property_mutator_ref,
            base_uri
        };
        move_to(&object_signer, knight_token);
    }

    public entry fun feed_corn(from: &signer, to: Object<KnightToken>, amount: u64) acquires HealthPoint, KnightToken {
        let corn_token = object::address_to_object<FoodToken>(food::corn_token_address());
        feed_food(from, corn_token, to, amount);
    }

    public entry fun feed_meat(from: &signer, to: Object<KnightToken>, amount: u64) acquires HealthPoint, KnightToken {
        let meat_token = object::address_to_object<FoodToken>(food::meat_token_address());
        feed_food(from, meat_token, to, amount);
    }

    public entry fun feed_food(
        from: &signer,
        food: Object<FoodToken>,
        to: Object<KnightToken>,
        amount: u64
    ) acquires HealthPoint, KnightToken {
        food::burn_food(from, food, amount);

        let restoration_amount = food::restoration_value(food) * amount;
        let knight_token_address = object::object_address(&to);
        let health_point = borrow_global_mut<HealthPoint>(knight_token_address);
        let old_health_point = health_point.value;
        let new_health_point = old_health_point + restoration_amount;
        health_point.value = new_health_point;

        let knight = borrow_global_mut<KnightToken>(knight_token_address);
        // Gets `property_mutator_ref` to update the health point and condition in the property map.
        let property_mutator_ref = &knight.property_mutator_ref;
        // Updates the health point in the property map.
        property_map::update_typed(property_mutator_ref, &string::utf8(HEALTH_POINT_PROPERTY_NAME), new_health_point);

        event::emit(
            HealthUpdate {
                token: knight_token_address,
                old_health: old_health_point,
                new_health: new_health_point,
            }
        );

        // `new_condition` is determined based on the new health point.
        let new_condition = if (new_health_point <= 20) {
            CONDITION_HUNGRY
        } else {
            CONDITION_GOOD
        };
        // Updates the condition in the property map.
        property_map::update_typed(
            property_mutator_ref,
            &string::utf8(CONDITION_PROPERTY_NAME),
            string::utf8(new_condition)
        );

        // Updates the token URI based on the new condition.
        let uri = knight.base_uri;
        string::append(&mut uri, string::utf8(new_condition));
        token::set_uri(&knight.mutator_ref, uri);
    }

    /// Creates the knight collection. This function creates a collection with unlimited supply using
    /// the module constants for description, name, and URI, defined above. The royalty configuration
    /// is skipped in this collection for simplicity.
    fun create_knight_collection(creator: &signer) {
        // Constructs the strings from the bytes.
        let description = string::utf8(KNIGHT_COLLECTION_DESCRIPTION);
        let name = string::utf8(KNIGHT_COLLECTION_NAME);
        let uri = string::utf8(KNIGHT_COLLECTION_URI);

        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            creator,
            description,
            name,
            option::none(),
            uri,
        );
    }

    #[test(creator = @knight, user1 = @0x456)]
    public fun test_knight(creator: &signer, user1: &signer) acquires HealthPoint, KnightToken {
        // This test assumes that the creator's address is equal to @knight.
        assert!(signer::address_of(creator) == @knight, 0);

        // ---------------------------------------------------------------------
        // Creator creates the collection, and mints corn and meat tokens in it.
        // ---------------------------------------------------------------------
        food::init_module_for_test(creator);
        init_module(creator);

        // -------------------------------------------------------
        // Creator mints and sends 90 corns and 20 meats to User1.
        // -------------------------------------------------------
        let user1_addr = signer::address_of(user1);
        food::mint_corn(creator, user1_addr, 90);
        food::mint_meat(creator, user1_addr, 20);

        // ---------------------------------------
        // Creator mints a knight token for User1.
        // ---------------------------------------
        let token_name = string::utf8(b"Knight Token #1");
        let token_description = string::utf8(b"Knight Token #1 Description");
        let token_uri = string::utf8(b"Knight Token #1 URI");
        let user1_addr = signer::address_of(user1);
        // Creates the knight token for User1.
        mint_knight(
            creator,
            token_description,
            token_name,
            token_uri,
            user1_addr,
        );
        let token_address = knight_token_address(token_name);
        let knight_token = object::address_to_object<KnightToken>(token_address);

        // Asserts that the owner of the token is User1.
        assert!(object::owner(knight_token) == user1_addr, 1);
        // Asserts that the health point of the token is 1.
        assert!(health_point(knight_token) == 1, 2);

        let corn_token = object::address_to_object<FoodToken>(food::corn_token_address());
        let old_corn_balance = food::food_balance(user1_addr, corn_token);
        feed_food(user1, corn_token, knight_token, 3);
        // Asserts that the corn balance decreases by 3.
        assert!(food::food_balance(user1_addr, corn_token) == old_corn_balance - 3, 0);
        // Asserts that the health point increases by 15 (= amount * restoration_value = 3 * 5).
        assert!(health_point(knight_token) == 16, 2);

        let meat_token = object::address_to_object<FoodToken>(food::meat_token_address());
        let old_meat_balance = food::food_balance(user1_addr, meat_token);
        feed_food(user1, meat_token, knight_token, 2);
        // Asserts that the corn balance decreases by 3.
        assert!(food::food_balance(user1_addr, meat_token) == old_meat_balance - 2, 0);
        // Asserts that the health point increases by 40 (= amount * restoration_value = 2 * 20).
        assert!(health_point(knight_token) == 56, 3);
    }
}
