module tournament::room {

    use std::option::{Self, Option};
    use std::signer;
    use std::vector;
    use aptos_framework::object::{Self, Object};

    use aptos_token_objects::token::Token;

    use tournament::object_refs;

    friend tournament::matchmaker;

    /// Attempted to do something to a room that the signer does not own
    const ENOT_ROOM_OWNER: u64 = 0;
    /// Player is not in the room
    const EUNKNOWN_PLAYER: u64 = 1;
    /// This is not a limited room
    const ENOT_LIMITED_ROOM: u64 = 2;

    // This is stored at a random address: this is the instance of an individual game room
    // Like a table of poker players, two players doing 1v1 RPS, or everyone doing trivia, etc
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Room<phantom GameType> has key, drop {
        players: Option<vector<Object<Token>>>,
    }

    public fun get_players<GameType>(
        room_address: address,
    ): Option<vector<Object<Token>>> acquires Room {
        let room = borrow_global<Room<GameType>>(room_address);
        room.players
    }

    // Makes sure the player is in the room, and returns the player index and token address
    public fun assert_player_in_limited_room<GameType>(
        room_address: address,
        player_address: address,
    ): (u64, address) acquires Room {
        let room = borrow_global_mut<Room<GameType>>(room_address);
        assert!(option::is_some(&room.players), ENOT_LIMITED_ROOM);
        let players = option::borrow(&room.players);

        let i = 0;
        let len = vector::length(players);
        while (i < len) {
            let player = vector::borrow(players, i);
            if (object::is_owner(*player, player_address)) {
                return (i, object::object_address(player))
            };
            i = i + 1;
        };
        abort EUNKNOWN_PLAYER
    }

    public(friend) fun add_players<GameType>(
        room_address: address,
        players: vector<Object<Token>>,
    ) acquires Room {
        let room = borrow_global_mut<Room<GameType>>(room_address);
        if (option::is_some(&room.players)) {
            let player_arr = option::borrow_mut(&mut room.players);
            vector::reverse_append(player_arr, players);
        };
    }

    public(friend) fun create_room<GameType>(
        owner: &signer,
        is_limited_room: bool,
    ): signer {
        let tournament_addr = signer::address_of(owner);
        let constructor_ref = object::create_object(tournament_addr);
        let (room_obj_signer, _room_obj_addr) = object_refs::create_refs<Room<GameType>>(&constructor_ref);

        let players = if (is_limited_room) {
            option::some<vector<Object<Token>>>(vector::empty<Object<Token>>())
        } else {
            option::none<vector<Object<Token>>>()
        };
        move_to(&room_obj_signer, Room<GameType> { players });
        room_obj_signer
    }

    public fun get_tournament_address<GameType>(room_address: address): address {
        let room = object::address_to_object<Room<GameType>>(room_address);
        object::owner(room)
    }

    public fun close_room<GameType>(
        owner: &signer,
        room_address: address,
    ) acquires Room {
        let room = object::address_to_object<Room<GameType>>(room_address);
        assert!(object::owns(room, signer::address_of(owner)), ENOT_ROOM_OWNER);
        move_from<Room<GameType>>(room_address);
        object_refs::destroy_object(room_address);
    }

}
