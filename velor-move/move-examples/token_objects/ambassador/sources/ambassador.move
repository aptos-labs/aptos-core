/// This module is used to create ambassador tokens which are example soulbound tokens.
/// A collection for ambassador tokens is created when the module is published and initialized.
/// The creator of the collection is the only one who can mint and burn ambassador tokens.
/// Ambassador tokens are souldbound, thus non-transferable. Each ambassador token has a custom attribute
/// called level. The level of a newly minted token is 0, and can be updated by the creator.
/// Whenever the level of a token is updated, an event called LevelUpdate is emitted.
/// Each ambassador token has another custom attribute called rank, which is associated with the level.
/// The rank is determined by the level such that the rank is Bronze if the level is between 0 and 9,
/// Silver if the level is between 10 and 19, and Gold if the level is 20 or greater.
/// The rank is stored in the property map, thus displayed in a wallet as a trait of the token.
/// The token uri is the concatenation of the base uri and the rank, where the base uri is given
/// as an argument of the minting function. So, the token uri changes when the rank changes.
module ambassador::ambassador {
    use std::error;
    use std::option;
    use std::string::{Self, String};
    use std::signer;

    use velor_framework::object::{Self, Object};
    use velor_token_objects::collection;
    use velor_token_objects::token;
    use velor_token_objects::property_map;
    use velor_framework::event;
    use velor_std::string_utils::{to_string};

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

    /// The ambassador token collection name
    const COLLECTION_NAME: vector<u8> = b"Ambassador Collection Name";
    /// The ambassador token collection description
    const COLLECTION_DESCRIPTION: vector<u8> = b"Ambassador Collection Description";
    /// The ambassador token collection URI
    const COLLECTION_URI: vector<u8> = b"Ambassador Collection URI";

    /// The ambassador rank
    const RANK_GOLD: vector<u8> = b"Gold";
    const RANK_SILVER: vector<u8> = b"Silver";
    const RANK_BRONZE: vector<u8> = b"Bronze";

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// The ambassador token
    struct AmbassadorToken has key {
        /// Used to mutate the token uri
        mutator_ref: token::MutatorRef,
        /// Used to burn.
        burn_ref: token::BurnRef,
        /// Used to mutate properties
        property_mutator_ref: property_map::MutatorRef,
        /// the base URI of the token
        base_uri: String,
    }

    #[resource_group_member(group = velor_framework::object::ObjectGroup)]
    /// The ambassador level
    struct AmbassadorLevel has key {
        ambassador_level: u64,
    }

    #[event]
    /// The ambassador level update event
    struct LevelUpdate has drop, store {
        token: Object<AmbassadorToken>,
        old_level: u64,
        new_level: u64,
    }

    /// Initializes the module, creating the ambassador collection. The creator of the module is the creator of the
    /// ambassador collection. As this init function is called only once when the module is published, there will
    /// be only one ambassador collection.
    fun init_module(sender: &signer) {
        create_ambassador_collection(sender);
    }

    #[view]
    /// Returns the ambassador level of the token
    public fun ambassador_level(token: Object<AmbassadorToken>): u64 acquires AmbassadorLevel {
        let ambassador_level = borrow_global<AmbassadorLevel>(object::object_address(&token));
        ambassador_level.ambassador_level
    }

    #[view]
    /// Returns the ambassador rank of the token
    public fun ambassador_rank(token: Object<AmbassadorToken>): String {
        property_map::read_string(&token, &string::utf8(b"Rank"))
    }

    #[view]
    /// Returns the ambassador level of the token of the address
    public fun ambassador_level_from_address(addr: address): u64 acquires AmbassadorLevel {
        let token = object::address_to_object<AmbassadorToken>(addr);
        ambassador_level(token)
    }

    #[view]
    /// Returns the ambassador rank of the token of the address
    public fun ambassador_rank_from_address(addr: address): String {
        let token = object::address_to_object<AmbassadorToken>(addr);
        ambassador_rank(token)
    }

