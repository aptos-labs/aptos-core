module no_code_mint::mint_machine {
    use std::error;
    use std::signer;
    use std::object::{Self, Object, ExtendRef, DeleteRef};
    use std::string::{Self, String, utf8 as str};
    use aptos_std::string_utils::{Self};
    use std::timestamp;
    use std::vector;
    use no_code_mint::allowlist;
    use no_code_mint::package_manager;
    use aptos_std::smart_vector::{Self, SmartVector};
    use aptos_std::smart_table::{Self, SmartTable};
    use aptos_token_objects::aptos_token::{Self, AptosToken};
    use aptos_token_objects::collection::{Self, Collection};
    use no_code_mint::auid_manager::{Self, AuidManager};
    use aptos_token_objects::property_map::{Self};

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    /// This resource stores the collection configuration information and the object refs for the object it's stored at.
    /// Most things that are stored here are things that can change post-collection creation.
    /// We will also store the Allowlist resource at this Object's address.
    struct MintConfiguration has key {
        collection_name: String,          // Immutable
        collection_addr: address,         // The address of the collection object. Immutable
        max_supply: u64,                  // Immutable, only in here because it's not in collection.move
        token_base_name: String,          // The base name for all tokens. The token number is appended to this. Mutable until mint begins
        minting_enabled: bool,            // Mutable, must be set to true before minting can begin
        extend_ref: ExtendRef,            // ExtendRef for the creator object
        delete_ref: DeleteRef,            // DeleteRef for the creator object
        token_uris: SmartVector<String>,  // Key is a unique token uri. Used as a set in conjunction with metadata table
        metadata_table: SmartTable<String, TokenMetadata>, // The key is the token uri. we pop from token_uris to get TokenMetadata from this table
    }

    struct TokenMetadata has copy, drop, store {
        description: String,
        property_keys: vector<String>,
        property_values: vector<vector<u8>>,
        property_types: vector<String>,
    }

    struct ReadyForLaunch {
        mint_config_exists: bool,
        allowlist_exists: bool,
        has_valid_tier: bool,
        collection_exists: bool,
        metadata_complete: bool,
    }

    /// Action not authorized because the signer is not the admin.
    const ENOT_AUTHORIZED: u64 = 1;
    /// There is no creator object for the given signer.
    const ECREATOR_OBJECT_NOT_FOUND: u64 = 2;
    /// The given signer is not the owner of the creator object.
    const ENOT_OWNER_OF_CREATOR_OBJECT: u64 = 3;
    /// The collection hasn't been created yet.
    const ECOLLECTION_OBJECT_NOT_FOUND: u64 = 4;
    /// There are no valid tiers in the allowlist that a user can mint from.
    const ENO_VALID_TIERS: u64 = 5;
    /// The input vector lengths aren't the same.
    const EVECTOR_LENGTHS_INCONSISTENT: u64 = 6;
    /// There isn't enough metadata to mint the `max_supply` # of tokens.
    const EINSUFFICIENT_METADATA: u64 = 7;
    /// The minting machine is not ready for launch.
    const ENOT_READY_FOR_LAUNCH: u64 = 8;
    /// The token name is already in the list of token names.
    const EDUPLICATE_TOKEN_URI: u64 = 9;
    /// Minting is disabled.
    const EMINTING_DISABLED: u64 = 10;
    /// Maximum supply exceeded.
    const EMAXIMUM_SUPPLY_EXCEEDED: u64 = 11;
    /// Minting has already begun.
    const EMINTING_ALREADY_BEGUN: u64 = 12;
    /// Minting has not completed yet.
    const EINCOMPLETE_MINT: u64 = 13;
    /// One of the token uris is not in the list of token uris.
    const ETOKEN_METADATA_NOT_FOUND: u64 = 13;

    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //                                                                                      //
    //                                   Setup / pre-launch                                 //
    //                                                                                      //
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////

    /// This function creates the collection, the MintConfiguration object (aka the creator/creator_obj),
    /// and stores the object addresses in the MintConfiguration object.
    /// The package manager stores the address of both objects for lookup later.
    public entry fun initialize_mint_machine(
        admin: &signer,
        description: String,
        max_supply: u64,
        name: String,
        uri: String,
        mutable_description: bool,
        mutable_royalty: bool,
        mutable_uri: bool,
        mutable_token_description: bool,
        mutable_token_name: bool,
        mutable_token_properties: bool,
        mutable_token_uri: bool,
        tokens_burnable_by_creator: bool,
        tokens_freezable_by_creator: bool,
        royalty_numerator: u64,
        royalty_denominator: u64,
        token_base_name: String,        // "Token #" => "Token #1", "Token #2", etc.
    ) {
        // Create the creator object for the `admin` account.
        let constructor_ref = object::create_object_from_account(admin);
        let creator = object::generate_signer(&constructor_ref);
        let creator_addr = object::address_from_constructor_ref(&constructor_ref);
        // Create the delete and extend refs to store in the MintConfiguration resource that's on the object itself
        let delete_ref = object::generate_delete_ref(&constructor_ref);
        let extend_ref = object::generate_extend_ref(&constructor_ref);

        // Soulbound MintConfiguration
        object::disable_ungated_transfer(&object::generate_transfer_ref(&constructor_ref));

        // Store the address of the creator object in the package manager
        // The key is the normalized string representation of the admin's address
        package_manager::add_named_address(
            string_utils::to_string_with_canonical_addresses(&signer::address_of(admin)),
            creator_addr
        );
        aptos_token::create_collection(
            &creator,
            description,
            max_supply,
            name,
            uri,
            mutable_description,
            mutable_royalty,
            mutable_uri,
            mutable_token_description,
            mutable_token_name,
            mutable_token_properties,
            mutable_token_uri,
            tokens_burnable_by_creator,
            tokens_freezable_by_creator,
            royalty_numerator,
            royalty_denominator,
        );

        // Move the MintConfiguration resource to the creator object with minting disabled and empty metadata/token uris
        move_to(&creator, MintConfiguration {
            collection_name: name,
            collection_addr: collection::create_collection_address(&creator_addr, &name),
            max_supply,
            token_base_name,
            minting_enabled: false,
            extend_ref,
            delete_ref,
            token_uris: smart_vector::new(),
            metadata_table: smart_table::new(),
        });
    }

    // Add or update a tier in the allowlist
    // Note that you can do this while the minting machine is running, it is left up to the admin's discretion
    public entry fun upsert_tier(
        admin: &signer,
        tier_name: String,
        open_to_public: bool,
        price: u64,
        start_time: u64,
        end_time: u64,
        per_user_limit: u64,
    ) acquires MintConfiguration {
        let admin_addr = signer::address_of(admin);
        // You can only create an allowlist if minting hasn't begun
        assert!(get_collection_supply(admin_addr) == 0, error::invalid_state(EMINTING_ALREADY_BEGUN));
        allowlist::upsert_tier_config(&get_creator(admin_addr), tier_name, open_to_public, price, start_time, end_time, per_user_limit);
    }

    /// This function adds tokens indexed to a table with their uri as the key
    /// and pushes the token uri onto the vector of token_uris. This vector is used to
    /// pseudorandomly pop from later when minting.
    public entry fun add_tokens(
        admin: &signer,
        uris: vector<String>,
        descriptions: vector<String>,
        property_keys: vector<vector<String>>,
        property_values: vector<vector<vector<u8>>>,
        property_types: vector<vector<String>>,
        safe: bool,
    ) acquires MintConfiguration {
        let admin_addr = signer::address_of(admin);
        // You can only add token metadata if minting hasn't begun
        assert!(get_collection_supply(admin_addr) == 0, error::invalid_state(EMINTING_ALREADY_BEGUN));
        // Correct serialization of property maps is checked here
        // Note that this may seem like it could be covered in a unit test, but this only an issue because
        // the property maps are only checked for valid serialization upon mint. If you store an incorrectly serialized
        // property map value in the metadata table, it would not be caught until minting.
        if (safe) {
            verify_valid_property_maps(property_keys, property_values, property_types);
        };
        let mint_configuration = borrow_mut_config(admin_addr);
        let amount = vector::length(&uris);
        assert!(
            (amount == vector::length(&descriptions) &&
            amount == vector::length(&property_keys) &&
            amount == vector::length(&property_values) &&
            amount == vector::length(&property_types)),
            error::invalid_argument(EVECTOR_LENGTHS_INCONSISTENT)
        );
        let existing_metadata_length = smart_vector::length(&mint_configuration.token_uris);
        assert!(amount + existing_metadata_length <= mint_configuration.max_supply, error::invalid_argument(EMAXIMUM_SUPPLY_EXCEEDED));

        vector::enumerate_ref(&uris, |i, uri| {
            assert!(!smart_table::contains(&mint_configuration.metadata_table, *uri), error::invalid_argument(EDUPLICATE_TOKEN_URI));
            smart_table::add(&mut mint_configuration.metadata_table, *uri, TokenMetadata {
                description: *vector::borrow(&descriptions, i),
                property_keys: *vector::borrow(&property_keys, i),
                property_values: *vector::borrow(&property_values, i),
                property_types: *vector::borrow(&property_types, i),
            });
            // TODO: if we wanted to allow update later, could call upsert instead of add above,
            // but we would need to not push the token name onto the vector if it is an upsert.
            smart_vector::push_back(&mut mint_configuration.token_uris, *uri);
        });
    }

    public entry fun enable_minting(
        admin: &signer,
    ) acquires MintConfiguration {
        let admin_addr = signer::address_of(admin);
        assert_ready_for_launch(admin_addr);
        borrow_mut_config(admin_addr).minting_enabled = true;
    }

    #[view]
    /// Checks to see if everything has been set up and the minting machine can be enabled.
    /// This function is intended to avoid error codes and only return whether or not the mint machine is ready.
    public fun ready_for_launch(admin_addr: address): ReadyForLaunch acquires MintConfiguration {
        let ready_for_launch = ReadyForLaunch {
            mint_config_exists: false,
            allowlist_exists: false,
            has_valid_tier: false,
            collection_exists: false,
            metadata_complete: false,
        };

        if (!creator_exists(admin_addr)) { return ready_for_launch };
        ready_for_launch.mint_config_exists = true;

        let creator_addr = get_creator_addr(admin_addr);
        if (!allowlist::exists_at(creator_addr)) { return ready_for_launch };
        ready_for_launch.allowlist_exists = true;

        if (!allowlist::has_valid_tier(creator_addr)) { return ready_for_launch };
        ready_for_launch.has_valid_tier = true;

        if (!object::is_object(get_collection_addr(admin_addr))) { return ready_for_launch };
        ready_for_launch.collection_exists = true;

        let table_length = smart_table::length(&borrow_config(admin_addr).metadata_table);
        if (table_length != borrow_config(admin_addr).max_supply) { return ready_for_launch };
        ready_for_launch.metadata_complete = true;

        ready_for_launch
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //                                                                                      //
    //                                     post-launch                                      //
    //                                                                                      //
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////

    // Right now these functions need to be private entry, because they require single tx-specific context
    // Specifically, they need to declare a new auid_manager, otherwise it isn't possible to find the token
    // object address to transfer it to the receiver.
    // This also disallows abuse of the pseudo-random number generation based on the timestamp
    entry fun mint_multiple(
        receiver: &signer,
        admin_addr: address,
        amount: u64
    ) acquires MintConfiguration {
        let auids = auid_manager::create();
        let i = 0;
        while(i < amount) {
            mint_internal(receiver, admin_addr, &mut auids);
            i = i + 1;
        };
    }

    // Right now these functions need to be private entry, because they require single tx-specific context
    // Specifically, they need to declare a new auid_manager, otherwise it isn't possible to find the token
    // object address to transfer it to the receiver.
    // This also disallows abuse of the pseudo-random number generation based on the timestamp
    entry fun mint(
        receiver: &signer,
        admin_addr: address,
    ) acquires MintConfiguration {
        let auids = auid_manager::create();
        mint_internal(receiver, admin_addr, &mut auids);
    }

    /// Mint an NFT to a receiver who requests it as long as they are eligible to do so
    fun mint_internal(
        receiver: &signer,
        admin_addr: address,
        auids: &mut AuidManager,
    ) acquires MintConfiguration {
        // must come first due to borrow
        let creator = get_creator(admin_addr);

        let mint_configuration = borrow_mut_config(admin_addr);
        assert!(mint_configuration.minting_enabled, error::invalid_state(EMINTING_DISABLED));
        let tokens_left = smart_vector::length(&mint_configuration.token_uris);
        assert!(tokens_left > 0, error::invalid_argument(EMAXIMUM_SUPPLY_EXCEEDED));

        // internally handles mint counter per user, price, tier, start time, end time, etc
        allowlist::increment(&creator, receiver);

        let metadata_table = &mint_configuration.metadata_table;

        // The timestamp is used as the "seed" in our pseudo-random number generation
        let now = timestamp::now_microseconds();

        let token_uris = &mut mint_configuration.token_uris;
        let idx = now % smart_vector::length(token_uris);

        let token_uri = smart_vector::swap_remove(token_uris, idx);
        // You could remove the uri borrowed from the token metadata table here, but to reduce the cost of minting, we put it off until later.
        // The metadata vector pops each value, so nothing will be reused.
        let token_metadata = smart_table::borrow(metadata_table, token_uri);

        // Build token name and token uri from base versions + popped token metadata
        // "token_base_name#{n}" is the name where n = (max supply - the # of tokens left)
        // that is: 0, 1, 2, ... max_supply - 1
        let full_token_name = concat_u64(mint_configuration.token_base_name, mint_configuration.max_supply - tokens_left);

        // mint token to the receiver
        aptos_token::mint(
            &creator,
            mint_configuration.collection_name,
            token_metadata.description,
            full_token_name,
            token_uri,
            token_metadata.property_keys,
            token_metadata.property_types,
            token_metadata.property_values,
        );

        // Increment the auid manager counter and get the newest token address
        let token_address = auid_manager::increment(auids);

        // Transfer the token object from the creator to the receiver/minter
        let token_object = object::address_to_object<AptosToken>(token_address);
        object::transfer(&creator, token_object, signer::address_of(receiver));
    }

    #[test_only]
    /// Specifically to facilitate testing so we can call mint multiple times in the same tx
    public fun mint_for_test(
        receiver: &signer,
        admin_addr: address,
        amount: u64,
        auids: &mut AuidManager,
    ) acquires MintConfiguration {
        let i = 0;
        while(i < amount) {
            mint_internal(receiver, admin_addr, auids);
            i = i + 1;
        };
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //                                                                                      //
    //                                     Getters/setters                                  //
    //                                                                                      //
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////

    #[view]
    public fun creator_exists(admin_addr: address): bool {
        let admin_addr_name = string_utils::to_string_with_canonical_addresses(&admin_addr);
        package_manager::named_address_exists(admin_addr_name)
    }

    #[view]
    /// The creator is the object that creates the collection and has the MintConfiguration resource
    /// and the Allowlist resource. Each account address can only have one allotted creator object.
    /// This is to simplify the contract since most people will never need more than one mint machine at a time.
    public fun get_creator_addr(admin_addr: address): address {
        let admin_addr_name = string_utils::to_string_with_canonical_addresses(&admin_addr);
        assert!(creator_exists(admin_addr), error::not_found(ECREATOR_OBJECT_NOT_FOUND));
        package_manager::get_named_address(admin_addr_name)
    }

    inline fun get_creator_obj<T: key>(admin_addr: address): Object<T> {
        object::address_to_object<T>(get_creator_addr(admin_addr))
    }

    inline fun get_creator(admin_addr: address): signer acquires MintConfiguration {
        object::generate_signer_for_extending(&borrow_config(admin_addr).extend_ref)
    }

    inline fun get_collection_name(admin_addr: address): String acquires MintConfiguration {
        borrow_config(admin_addr).collection_name
    }

    inline fun get_collection_addr(admin_addr: address): address acquires MintConfiguration {
        borrow_config(admin_addr).collection_addr
    }

    inline fun borrow_config(admin_addr: address): &MintConfiguration acquires MintConfiguration {
        let creator_addr = get_creator_addr(admin_addr);
        assert!(exists<MintConfiguration>(creator_addr), error::not_found(ECREATOR_OBJECT_NOT_FOUND));
        borrow_global<MintConfiguration>(creator_addr)
    }

    // We do not need to verify ownership because the creator object is soulbound to the admin
    inline fun borrow_mut_config(admin_addr: address): &mut MintConfiguration acquires MintConfiguration {
        let creator_addr = get_creator_addr(admin_addr);
        assert!(exists<MintConfiguration>(creator_addr), error::not_found(ECREATOR_OBJECT_NOT_FOUND));
        borrow_global_mut<MintConfiguration>(creator_addr)
    }

    inline fun get_collection_supply(admin_addr: address): u64 acquires MintConfiguration {
        std::option::extract(&mut collection::count(object::address_to_object<Collection>(get_collection_addr(admin_addr))))
    }

    inline fun borrow_token_uris(admin_addr: address): &SmartVector<String> acquires MintConfiguration {
        &borrow_config(admin_addr).token_uris
    }
    inline fun borrow_metadata_table(admin_addr: address): &SmartTable<String, TokenMetadata> acquires MintConfiguration {
        &borrow_config(admin_addr).metadata_table
    }

    // enable_minting is in setup section

    public entry fun disable_minting(
        admin: &signer,
    ) acquires MintConfiguration {
        borrow_mut_config(signer::address_of(admin)).minting_enabled = false;
    }

    public entry fun set_token_base_name(
        admin: &signer,
        token_base_name: String,
    ) acquires MintConfiguration {
        borrow_mut_config(signer::address_of(admin)).token_base_name = token_base_name;
    }

    // Returns the vector of all token uris in token metadata. Expensive call for large vectors.
    #[view]
    public fun view_token_uris(admin_addr: address): vector<String> acquires MintConfiguration {
        smart_vector::to_vector(borrow_token_uris(admin_addr))
    }

    // Returns the vector of token metadata corresponding to the input token_uris vector. Ordered based on input vector.
    #[view]
    public fun view_token_metadatas(admin_addr: address, token_uris: vector<String>): vector<TokenMetadata> acquires MintConfiguration {
        let metadata_table = borrow_metadata_table(admin_addr);
        let metadata = vector::map(token_uris, |token_uri| {
            assert!(smart_table::contains(metadata_table, token_uri), error::not_found(ETOKEN_METADATA_NOT_FOUND));
            *smart_table::borrow(metadata_table, token_uri)
        });
        metadata
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //                                                                                      //
    //                           Interfacing with other modules                             //
    //                                                                                      //
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////

    // upsert_tier is in setup section

    public entry fun add_to_tier(
        admin: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires MintConfiguration {
        allowlist::add_to_tier(&get_creator(signer::address_of(admin)), tier_name, addresses);
    }

    public entry fun remove_from_tier(
        admin: &signer,
        tier_name: String,
        addresses: vector<address>,
    ) acquires MintConfiguration {
        allowlist::remove_from_tier(&get_creator(signer::address_of(admin)), tier_name, addresses);
    }

    /// Verifies that inputs to a property map work calling `prepare_input` for each property_map
    public entry fun verify_valid_property_maps(
        outer_keys: vector<vector<String>>,
        outer_values: vector<vector<vector<u8>>>,
        outer_types: vector<vector<String>>,
    ) {
        vector::enumerate_ref(&outer_keys, |i, keys| {
            let values = vector::borrow(&outer_values, i);
            let types = vector::borrow(&outer_types, i);
            // create property map on our new object
            property_map::prepare_input(
                *keys,
                *types,
                *values,
            );
        });
    }

    /// Destroys the allowlist and disables minting.
    public entry fun destroy_allowlist(admin: &signer) acquires MintConfiguration {
        let admin_addr = signer::address_of(admin);
        let max_supply = borrow_config(admin_addr).max_supply;
        assert!(get_collection_supply(admin_addr) == max_supply, error::invalid_state(EINCOMPLETE_MINT));
        disable_minting(admin);
        allowlist::destroy(&get_creator(admin_addr));
    }

    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //                                                                                      //
    //                                    Utility functions                                 //
    //                                                                                      //
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////
    //////////////////////////////////////////////////////////////////////////////////////////

    public fun assert_ready_for_launch(admin_addr: address) acquires MintConfiguration {
        let creator_addr = get_creator_addr(admin_addr);
        allowlist::assert_exists(creator_addr);
        assert!(allowlist::has_valid_tier(creator_addr), error::not_found(ENO_VALID_TIERS));

        assert!(object::is_object(get_collection_addr(admin_addr)), error::not_found(ECOLLECTION_OBJECT_NOT_FOUND));
        let table_length = smart_table::length(&borrow_config(admin_addr).metadata_table);
        assert!(table_length == borrow_config(admin_addr).max_supply, error::invalid_argument(EINSUFFICIENT_METADATA));
    }

    public inline fun u64_to_string(value: u64): String {
        if (value == 0) {
            str(b"0")
        } else {
            let buffer = vector::empty<u8>();
            while (value != 0) {
                vector::push_back(&mut buffer, ((48 + value % 10) as u8));
                value = value / 10;
            };
            vector::reverse(&mut buffer);
            str(buffer)
        }
    }

    public inline fun concat_u64(s: String, n: u64): String {
        let n_str = u64_to_string(n);
        string::append(&mut s, n_str);
        s
    }
}

//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////
//                                                                                      //
//                                     Unit tests                                       //
//                                                                                      //
//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////
//////////////////////////////////////////////////////////////////////////////////////////

#[test_only]
module no_code_mint::unit_tests {
    use std::string::{String, utf8 as str};
    use no_code_mint::allowlist;
    use no_code_mint::package_manager;
    use no_code_mint::mint_machine;
    use no_code_mint::auid_manager::{Self, AuidManager};
    use std::account;
    use std::bcs;
    use std::vector;
    use std::signer;
    use aptos_token_objects::aptos_token::{AptosToken};
    use std::timestamp;
    use aptos_std::object;
    use aptos_std::aptos_coin::{AptosCoin};
    use aptos_std::coin;
    use aptos_token_objects::token::{Self, Token};
    const COLLECTION_DESCRIPTION: vector<u8> = b"Your collection description here!";
    const TOKEN_DESCRIPTION: vector<u8> = b"Your token description here!";
    const MUTABLE_COLLECTION_DESCRIPTION: bool = false;
    const MUTABLE_ROYALTY: bool = false;
    const MUTABLE_URI: bool = false;
    const MUTABLE_TOKEN_DESCRIPTION: bool = false;
    const MUTABLE_TOKEN_NAME: bool = false;
    const MUTABLE_TOKEN_PROPERTIES: bool = true;
    const MUTABLE_TOKEN_URI: bool = false;
    const TOKENS_BURNABLE_BY_CREATOR: bool = false;
    const TOKENS_FREEZABLE_BY_CREATOR: bool = false;
    const MINTER_STARTING_COINS: u64 = 100;
    const COLLECTION_NAME: vector<u8> = b"Krazy Kangaroos";
    const TOKEN_BASE_NAME: vector<u8> = b"Krazy Kangaroo #";
    const TOKEN_BASE_URI: vector<u8> = b"https://arweave.net/";
    const COLLECTION_URI: vector<u8> = b"https://www.link-to-your-collection-image.com";
    const ROYALTY_NUMERATOR: u64 = 5;
    const ROYALTY_DENOMINATOR: u64 = 100;
    const MAX_SUPPLY: u64 = 100;
    const START_TIMESTAMP_PUBLIC: u64 = 100000000;
    const START_TIMESTAMP_WHITELIST: u64 = 100000000 - 1;
    const END_TIMESTAMP_PUBLIC: u64 = 100000000 + 2;
    const PER_USER_LIMIT: u64 = 123;

    fun setup_test(
        admin: &signer,
        resource_signer: &signer,
        minter_1: &signer,
        aptos_framework: &signer,
        timestamp: u64,
    ): AuidManager {
        auid_manager::enable_auids_for_test(aptos_framework);

        timestamp::set_time_has_started_for_testing(aptos_framework);
        timestamp::update_global_time_for_test_secs(timestamp);
        account::create_account_for_test(signer::address_of(admin));
        account::create_account_for_test(signer::address_of(aptos_framework));
        std::resource_account::create_resource_account(admin, vector<u8> [], vector<u8> []);

        package_manager::init_module_for_test(resource_signer);

        let (burn, mint) = aptos_framework::aptos_coin::initialize_for_test(aptos_framework);
        allowlist::setup_account<AptosCoin>(minter_1, MINTER_STARTING_COINS, &mint);
        coin::destroy_burn_cap(burn);
        coin::destroy_mint_cap(mint);

        init_mint_machine_for_test(admin);

        auid_manager::create()
    }

    fun init_mint_machine_for_test(admin: &signer) {
        mint_machine::initialize_mint_machine(
            admin,
            str(COLLECTION_DESCRIPTION),
            MAX_SUPPLY,
            str(COLLECTION_NAME),
            str(COLLECTION_URI),
            MUTABLE_COLLECTION_DESCRIPTION,
            MUTABLE_ROYALTY,
            MUTABLE_URI,
            MUTABLE_TOKEN_DESCRIPTION,
            MUTABLE_TOKEN_NAME,
            MUTABLE_TOKEN_PROPERTIES,
            MUTABLE_TOKEN_URI,
            TOKENS_BURNABLE_BY_CREATOR,
            TOKENS_FREEZABLE_BY_CREATOR,
            ROYALTY_NUMERATOR,
            ROYALTY_DENOMINATOR,
            str(TOKEN_BASE_NAME),
        );
    }

    #[test(admin = @deployer, resource_signer = @no_code_mint, minter_1 = @0xAAAA, aptos_framework = @0x1)]
    fun test_happy_path(
        admin: &signer,
        resource_signer: &signer,
        minter_1: &signer,
        aptos_framework: &signer,
    ) {
        let admin_addr = signer::address_of(admin);
        let auids = setup_test(admin, resource_signer, minter_1, aptos_framework, START_TIMESTAMP_PUBLIC + 1);
        mint_machine::upsert_tier(
            admin,
            str(b"public"),
            true, // open to public
            1,
            START_TIMESTAMP_PUBLIC,
            END_TIMESTAMP_PUBLIC,
            PER_USER_LIMIT
        );

        add_test_metadata(admin, MAX_SUPPLY);
        mint_machine::assert_ready_for_launch(admin_addr);

        // collection is ready for launch, enable it!
        mint_machine::enable_minting(admin);

        let minter_1_addr = signer::address_of(minter_1);
        let allowlist_addr = mint_machine::get_creator_addr(admin_addr);
        allowlist::assert_eligible_for_tier(allowlist_addr, minter_1_addr, str(b"public"));

        let i = 0;
        while(i < MAX_SUPPLY) {
            mint_machine::mint_for_test(minter_1, admin_addr, 1, &mut auids);
            let aptos_token_object = object::address_to_object<AptosToken>(*auid_manager::get(&auids, i));
            assert!(object::is_owner(aptos_token_object, minter_1_addr), i);
            let token_object = object::convert<AptosToken, Token>(aptos_token_object);
            // NOTE: the name and uris will be randomly matched. This is because the vector of token_uris is randomly popped from
            // the minting machine. This would normally be obstructive for indexing the names => metadata after, but
            // since the object addresses are random (and can't be derived from creator+collection+name anymore),
            // we'll already be indexing by object address anyway.
            assert!(token::name(token_object) == mint_machine::concat_u64(str(TOKEN_BASE_NAME), i), i);
            i = i + 1;
        };

        // destroy the allowlist, only possible if all tokens have been minted
        mint_machine::destroy_allowlist(admin);
        assert!(!allowlist::exists_at(admin_addr), 0);
    }

    fun add_test_metadata(
        admin: &signer,
        n: u64
    ) {
        let uris = vector<String> [];
        let descriptions = vector<String> [];
        let property_keys = vector<vector<String>> [];
        let property_values = vector<vector<vector<u8>>> [];
        let property_types = vector<vector<String>> [];
        let base_token_uri = str(TOKEN_BASE_URI);

        let i = 0;
        while (i < n) {
            vector::push_back(&mut uris, mint_machine::concat_u64(base_token_uri, i));
            vector::push_back(&mut descriptions, str(TOKEN_DESCRIPTION));
            vector::push_back(&mut property_keys, vector<String> [
                str(b"key 1"),
                str(b"key 2"),
                str(b"key 3"),
                str(b"key 4"),
                str(b"key 5"),
            ]);
            vector::push_back(&mut property_values, vector<vector<u8>> [
                bcs::to_bytes(&str(b"value 1")),
                bcs::to_bytes(&str(b"value 2")),
                bcs::to_bytes(&str(b"value 3")),
                bcs::to_bytes(&9001),
                bcs::to_bytes(&true),
            ]);
            vector::push_back(&mut property_types, vector<String> [
                str(b"0x1::string::String"),
                str(b"0x1::string::String"),
                str(b"0x1::string::String"),
                str(b"u64"),
                str(b"bool"),
            ]);
            i = i + 1;
        };
        mint_machine::add_tokens(
            admin,
            uris,
            descriptions,
            property_keys,
            property_values,
            property_types,
            false
        );
    }
}
