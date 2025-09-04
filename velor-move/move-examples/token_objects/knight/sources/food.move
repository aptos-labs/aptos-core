/// This module implements the food tokens (fungible token). When the module initializes,
/// it creates the collection and two fungible tokens such as Corn and Meat.
module knight::food {
    use velor_framework::fungible_asset::{Self, Metadata};
    use velor_framework::object::{Self, Object};
    use velor_framework::primary_fungible_store;
    use velor_token_objects::collection;
    use velor_token_objects::property_map;
    use velor_token_objects::token;
    use std::error;
    use std::option;
    use std::signer;
    use std::string::{Self, String};

    friend knight::knight;

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

    /// The food collection name
    const FOOD_COLLECTION_NAME: vector<u8> = b"Food Collection Name";
    /// The food collection description
    const FOOD_COLLECTION_DESCRIPTION: vector<u8> = b"Food Collection Description";
    /// The food collection URI
    const FOOD_COLLECTION_URI: vector<u8> = b"https://food.collection.uri";

    /// The knight token collection name
    const KNIGHT_COLLECTION_NAME: vector<u8> = b"Knight Collection Name";
    /// The knight collection description
    const KNIGHT_COLLECTION_DESCRIPTION: vector<u8> = b"Knight Collection Description";
    /// The knight collection URI
    const KNIGHT_COLLECTION_URI: vector<u8> = b"https://knight.collection.uri";

    /// The corn token name
    const CORN_TOKEN_NAME: vector<u8> = b"Corn Token";
    /// The meat token name
    const MEAT_TOKEN_NAME: vector<u8> = b"Meat Token";

    /// Property names
    const CONDITION_PROPERTY_NAME: vector<u8> = b"Condition";
    const RESTORATION_VALUE_PROPERTY_NAME: vector<u8> = b"Restoration Value";
    const HEALTH_POINT_PROPERTY_NAME: vector<u8> = b"Health Point";

    /// The condition of a knight
    const CONDITION_HUNGRY: vector<u8> = b"Hungry";
    const CONDITION_GOOD: vector<u8> = b"Good";

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    // Food Token
    struct FoodToken has key {
        /// Used to mutate properties
        property_mutator_ref: property_map::MutatorRef,
        /// Used to mint fungible assets.
        fungible_asset_mint_ref: fungible_asset::MintRef,
        /// Used to burn fungible assets.
        fungible_asset_burn_ref: fungible_asset::BurnRef,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// Restoration value of the food. An attribute of a food token.
    struct RestorationValue has key {
        value: u64,
    }

    /// Initializes the module, creating the food collection and creating two fungible tokens such as Corn, and Meat.
    fun init_module(sender: &signer) {
        // Create a collection for food tokens.
        create_food_collection(sender);
        // Create two food token (i.e., Corn and Meat) as fungible tokens, meaning that there can be multiple units of them.
        create_food_token_as_fungible_token(
            sender,
            string::utf8(b"Corn Token Description"),
            string::utf8(CORN_TOKEN_NAME),
            string::utf8(b"https://raw.githubusercontent.com/velor-chain/velor-core/main/ecosystem/typescript/sdk/examples/typescript/metadata/knight/Corn"),
            string::utf8(b"Corn"),
            string::utf8(b"CORN"),
            string::utf8(b"https://raw.githubusercontent.com/velor-chain/velor-core/main/ecosystem/typescript/sdk/examples/typescript/metadata/knight/Corn.png"),
            string::utf8(b"https://www.velorlabs.com"),
            5,
        );
        create_food_token_as_fungible_token(
            sender,
            string::utf8(b"Meat Token Description"),
            string::utf8(MEAT_TOKEN_NAME),
            string::utf8(b"https://raw.githubusercontent.com/velor-chain/velor-core/main/ecosystem/typescript/sdk/examples/typescript/metadata/knight/Meat"),
            string::utf8(b"Meat"),
            string::utf8(b"MEAT"),
            string::utf8(b"https://raw.githubusercontent.com/velor-chain/velor-core/main/ecosystem/typescript/sdk/examples/typescript/metadata/knight/Meat.png"),
            string::utf8(b"https://www.velorlabs.com"),
            20,
        );
    }