    /// Creates the ambassador collection. This function creates a collection with unlimited supply using
    /// the module constants for description, name, and URI, defined above. The collection will not have
    /// any royalty configuration because the tokens in this collection will not be transferred or sold.
    fun create_ambassador_collection(creator: &signer) {
        // Constructs the strings from the bytes.
        let description = string::utf8(COLLECTION_DESCRIPTION);
        let name = string::utf8(COLLECTION_NAME);
        let uri = string::utf8(COLLECTION_URI);

        // Creates the collection with unlimited supply and without establishing any royalty configuration.
        collection::create_unlimited_collection(
            creator,
            description,
            name,
            option::none(),
            uri,
        );
    }

    /// Mints an ambassador token. This function mints a new ambassador token and transfers it to the
    /// `soul_bound_to` address. The token is minted with level 0 and rank Bronze.
    public entry fun mint_ambassador_token(
        creator: &signer,
        description: String,
        name: String,
        base_uri: String,
        soul_bound_to: address,
    ) {
        mint_ambassador_token_impl(creator, description, name, base_uri, soul_bound_to, false);
    }

    /// Mints an ambassador token. This function mints a new ambassador token and transfers it to the
    /// `soul_bound_to` address. The token is minted with level 0 and rank Bronze.
    public entry fun mint_numbered_ambassador_token(
        creator: &signer,
        description: String,
        name: String,
        base_uri: String,
        soul_bound_to: address,
    ) {
        mint_ambassador_token_impl(creator, description, name, base_uri, soul_bound_to, true);
    }

    /// Function used for benchmarking.
    /// Uses multisig to mint to user, with creator permissions.
    /// Uses users address as unique name of the soulbound token.
    public entry fun mint_ambassador_token_by_user(
        user: &signer,
        creator: &signer,
        description: String,
        uri: String,
    ) {
        let user_addr = signer::address_of(user);
        mint_ambassador_token(creator, description, to_string<address>(&user_addr), uri, user_addr);
    }

    /// Function used for benchmarking.
    /// Uses multisig to mint to user, with creator permissions.
    public entry fun mint_numbered_ambassador_token_by_user(
        user: &signer,
        creator: &signer,
        description: String,
        name: String,
        uri: String,
    ) {
        mint_numbered_ambassador_token(creator, description, name, uri, signer::address_of(user));
    }

    /// Mints an ambassador token. This function mints a new ambassador token and transfers it to the
    /// `soul_bound_to` address. The token is minted with level 0 and rank Bronze.
    fun mint_ambassador_token_impl(
        creator: &signer,
        description: String,
        name: String,
        base_uri: String,
        soul_bound_to: address,
        numbered: bool,
    ) {
        // The collection name is used to locate the collection object and to create a new token object.
        let collection = string::utf8(COLLECTION_NAME);
        // Creates the ambassador token, and get the constructor ref of the token. The constructor ref
        // is used to generate the refs of the token.
        let uri = base_uri;
        string::append(&mut uri, string::utf8(RANK_BRONZE));
        let constructor_ref = if (numbered) {
            token::create_numbered_token(
                creator,
                collection,
                description,
                name,
                string::utf8(b""),
                option::none(),
                uri,
            )
        } else {
            token::create_named_token(
                creator,
                collection,
                description,
                name,
                option::none(),
                uri,
            )
        };

        // Generates the object signer and the refs. The object signer is used to publish a resource
        // (e.g., AmbassadorLevel) under the token object address. The refs are used to manage the token.
        let object_signer = object::generate_signer(&constructor_ref);
        let transfer_ref = object::generate_transfer_ref(&constructor_ref);
        let mutator_ref = token::generate_mutator_ref(&constructor_ref);
        let burn_ref = token::generate_burn_ref(&constructor_ref);
        let property_mutator_ref = property_map::generate_mutator_ref(&constructor_ref);

        // Transfers the token to the `soul_bound_to` address
        let linear_transfer_ref = object::generate_linear_transfer_ref(&transfer_ref);
        object::transfer_with_ref(linear_transfer_ref, soul_bound_to);

        // Disables ungated transfer, thus making the token soulbound and non-transferable
        object::disable_ungated_transfer(&transfer_ref);

        // Initializes the ambassador level as 0
        move_to(&object_signer, AmbassadorLevel { ambassador_level: 0 });

        // Initialize the property map and the ambassador rank as Bronze
        let properties = property_map::prepare_input(vector[], vector[], vector[]);
        property_map::init(&constructor_ref, properties);
        property_map::add_typed(
            &property_mutator_ref,
            string::utf8(b"Rank"),
            string::utf8(RANK_BRONZE)
        );

        // Publishes the AmbassadorToken resource with the refs.
        let ambassador_token = AmbassadorToken {
            mutator_ref,
            burn_ref,
            property_mutator_ref,
            base_uri
        };
        move_to(&object_signer, ambassador_token);
    }

