module tournament::round {

    use std::option::Option;
    use std::signer;
    use aptos_framework::object;
    use aptos_framework::object::Object;

    use aptos_token_objects::token::Token;

    use tournament::matchmaker;
    use tournament::object_refs;

    /// The signer is not the owner of the Round
    const ENOT_ROUND_OWNER: u64 = 0;
    /// The Round has not been started yet
    const EROUND_NOT_STARTED: u64 = 1;
    /// Play is not currently allowed
    const EPLAY_NOT_ALLOWED: u64 = 2;
    /// The Round has already ended
    const EROUND_ALREADY_ENDED: u64 = 3;
    /// Play has already started
    const EPLAY_ALREADY_STARTED: u64 = 4;
    /// Matchmaking has already started
    const EMATCHMAKING_ALREADY_STARTED: u64 = 5;
    /// Matchmaking has already ended
    const EMATCHMAKING_ALREADY_ENDED: u64 = 6;
    /// Can not end matchmaking before it has started
    const EMATCHMAKING_NOT_STARTED: u64 = 7;
    /// Can not end play before it has started
    const EPLAY_NOT_STARTED: u64 = 8;
    /// Play is already ended
    const EPLAY_ALREADY_ENDED: u64 = 9;
    /// Matchmaking is not currently allowed
    const EMATCHMAKING_NOT_ALLOWED: u64 = 10;


    // This is basically a state machine for the flow of a given round
    // First matchmaking is started, then matchmaking is ended, then play is started, then play is ended
    // After the round is over and all players have moved on, we can clean it up and delete the Round/Matchmaker/Rooms
    // It's valid for both matchmaking and play to be allowed at the same time!
    // These booleans are here to make it easier to check in indexing/UI
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Round<phantom GameType> has key {
        number: u64,
        matchmaking_ended: bool,
        play_started: bool,
        play_ended: bool,
        paused: bool,
        matchmaker_address: address,
    }

    #[view]
    public fun is_play_allowed<GameType>(round_address: address): bool acquires Round {
        let round = borrow_global<Round<GameType>>(round_address);
        round.play_started && !round.play_ended && !round.paused
    }

    #[view]
    public fun can_player_join<GameType>(round_address: address): bool acquires Round {
        let round = borrow_global<Round<GameType>>(round_address);
        !round.matchmaking_ended && !round.paused
    }

    #[view]
    public fun round_is_paused<GameType>(round_address: address): bool acquires Round {
        let round = borrow_global<Round<GameType>>(round_address);
        round.paused
    }

    #[view]
    public fun get_matchmaker_address<GameType>(round_address: address): address acquires Round {
        let round = borrow_global<Round<GameType>>(round_address);
        round.matchmaker_address
    }

    #[view]
    public fun get_tournament_address<GameType>(round_address: address): address {
        let round = object::address_to_object<Round<GameType>>(round_address);
        object::owner(round)
    }

    // Returns the round_signer, matchmaker_signer, and Option<unlimited_room_signer>
    // The unlimited room signer is only returned if max_players_per_room is 0: i.e we want all players in one room
    public fun create_round<GameType>(
        owner: &signer,
        number: u64,
        min_players_per_room: u64,
        max_players_per_room: u64,
    ): (signer, signer, Option<signer>) {
        let (matchmaker_signer, unlimited_room_signer) = if (max_players_per_room == 0) {
            matchmaker::create_unlimited_matchmaker<GameType>(owner)
        } else {
            matchmaker::create_limited_matchmaker<GameType>(owner, min_players_per_room, max_players_per_room)
        };

        let round = Round<GameType> {
            number,
            matchmaking_ended: false,
            play_started: false,
            play_ended: false,
            paused: false,
            matchmaker_address: signer::address_of(&matchmaker_signer),
        };

        let owner_addr = signer::address_of(owner);
        let constructor_ref = object::create_object(owner_addr);
        let (round_signer, _round_addr) = object_refs::create_refs<Round<GameType>>(&constructor_ref);
        move_to(&round_signer, round);

        (round_signer, matchmaker_signer, unlimited_room_signer)
    }

    public fun add_players<GameType>(
        owner: &signer,
        round_address: address,
        players: vector<Object<Token>>,
    ): Option<vector<signer>> acquires Round {
        assert_round_owner<GameType>(owner, round_address);

        assert_player_can_join<GameType>(round_address);

        let matchmaker_address = get_matchmaker_address<GameType>(round_address);
        matchmaker::add_players<GameType>(owner, matchmaker_address, players)
    }

    public entry fun end_matchmaking<GameType>(
        owner: &signer,
        round_address: address,
    ) acquires Round {
        assert_round_owner<GameType>(owner, round_address);

        let round = borrow_global_mut<Round<GameType>>(round_address);
        round.matchmaking_ended = true;
        matchmaker::finish_matchmaking<GameType>(owner, round.matchmaker_address);
    }

    public entry fun start_play<GameType>(
        owner: &signer,
        round_address: address,
    ) acquires Round {
        assert_round_owner<GameType>(owner, round_address);

        let round = borrow_global_mut<Round<GameType>>(round_address);
        assert!(!round.play_started, EPLAY_ALREADY_STARTED);
        assert!(!round.play_ended, EPLAY_ALREADY_ENDED);
        round.play_started = true;
    }

    public entry fun end_play<GameType>(
        owner: &signer,
        round_address: address,
    ) acquires Round {
        assert_round_owner<GameType>(owner, round_address);

        let round = borrow_global_mut<Round<GameType>>(round_address);
        assert!(round.play_started, EPLAY_NOT_STARTED);
        round.play_ended = true;
    }

    public fun destroy_and_cleanup_round<GameType>(
        owner: &signer,
        round_address: address,
    ) acquires Round {
        assert_round_owner<GameType>(owner, round_address);

        let Round<GameType> {
            number: _,
            matchmaking_ended: _,
            play_started: _,
            play_ended: _,
            paused: _,
            matchmaker_address,
        } = move_from<Round<GameType>>(round_address);
        object_refs::destroy_object(round_address);
        matchmaker::destroy_matchmaker<GameType>(matchmaker_address);
    }

    public fun get_round_signer<GameType>(owner: &signer, round_address: address): signer {
        assert_round_owner<GameType>(owner, round_address);

        object_refs::get_signer(round_address)
    }

    public fun assert_player_can_play<GameType>(round_address: address) acquires Round {
        assert!(is_play_allowed<GameType>(round_address), EPLAY_NOT_ALLOWED);
    }

    public fun assert_player_can_join<GameType>(round_address: address) acquires Round {
        assert!(can_player_join<GameType>(round_address), EMATCHMAKING_NOT_ALLOWED);
    }

    public fun assert_round_owner<GameType>(owner: &signer, round_address: address) {
        let round = object::address_to_object<Round<GameType>>(round_address);
        assert!(object::owns(round, signer::address_of(owner)), ENOT_ROUND_OWNER);
    }
}
