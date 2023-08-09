/// This module implements the egg and animal tokens.
/// The flow works as follows:
/// 1. When the admin (i.e., the contract owner) publishes this module,
///   - a egg collection manager object is created,
///   - a egg collection is created with the signer of the egg collection manager, and
///   - a whitelist for egg masters is created.
/// 2. The admin adds egg masters to the whitelist.
/// 3. A whitelisted egg master calls `mint_egg` to mint a egg token,
///    and creates a animal collection for the egg.
/// 4. A egg master with a egg token calls `mint_animal` to mint
///    a animal token for a user who is a egg animal.
/// 5. A egg master burns (or revokes) a animal token.
module egg::egg {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use aptos_std::smart_vector::{Self, SmartVector};
    use aptos_std::string_utils;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::timestamp;
    use aptos_token_objects::collection;
    use aptos_token_objects::token;

    /// The provided signer is not the admin
    const ENOT_ADMIN: u64 = 1;
    /// The provided signer is not the owner
    const ENOT_OWNER: u64 = 2;
    /// The provided signer is not in the whitelist
    const ENOT_IN_WHITELIST: u64 = 3;

    /// The egg collection name
    const EGG_COLLECTION_NAME: vector<u8> = b"Egg Collection Name";
    /// The egg collection description
    const EGG_COLLECTION_DESCRIPTION: vector<u8> = b"Egg Collection Description";
    /// The egg collection URI
    const EGG_COLLECTION_URI: vector<u8> = b"https://egg.collection.uri";

    /// The egg token name
    const EGG_TOKEN_NAME: vector<u8> = b"Egg Token Name";
    /// The egg token description
    const EGG_TOKEN_DESCRIPTION: vector<u8> = b"Egg Token Description";
    /// The egg token URI
    const EGG_TOKEN_URI: vector<u8> = b"https://raw.githubusercontent.com/junkil-park/metadata/main/egg/egg";

    /// The animal collection name
    const ANIMAL_COLLECTION_NAME: vector<u8> = b"Animal Collection Name";
    /// The animal collection description
    const ANIMAL_COLLECTION_DESCRIPTION: vector<u8> = b"Animal Collection Description";
    /// The animal collection URI
    const ANIMAL_COLLECTION_URI: vector<u8> = b"https://animal.collection.uri";

    /// The animal token name
    const ANIMAL_TOKEN_NAME: vector<u8> = b"Animal Token Name";
    /// The animal token description
    const ANIMAL_TOKEN_DESCRIPTION: vector<u8> = b"Animal Token Description";
    /// The animal token URI
    const ANIMAL_TOKEN_BASE_URI: vector<u8> = b"https://raw.githubusercontent.com/junkil-park/metadata/main/egg/animal";

    /// Published under the contract owner's account.
    struct Config has key {
        /// Whitelist of egg masters.
        whitelist: SmartVector<address>,
        /// Egg counter
        egg_counter: u64,
        /// `extend_ref` of the collection manager object. Used to obtain its signer.
        extend_ref: object::ExtendRef,
    }

    #[resource_group_animal(group = aptos_framework::object::ObjectGroup)]
    /// Egg token
    struct EggToken has key {
        /// Used to burn
        burn_ref: token::BurnRef,
    }

    #[resource_group_animal(group = aptos_framework::object::ObjectGroup)]
    /// Animal token
    struct AnimalToken has key {
    }

    /// Initializes the module, creating the manager object, the egg token collection and the whitelist.
    fun init_module(sender: &signer) acquires Config {
        // Creates the collection manager object.
        create_collection_manage(sender);
        // Create the egg collection.
        create_collection(&collection_manager_signer(), EGG_COLLECTION_DESCRIPTION, EGG_COLLECTION_NAME, EGG_COLLECTION_URI);
        // Create the animal collection.
        create_collection(&collection_manager_signer(), ANIMAL_COLLECTION_DESCRIPTION, ANIMAL_COLLECTION_NAME, ANIMAL_COLLECTION_URI);
    }

    #[view]
    /// Returns the egg token address by name
    public fun egg_token_address(egg_token_name: String): address acquires Config {
        token::create_token_address(&collection_manager_address(), &string::utf8(EGG_COLLECTION_NAME), &egg_token_name)
    }

    #[view]
    /// Returns the egg token address by name
    public fun animal_token_address(animal_token_name: String): address acquires Config {
        token::create_token_address(&collection_manager_address(), &string::utf8(ANIMAL_COLLECTION_NAME), &animal_token_name)
    }

    /// Adds an account to the whitelist. This function allows the admin to add an account to the whitelist.
    public entry fun add_to_whitelist(admin: &signer, account: address) acquires Config {
        assert!(signer::address_of(admin) == collection_manager_owner(), ENOT_ADMIN);
        let config = borrow_global_mut<Config>(@egg);
        smart_vector::push_back(&mut config.whitelist, account);
    }

    /// Removes an account from the whitelist. This function allows the admin to remove an account from the whitelist.
    fun remove_from_whitelist(account: address) acquires Config {
        let config = borrow_global_mut<Config>(@egg);
        let (_, idx) = smart_vector::index_of(&config.whitelist, &account);
        smart_vector::swap_remove(&mut config.whitelist, idx);
    }

    fun egg_counter(): u64 acquires Config {
        let config = borrow_global<Config>(@egg);
        config.egg_counter
    }

    fun increase_egg_counter() acquires Config {
        let config = borrow_global_mut<Config>(@egg);
        config.egg_counter = config.egg_counter + 1;
    }