    #[view]
    /// Returns the restoration value of the food token
    public fun restoration_value(token: Object<FoodToken>): u64 acquires RestorationValue {
        let restoration_value_in_food = borrow_global<RestorationValue>(object::object_address(&token));
        restoration_value_in_food.value
    }

    #[view]
    /// Returns the balance of the food token of the owner
    public fun food_balance(owner_addr: address, food: Object<FoodToken>): u64 {
        let metadata = object::convert<FoodToken, Metadata>(food);
        let store = primary_fungible_store::ensure_primary_store_exists(owner_addr, metadata);
        fungible_asset::balance(store)
    }

    #[view]
    /// Returns the corn token address
    public fun corn_token_address(): address {
        food_token_address(string::utf8(CORN_TOKEN_NAME))
    }

    #[view]
    /// Returns the meat token address
    public fun meat_token_address(): address {
        food_token_address(string::utf8(MEAT_TOKEN_NAME))
    }

    #[view]
    /// Returns the food token address by name
    public fun food_token_address(food_token_name: String): address {
        token::create_token_address(&@knight, &string::utf8(FOOD_COLLECTION_NAME), &food_token_name)
    }

    /// Mints the given amount of the corn token to the given receiver.
    public entry fun mint_corn(creator: &signer, receiver: address, amount: u64) acquires FoodToken {
        let corn_token = object::address_to_object<FoodToken>(corn_token_address());
        mint_internal(creator, corn_token, receiver, amount);
    }

    /// Mints the given amount of the meat token to the given receiver.
    public entry fun mint_meat(creator: &signer, receiver: address, amount: u64) acquires FoodToken {
        let meat_token = object::address_to_object<FoodToken>(meat_token_address());
        mint_internal(creator, meat_token, receiver, amount);
    }

    /// Transfers the given amount of the corn token from the given sender to the given receiver.
    public entry fun transfer_corn(from: &signer, to: address, amount: u64) {
        transfer_food(from, object::address_to_object<FoodToken>(corn_token_address()), to, amount);
    }

    /// Transfers the given amount of the meat token from the given sender to the given receiver.
    public entry fun transfer_meat(from: &signer, to: address, amount: u64) {
        transfer_food(from, object::address_to_object<FoodToken>(meat_token_address()), to, amount);
    }

    public entry fun transfer_food(from: &signer, food: Object<FoodToken>, to: address, amount: u64) {
        let metadata = object::convert<FoodToken, Metadata>(food);
        primary_fungible_store::transfer(from, metadata, to, amount);
    }

    public(friend) fun burn_food(from: &signer, food: Object<FoodToken>, amount: u64) acquires FoodToken {
        let metadata = object::convert<FoodToken, Metadata>(food);
        let food_addr = object::object_address(&food);
        let food_token = borrow_global<FoodToken>(food_addr);
        let from_store = primary_fungible_store::ensure_primary_store_exists(signer::address_of(from), metadata);
        fungible_asset::burn_from(&food_token.fungible_asset_burn_ref, from_store, amount);
    }

    /// Creates the food collection.
    fun create_food_collection(creator: &signer) {
        // Constructs the strings from the bytes.
        let description = string::utf8(FOOD_COLLECTION_DESCRIPTION);
        let name = string::utf8(FOOD_COLLECTION_NAME);
        let uri = string::utf8(FOOD_COLLECTION_URI);

        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            creator,
            description,
            name,
            option::none(),
            uri,
        );
    }

