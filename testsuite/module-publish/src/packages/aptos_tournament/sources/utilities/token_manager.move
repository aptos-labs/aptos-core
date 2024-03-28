module tournament::token_manager {
    use std::bcs;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_framework::event;
    use aptos_framework::object::{Self, Object, ObjectCore};

    use tournament::object_refs;
    use tournament::token_uris;

    /// The account is not authorized to update the resources
    const ENOT_AUTHORIZED: u64 = 1;
    /// The player name is too long: max 20
    const EPLAYER_NAME_TOO_LONG: u64 = 2;
    /// The player name is too short: min 1
    const EPLAYER_NAME_TOO_SHORT: u64 = 3;

    friend tournament::tournament_manager;
    friend tournament::rewards;

    struct CollectionConfig has key {
        creator_addr: address,
        collection_addr: address,
    }

    #[event]
    struct BurnPlayerTokenEvent has drop, store {
        round_number: u64,
        object_address: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TournamentPlayerToken has key, drop {
        tournament_address: address,
        player_name: String,
        token_uri: String,
    }

    /// Creates a single collection for the entire contract
    /// This is open to discussion
    fun init_module(
        deployer: &signer,
    ) {
        let deployer_addr = signer::address_of(deployer);
        assert!(deployer_addr == @tournament, ENOT_AUTHORIZED);
        let constructor_ref = object::create_object(deployer_addr);
        let (_obj_signer, obj_addr) = object_refs::create_refs<CollectionConfig>(&constructor_ref);
        move_to(deployer, CollectionConfig {
            creator_addr: obj_addr,
            collection_addr: object::address_from_constructor_ref(&constructor_ref),
        });
    }

    public(friend) fun mint(
        mint_to: address,
        tournament_address: address,
        player_name: String,
    ): Object<TournamentPlayerToken> acquires CollectionConfig {
        let collection_config = borrow_global<CollectionConfig>(@tournament);
        let obj_signer = object_refs::get_signer(collection_config.creator_addr);
        assert!(string::length(&player_name) < 20, EPLAYER_NAME_TOO_LONG);
        assert!(string::length(&player_name) > 0, EPLAYER_NAME_TOO_SHORT);

        let unique_token_seed = bcs::to_bytes(&tournament_address);
        vector::append(&mut unique_token_seed, bcs::to_bytes(&mint_to));
        let seed = aptos_std::aptos_hash::blake2b_256(unique_token_seed);

        let constructor_ref = object::create_named_object(&obj_signer, seed);

        let (object_signer, object_addr) = object_refs::create_refs<ObjectCore>(&constructor_ref);

        // Transfers the token to the `claimer` address
        let linear_transfer_ref = object_refs::get_linear_transfer_ref(object_addr);
        object::transfer_with_ref(linear_transfer_ref, mint_to);
        // Add the traits to the object
        let tournament_token = TournamentPlayerToken {
            tournament_address,
            player_name,
            token_uri: token_uris::get_random_token_uri(),
        };
        // move tournament_token to the token
        move_to(&object_signer, tournament_token);
        object::object_from_constructor_ref(&constructor_ref)
    }

    /// If player loses we need to burn the NFT
    public fun mark_token_loss(
        owner: &signer,
        token_address: address,
        current_round: u64,
    ) acquires TournamentPlayerToken {
        assert_is_admin(owner, token_address);
        mark_token_loss_internal(token_address, current_round);
    }

    public(friend) fun mark_token_loss_internal(token_address: address, current_round: u64) acquires TournamentPlayerToken {
        move_from<TournamentPlayerToken>(token_address);
        event::emit(BurnPlayerTokenEvent {
            object_address: token_address,
            round_number: current_round,
        });
        object_refs::destroy_object(token_address);
    }

    public fun get_token_signer(
        owner: &signer,
        token_address: address,
    ): signer acquires TournamentPlayerToken {
        assert_is_admin(owner, token_address);
        object_refs::get_signer(token_address)
    }

    public fun get_tournament_address(
        token_addr: address
    ): address acquires TournamentPlayerToken {
        let tournament_token = borrow_global<TournamentPlayerToken>(token_addr);
        tournament_token.tournament_address
    }

    public fun has_player_token(token_address: address): bool {
        exists<TournamentPlayerToken>(token_address)
    }

    public fun assert_is_admin(
        admin: &signer,
        token_address: address,
    ) acquires TournamentPlayerToken {
        let tournament_token = borrow_global_mut<TournamentPlayerToken>(token_address);
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

    #[test_only]
    public fun init_module_for_test(deployer: &signer) {
        init_module(deployer);
    }
}