    /// Mints a egg token, and creates a new associated animal collection.
    /// This function allows a whitelisted account to mint a new egg token.
    public entry fun mint_egg(
        account: &signer,
    ) acquires Config {
        let account_addr = signer::address_of(account);

        // // Checks if the egg master is whitelisted.
        // assert!(is_whitelisted(account_addr), ENOT_IN_WHITELIST);
        // // Removes the account from the whitelist.
        // remove_from_whitelist(account_addr);

        let description = string::utf8(EGG_TOKEN_DESCRIPTION);
        let name = string_utils::to_string(&egg_counter());
        let uri = string::utf8(EGG_TOKEN_URI);
        increase_egg_counter();

        // The collection name is used to locate the collection object and to create a new token object.
        let collection = string::utf8(EGG_COLLECTION_NAME);
        // Creates the egg token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        // TODO: Switch to `create_token` once it is available.
        let constructor_ref = token::create_named_token(
            &collection_manager_signer(),
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let burn_ref = token::generate_burn_ref(&constructor_ref);

        // Transfers the token to the egg master.
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, account_addr);

        // Publishes the EggToken resource with the refs.
        let egg_token = EggToken {
            burn_ref,
        };
        move_to(&object_signer, egg_token);
    }

    /// Mints a animal token. This function mints a new animal token and transfers it to the
    /// `receiver` address.
    public entry fun hatch_egg(
        account: &signer,
        egg_token: Object<EggToken>,
    ) acquires EggToken, Config {
        let account_addr = signer::address_of(account);
        // Checks if the egg master is the owner of the egg token.
        assert!(object::owner(egg_token) == account_addr, ENOT_OWNER);

        let description = string::utf8(ANIMAL_TOKEN_DESCRIPTION);
        let name = token::name(egg_token);
        let uri = string::utf8(ANIMAL_TOKEN_BASE_URI);
        let postfix = string_utils::to_string(&(timestamp::now_microseconds() % 30));
        string::append(&mut uri, postfix);
        let collection = string::utf8(ANIMAL_COLLECTION_NAME);

        // Creates the animal token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let constructor_ref = token::create_named_token(
            &collection_manager_signer(),
            collection,
            description,
            name,
            option::none(),
            uri,
        );

        // Generates the object signer and the refs. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);

        // Transfers the token to the `soul_bound_to` address
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, account_addr);

        // Publishes the AnimalToken resource with the refs.
        let animal_token = AnimalToken {
        };
        move_to(&object_signer, animal_token);

        burn_egg(egg_token);
    }

    /// Burns a animal token.
    fun burn_egg(
        token: Object<EggToken>,
    ) acquires EggToken {
        let EggToken { burn_ref } = move_from<EggToken>(object::object_address(&token));
        token::burn(burn_ref);
    }

    /// Creates the collection manager object.
    fun create_collection_manage(sender: &signer) {
        // Create the collection manager object to use it to autonomously
        // manage the collections (e.g., create the collection and mint tokens).
        let constructor_ref = object::create_object(signer::address_of(sender));
        let extend_ref = object::generate_extend_ref(&constructor_ref);

        // Publish the config resource.
        move_to(sender, Config { whitelist: smart_vector::new(), egg_counter: 0, extend_ref});
    }

    /// Returns the signer of the collection manager object.
    fun collection_manager_signer(): signer acquires Config {
        let manager = borrow_global<Config>(@egg);
        object::generate_signer_for_extending(&manager.extend_ref)
    }

    /// Returns the signer of the collection manager object.
    fun collection_manager_owner(): address acquires Config {
        let manager = borrow_global<Config>(@egg);
        let manager_addr = object::address_from_extend_ref(&manager.extend_ref);
        object::owner(object::address_to_object<object::ObjectCore>(manager_addr))
    }

    /// Returns the address of the collection manager object.
    fun collection_manager_address(): address acquires Config {
        let manager = borrow_global<Config>(@egg);
        object::address_from_extend_ref(&manager.extend_ref)
    }

    /// Creates a collection with unlimited supply. The royalty configuration
    /// is skipped in this collection for simplicity.
    fun create_collection(admin: &signer, collection_description: vector<u8>, collection_name: vector<u8>, collection_uri: vector<u8>) {
        // Constructs the strings from the bytes.
        let description = string::utf8(collection_description);
        let name = string::utf8(collection_name);
        let uri = string::utf8(collection_uri);

        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            admin,
            description,
            name,
            option::none(),
            uri,
        );
    }

    inline fun is_whitelisted(account: address): bool {
        let whitelist = &borrow_global<Config>(@egg).whitelist;
        smart_vector::contains(whitelist, &account)
    }

    #[test(fx = @std, admin = @egg, user = @0x456)]
    public fun test_egg(fx: signer, admin: &signer, user: &signer) acquires EggToken, Config {
        use std::features;

        let feature = features::get_auids();
        features::change_feature_flags(&fx, vector[feature], vector[]);

        // This test assumes that the creator's address is equal to @token_objects.
        assert!(signer::address_of(admin) == @egg, 0);


        timestamp::set_time_has_started_for_testing(&fx);

        // -----------------------------------
        // Admin creates the collections.
        // -----------------------------------
        init_module(admin);

        // ---------------------------------------------
        // Admin adds User to the whitelist.
        // ---------------------------------------------
        add_to_whitelist(admin, signer::address_of(user));

        // ------------------------------------------
        // User mints a egg token.
        // ------------------------------------------
        mint_egg(user);

        // -------------------------------------------
        // User hatches the egg token.
        // -------------------------------------------
        let egg_token_addr = egg_token_address(string::utf8(b"0"));
        let egg_token = object::address_to_object<EggToken>(egg_token_addr);

        // Hatch the egg token.
        hatch_egg(user, egg_token);
    }
}