    /// Creates the food token as fungible token.
    fun create_food_token_as_fungible_token(
        creator: &signer,
        description: String,
        name: String,
        uri: String,
        fungible_asset_name: String,
        fungible_asset_symbol: String,
        icon_uri: String,
        project_uri: String,
        restoration_value: u64,
    ) {
        // The collection name is used to locate the collection object and to create a new token object.
        let collection = string::utf8(FOOD_COLLECTION_NAME);
        // Creates the food token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let constructor_ref = token::create_named_token(
            creator,
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The object signer is used to publish a resource
        // (e.g., RestorationValue) under the token object address. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let property_mutator_ref = property_map::generate_mutator_ref(&constructor_ref);

        // Initializes the value with the given value in food.
        move_to(&object_signer, RestorationValue { value: restoration_value });

        // Initialize the property map.
        let properties = property_map::prepare_input(vector[], vector[], vector[]);
        property_map::init(&constructor_ref, properties);
        property_map::add_typed(
            &property_mutator_ref,
            string::utf8(RESTORATION_VALUE_PROPERTY_NAME),
            restoration_value
        );

        // Creates the fungible asset.
        primary_fungible_store::create_primary_store_enabled_fungible_asset(
            &constructor_ref,
            option::none(),
            fungible_asset_name,
            fungible_asset_symbol,
            0,
            icon_uri,
            project_uri,
        );
        let fungible_asset_mint_ref = fungible_asset::generate_mint_ref(&constructor_ref);
        let fungible_asset_burn_ref = fungible_asset::generate_burn_ref(&constructor_ref);

        // Publishes the FoodToken resource with the refs.
        let food_token = FoodToken {
            property_mutator_ref,
            fungible_asset_mint_ref,
            fungible_asset_burn_ref,
        };
        move_to(&object_signer, food_token);
    }

    /// The internal mint function.
    fun mint_internal(creator: &signer, token: Object<FoodToken>, receiver: address, amount: u64) acquires FoodToken {
        let food_token = authorized_borrow<FoodToken>(creator, &token);
        let fungible_asset_mint_ref = &food_token.fungible_asset_mint_ref;
        let fa = fungible_asset::mint(fungible_asset_mint_ref, amount);
        primary_fungible_store::deposit(receiver, fa);
    }

    inline fun authorized_borrow<T: key>(creator: &signer, token: &Object<T>): &FoodToken {
        let token_address = object::object_address(token);
        assert!(
            exists<FoodToken>(token_address),
            error::not_found(ETOKEN_DOES_NOT_EXIST),
        );

        assert!(
            token::creator(*token) == signer::address_of(creator),
            error::permission_denied(ENOT_CREATOR),
        );
        borrow_global<FoodToken>(token_address)
    }

    #[test_only]
    public fun init_module_for_test(creator: &signer) {
        init_module(creator);
    }

    #[test(creator = @knight, user1 = @0x456, user2 = @0x789)]
    public fun test_food(creator: &signer, user1: &signer, user2: &signer) acquires FoodToken {
        // This test assumes that the creator's address is equal to @knight.
        assert!(signer::address_of(creator) == @knight, 0);

        // ---------------------------------------------------------------------
        // Creator creates the collection, and mints corn and meat tokens in it.
        // ---------------------------------------------------------------------
        init_module(creator);

        // -------------------------------------------
        // Creator mints and sends 100 corns to User1.
        // -------------------------------------------
        let user1_addr = signer::address_of(user1);
        mint_corn(creator, user1_addr, 100);

        let corn_token = object::address_to_object<FoodToken>(corn_token_address());
        // Assert that the user1 has 100 corns.
        assert!(food_balance(user1_addr, corn_token) == 100, 0);

        // -------------------------------------------
        // Creator mints and sends 200 meats to User2.
        // -------------------------------------------
        let user2_addr = signer::address_of(user2);
        mint_meat(creator, user2_addr, 200);
        let meat_token = object::address_to_object<FoodToken>(meat_token_address());
        // Assert that the user2 has 200 meats.
        assert!(food_balance(user2_addr, meat_token) == 200, 0);

        // ------------------------------
        // User1 sends 10 corns to User2.
        // ------------------------------
        transfer_corn(user1, user2_addr, 10);
        // Assert that the user1 has 90 corns.
        assert!(food_balance(user1_addr, corn_token) == 90, 0);
        // Assert that the user2 has 10 corns.
        assert!(food_balance(user2_addr, corn_token) == 10, 0);

        // ------------------------------
        // User2 sends 20 meats to User1.
        // ------------------------------
        transfer_meat(user2, user1_addr, 20);
        // Assert that the user1 has 20 meats.
        assert!(food_balance(user1_addr, meat_token) == 20, 0);
        // Assert that the user2 has 180 meats.
        assert!(food_balance(user2_addr, meat_token) == 180, 0);
    }
}