    /// Burns an ambassador token. This function burns the ambassador token and destroys the
    /// AmbassadorToken resource, AmbassadorLevel resource, the event handle, and the property map.
    public entry fun burn(creator: &signer, token: Object<AmbassadorToken>) acquires AmbassadorToken, AmbassadorLevel {
        authorize_creator(creator, &token);
        let ambassador_token = move_from<AmbassadorToken>(object::object_address(&token));
        let AmbassadorToken {
            mutator_ref: _,
            burn_ref,
            property_mutator_ref,
            base_uri: _
        } = ambassador_token;

        let AmbassadorLevel {
            ambassador_level: _
        } = move_from<AmbassadorLevel>(object::object_address(&token));

        property_map::burn(property_mutator_ref);
        token::burn(burn_ref);
    }

    /// Function used for benchmarking.
    /// Uses multisig to mint to user, with creator permissions.
    /// Uses users address as unique name of the soulbound token.
    /// Burns token that was minted by mint_ambassador_token_by_user
    public entry fun burn_named_by_user(user: &signer, creator: &signer) acquires AmbassadorToken, AmbassadorLevel {
        let collection_name = string::utf8(COLLECTION_NAME);
        let token_address = token::create_token_address(
            &signer::address_of(creator),
            &collection_name,
            &to_string<address>(&signer::address_of(user)),
        );
        let token = object::address_to_object<AmbassadorToken>(token_address);
        burn(creator, token);
    }

    /// Sets the ambassador level of the token. Only the creator of the token can set the level. When the level
    /// is updated, the `LevelUpdate` is emitted. The ambassador rank is updated based on the new level.
    public entry fun set_ambassador_level(
        creator: &signer,
        token: Object<AmbassadorToken>,
        new_ambassador_level: u64
    ) acquires AmbassadorLevel, AmbassadorToken {
        // Asserts that `creator` is the creator of the token.
        authorize_creator(creator, &token);

        let token_address = object::object_address(&token);
        let ambassador_level = borrow_global_mut<AmbassadorLevel>(token_address);
        // Emits the `LevelUpdate`.
        event::emit(
            LevelUpdate {
                token,
                old_level: ambassador_level.ambassador_level,
                new_level: new_ambassador_level,
            }
        );
        // Updates the ambassador level.
        ambassador_level.ambassador_level = new_ambassador_level;
        // Updates the ambassador rank based on the new level.
        update_ambassador_rank(token, new_ambassador_level);
    }

