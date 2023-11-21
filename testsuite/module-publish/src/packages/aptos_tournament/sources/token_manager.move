module tournament::token_manager {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use aptos_framework::object::{Self, Object, ObjectCore};

    use aptos_token_objects::collection;
    use aptos_token_objects::property_map;
    use aptos_token_objects::token::{Self, Token};

    use tournament::object_refs;
    use tournament::token_uris;

    /// The account is not authorized to update the resources
    const ENOT_AUTHORIZED: u64 = 1;
    /// The player name is too long: max 20
    const EPLAYER_NAME_TOO_LONG: u64 = 2;

    const COLLECTION_NAME: vector<u8> = b"The Game";
    const COLLECTION_DESCRIPTION: vector<u8> = b"Welcome to THE GAME - An interactive, risk based, gamified and social experience on Aptos. Are you going to be the last person standing?";
    const COLLECTION_URI: vector<u8> = b"https://storage.googleapis.com/space-fighters-assets/game_collection.png";

    const TOKEN_NAME: vector<u8> = b"Player";
    const TOKEN_DESCRIPTION: vector<u8> = b"Welcome to THE GAME - An interactive, risk based, gamified and social experience on Aptos. Are you going to be the last person standing?";

    friend tournament::tournament_manager;

    struct CollectionConfig has key {
        creator_addr: address,
        collection_addr: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TournamentToken has key, drop {
        last_recorded_round: u64,
        tournament_address: address,
        room_address: address,
    }

    /// Creates a single collection for the entire contract
    /// This is open to discussion
    fun init_module(
        deployer: &signer,
    ) {
        let deployer_addr = signer::address_of(deployer);
        assert!(deployer_addr == @tournament, ENOT_AUTHORIZED);
        let constructor_ref = object::create_object(deployer_addr);
        let (obj_signer, obj_addr) = object_refs::create_refs<CollectionConfig>(&constructor_ref);
        let constructor_ref = collection::create_unlimited_collection(
            &obj_signer,
            string::utf8(COLLECTION_DESCRIPTION),
            string::utf8(COLLECTION_NAME),
            option::none(),
            string::utf8(COLLECTION_URI),
        );
        move_to(deployer, CollectionConfig {
            creator_addr: obj_addr,
            collection_addr: object::address_from_constructor_ref(&constructor_ref),
        });
    }

    public(friend) fun mint(
        mint_to: address,
        tournament_address: address,
        player_name: String,
    ): Object<Token> acquires CollectionConfig {
        let collection_config = borrow_global<CollectionConfig>(@tournament);
        let obj_signer = object_refs::get_signer(collection_config.creator_addr);
        assert!(string::length(&player_name) < 20, EPLAYER_NAME_TOO_LONG);
        let constructor_ref = token::create(
            &obj_signer,
            string::utf8(COLLECTION_NAME),
            string::utf8(TOKEN_DESCRIPTION),
            player_name,
            option::none(),
            token_uris::get_random_token_uri(),
        );
        let (object_signer, object_addr) = object_refs::create_refs<Token>(&constructor_ref);

        // Transfers the token to the `claimer` address
        let linear_transfer_ref = object_refs::get_linear_transfer_ref(object_addr);
        object::transfer_with_ref(linear_transfer_ref, mint_to);
        // Add the traits to the object
        let tournament_token = TournamentToken {
            last_recorded_round: 0,
            tournament_address,
            room_address: @0x0,
        };
        // move tournament_token to the token
        move_to(&object_signer, tournament_token);
        object::object_from_constructor_ref(&constructor_ref)
    }

    /// If player loses we need to burn the NFT
    public fun mark_token_loss(
        owner: &signer,
        token_address: address,
    ) acquires TournamentToken {
        assert_is_admin(owner, token_address);
        let (burn_ref, property_mutator_ref) = object_refs::destroy_for_token(token_address);
        property_map::burn(property_mutator_ref);
        token::burn(burn_ref);
        move_from<TournamentToken>(token_address);
    }

    public fun get_token_signer(
        owner: &signer,
        token_address: address,
    ): signer acquires TournamentToken {
        assert_is_admin(owner, token_address);
        object_refs::get_signer(token_address)
    }

    public fun get_tournament_address(
        token_addr: address
    ): address acquires TournamentToken {
        let tournament_token = borrow_global<TournamentToken>(token_addr);
        tournament_token.tournament_address
    }

    public fun assert_is_admin(
        admin: &signer,
        token_address: address,
    ) acquires TournamentToken {
        let tournament_token = borrow_global_mut<TournamentToken>(token_address);
        let tournament_object = object::address_to_object<ObjectCore>(tournament_token.tournament_address);
        assert!(object::owns(tournament_object, signer::address_of(admin)), ENOT_AUTHORIZED);
    }

    struct CollectionConfigView has copy, drop, store {
        creator_addr: address,
        collection_addr: address,
    }

    #[view]
    public fun get_collection_config(): CollectionConfigView acquires CollectionConfig {
        let collection_config = borrow_global<CollectionConfig>(@tournament);
        CollectionConfigView {
            creator_addr: collection_config.creator_addr,
            collection_addr: collection_config.collection_addr,
        }
    }

    public fun init_module_for_test(deployer: &signer) {
        init_module(deployer);
    }
}