    /// Updates the ambassador rank of the token based on the new level
    fun update_ambassador_rank(
        token: Object<AmbassadorToken>,
        new_ambassador_level: u64
    ) acquires AmbassadorToken {
        // `new_rank` is determined based on the new level.
        let new_rank = if (new_ambassador_level < 10) {
            RANK_BRONZE
        } else if (new_ambassador_level < 20) {
            RANK_SILVER
        } else {
            RANK_GOLD
        };

        let token_address = object::object_address(&token);
        let ambassador_token = borrow_global<AmbassadorToken>(token_address);
        // Gets `property_mutator_ref` to update the rank in the property map.
        let property_mutator_ref = &ambassador_token.property_mutator_ref;
        // Updates the rank in the property map.
        property_map::update_typed(property_mutator_ref, &string::utf8(b"Rank"), string::utf8(new_rank));
        // Updates the token URI based on the new rank.
        let uri = ambassador_token.base_uri;
        string::append(&mut uri, string::utf8(new_rank));
        token::set_uri(&ambassador_token.mutator_ref, uri);
    }

    /// Authorizes the creator of the token. Asserts that the token exists and the creator of the token
    /// is `creator`.
    inline fun authorize_creator<T: key>(creator: &signer, token: &Object<T>) {
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

    #[test(creator = @0x123, user1 = @0x456)]
    fun test_mint_burn(creator: &signer, user1: &signer) acquires AmbassadorToken, AmbassadorLevel {
        // ------------------------------------------
        // Creator creates the Ambassador Collection.
        // ------------------------------------------
        create_ambassador_collection(creator);

        // -------------------------------------------
        // Creator mints a Ambassador token for User1.
        // -------------------------------------------
        let token_name = string::utf8(b"Ambassador Token #1");
        let token_description = string::utf8(b"Ambassador Token #1 Description");
        let token_uri = string::utf8(b"Ambassador Token #1 URI/");
        let user1_addr = signer::address_of(user1);
        // Creates the Ambassador token for User1.
        mint_ambassador_token(
            creator,
            token_description,
            token_name,
            token_uri,
            user1_addr,
        );
        let collection_name = string::utf8(COLLECTION_NAME);
        let token_address = token::create_token_address(
            &signer::address_of(creator),
            &collection_name,
            &token_name
        );
        let token = object::address_to_object<AmbassadorToken>(token_address);
        // Asserts that the owner of the token is User1.
        assert!(object::owner(token) == user1_addr, 1);

        // -----------------------
        // Creator sets the level.
        // -----------------------
        // Asserts that the initial level of the token is 0.
        assert!(ambassador_level(token) == 0, 2);
        // Asserts that the initial rank of the token is "Bronze".
        assert!(ambassador_rank(token) == string::utf8(RANK_BRONZE), 3);
        assert!(token::uri(token) == string::utf8(b"Ambassador Token #1 URI/Bronze"), 4);
        // `creator` sets the level to 15.
        set_ambassador_level(creator, token, 15);
        // Asserts that the level is updated to 15.
        assert!(ambassador_level(token) == 15, 4);
        // Asserts that the rank is updated to "Silver" which is the expected rank for level 15.
        assert!(token::uri(token) == string::utf8(b"Ambassador Token #1 URI/Silver"), 5);

        // ------------------------
        // Creator burns the token.
        // ------------------------
        let token_addr = object::object_address(&token);
        // Asserts that the token exists before burning.
        assert!(exists<AmbassadorToken>(token_addr), 6);
        // Burns the token.
        burn(creator, token);
        // Asserts that the token does not exist after burning.
        assert!(!exists<AmbassadorToken>(token_addr), 7);
    }

    #[test(creator = @0x123, user1 = @0x456)]
    fun test_mint_burn_by_user(creator: &signer, user1: &signer) acquires AmbassadorToken, AmbassadorLevel {
        // ------------------------------------------
        // Creator creates the Ambassador Collection.
        // ------------------------------------------
        create_ambassador_collection(creator);

        let token_description = string::utf8(b"Ambassador Token #1 Description");
        let token_uri = string::utf8(b"Ambassador Token #1 URI/");
        mint_ambassador_token_by_user(
            user1,
            creator,
            token_description,
            token_uri
        );
        burn_named_by_user(user1, creator);
    }
}
