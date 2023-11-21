module tournament::tournament_manager {
    use std::signer;
    use std::string::{Self, String};
    use aptos_std::type_info::type_name;
    use aptos_framework::object::{Self, Object};

    use aptos_token_objects::token::Token;

    use tournament::object_refs;
    use tournament::round;
    use tournament::token_manager;

    /// The account is not authorized to perform that action.
    const ENOT_AUTHORIZED: u64 = 1;
    /// Token is not owned by the user
    const ENOT_OWNER: u64 = 2;
    /// Pool must be empty to be cleared
    const ENOT_EMPTY: u64 = 3;
    /// Tournament is not playable
    const ENOT_PLAYABLE: u64 = 4;
    /// This is not a tournament address
    const ENOT_TOURNAMENT_ADDRESS: u64 = 5;

    const ETOURNAMENT_ALREADY_STARTED: u64 = 10;
    /// The tournament is full.
    const ETOURNAMENT_FULL: u64 = 11;
    /// The tournament is not joinable.
    const ETOURNAMENT_NOT_JOINABLE: u64 = 12;
    /// The tournament has already ended.
    const ETOURNAMENT_HAS_ENDED: u64 = 13;
    /// The tournament has not started.
    const ETOURNAMENT_NOT_STARTED: u64 = 14;
    /// A round does not exist.
    const EROUND_DOES_NOT_EXIST: u64 = 15;
    /// A tournament does not exist.
    const ETOURNAMENT_DOES_NOT_EXIST: u64 = 16;

    // This is the Object that will manage the configuration settings for a Tournament
    // and act as the interface layer for the tournament creator to manage the tournament state.
    // This will go in the Creator Object's resources, the Creator Object being the creator of the collection for the Tournament
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TournamentDirector has key {
        max_players: u64,
        max_num_winners: u64,
        players_joined: u64,
        tournament_name: String,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct CurrentRound has key {
        // Round '0' means it hasn't started yet
        number: u64,
        round_address: address,
        game_module: String,
    }

    // This is the structure the tournament creator will use to manage the tournament state.
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TournamentState has key {
        is_joinable: bool,
        has_ended: bool,
    }

    struct ViewTournamentState has copy, drop, store {
        max_num_winners: u64,
        is_joinable: bool,
        has_ended: bool,
        tournament_name: String,
        current_round_game_module: String,
        round_number: u64,
        round_address: address,
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                           Tournament initialization and setup                          //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    public entry fun initialize_tournament(
        tournament_creator: &signer,
        tournament_name: String,
        max_players: u64,
        max_num_winners: u64,
    ) {
        initialize_tournament_with_return(
            tournament_creator,
            tournament_name,
            max_players,
            max_num_winners,
        );
    }

    public fun initialize_tournament_with_return(
        tournament_creator: &signer,
        tournament_name: String,
        max_players: u64,
        max_num_winners: u64,
    ): (signer, address) {
        let tournament_creator_addr = signer::address_of(tournament_creator);

        let constructor_ref = object::create_object(tournament_creator_addr);
        let (tournament_director, tournament_address) = object_refs::create_refs<TournamentDirector>(&constructor_ref);
        move_to(
            &tournament_director,
            TournamentDirector {
                max_players,
                max_num_winners,
                players_joined: 0,
                tournament_name,
            },
        );

        move_to(
            &tournament_director,
            TournamentState {
                is_joinable: false,
                has_ended: false,
            },
        );

        move_to(
            &tournament_director,
            CurrentRound {
                number: 0,
                round_address: @0x0,
                game_module: string::utf8(b""),
            },
        );

        (tournament_director, tournament_address)
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                    Join tournament                                     //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    public entry fun join_tournament(
        player: &signer,
        tournament_address: address,
        player_name: String,
    ) acquires TournamentState, TournamentDirector {
        join_tournament_with_return(
            player,
            tournament_address,
            player_name,
        );
    }

    public fun join_tournament_with_return(
        player: &signer,
        tournament_address: address,
        player_name: String,
    ): Object<Token> acquires TournamentState, TournamentDirector {
        assert_tournament_not_ended(tournament_address);
        assert_tournament_is_joinable(tournament_address);
        let td = borrow_global_mut<TournamentDirector>(tournament_address);
        td.players_joined = td.players_joined + 1;

        // Mint please
        token_manager::mint(
            signer::address_of(player),
            tournament_address,
            player_name,
        )
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                               Managing the tournament state                            //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    public fun get_tournament_signer(
        admin: &signer,
        tournament_address: address,
    ): signer {
        assert!(
            is_admin_address(signer::address_of(admin), tournament_address),
            ENOT_AUTHORIZED
        );
        object_refs::get_signer(tournament_address)
    }

    public fun is_admin_address(caller: address, tournament_address: address): bool {
        assert!(exists<TournamentDirector>(tournament_address), ENOT_TOURNAMENT_ADDRESS);
        let tournament_object = object::address_to_object<TournamentDirector>(tournament_address);
        object::owns(tournament_object, caller)
    }

    // NOTE: owner of Object<TournamentDirector> is the tournament creator
    // Returns the round_signer, matchmaker_signer, and Option<unlimited_room_signer>
    // The unlimited room signer is only returned if max_players_per_room is 0: i.e we want all players in one room
    public fun start_new_round<GameType>(
        caller: &signer,
        tournament_address: address,
        min_players_per_room: u64,
        max_players_per_room: u64,
    ) acquires TournamentState, CurrentRound {
        assert_is_admin(signer::address_of(caller), tournament_address);
        assert_tournament_not_ended(tournament_address);

        let state = borrow_global_mut<TournamentState>(tournament_address);
        let current_round = borrow_global_mut<CurrentRound>(tournament_address);

        let tournament_signer = get_tournament_signer(caller, tournament_address);

        cleanup_current_round<GameType>(&tournament_signer, current_round);

        let (round_signer, _, _) = round::create_round<GameType>(
            &tournament_signer,
            current_round.number,
            min_players_per_room,
            max_players_per_room,
        );
        let round_address = signer::address_of(&round_signer);

        current_round.game_module = type_name<GameType>();
        current_round.number = current_round.number + 1;
        current_round.round_address = round_address;

        // Only allow joining at the start!
        state.is_joinable = false;
    }

    public entry fun set_tournament_joinable(
        caller: &signer,
        tournament_address: address,
    ) acquires TournamentState {
        assert_is_admin(signer::address_of(caller), tournament_address);
        assert_tournament_not_ended(tournament_address);

        let tournament_state = borrow_global_mut<TournamentState>(tournament_address);
        tournament_state.is_joinable = true;
    }

    public entry fun set_tournament_not_joinable(
        caller: &signer,
        tournament_address: address,
    ) acquires TournamentState {
        assert_is_admin(signer::address_of(caller), tournament_address);
        assert_tournament_not_ended(tournament_address);

        let tournament_state = borrow_global_mut<TournamentState>(tournament_address);
        tournament_state.is_joinable = false;
    }

    public entry fun end_tournament(
        caller: &signer,
        tournament_address: address,
    ) acquires TournamentState {
        assert_is_admin(signer::address_of(caller), tournament_address);

        let tournament_state = borrow_global_mut<TournamentState>(tournament_address);
        tournament_state.is_joinable = false;
        tournament_state.has_ended = true;
    }

    inline fun cleanup_current_round<GameType>(admin_signer: &signer, current_round: &mut CurrentRound) {
        // We need to do some cleanup!
        if (current_round.round_address != @0x0) {
            round::destroy_and_cleanup_round<GameType>(
                admin_signer,
                current_round.round_address
            );
        };
    }

    #[view]
    public fun get_round_address(tournament_address: address): address acquires CurrentRound {
        assert!(exists<CurrentRound>(tournament_address), EROUND_DOES_NOT_EXIST);
        let current_round = borrow_global<CurrentRound>(tournament_address);
        current_round.round_address
    }

    #[view]
    public fun get_current_game_module(tournament_address: address): String acquires CurrentRound {
        assert!(exists<CurrentRound>(tournament_address), EROUND_DOES_NOT_EXIST);
        let current_round = borrow_global<CurrentRound>(tournament_address);
        current_round.game_module
    }


    // -------------------------------------------------------------------------------------- //
    //                                    Assertion helpers                                   //
    // -------------------------------------------------------------------------------------- //

    inline fun assert_tournament_not_ended(tournament_address: address) {
        let has_ended = borrow_global<TournamentState>(tournament_address).has_ended;
        assert!(!has_ended, ETOURNAMENT_HAS_ENDED);
    }

    inline fun assert_tournament_is_joinable(tournament_address: address) {
        assert!(exists<TournamentState>(tournament_address), ETOURNAMENT_DOES_NOT_EXIST);
        let is_joinable = borrow_global<TournamentState>(tournament_address).is_joinable;
        assert!(is_joinable, ETOURNAMENT_NOT_JOINABLE);
        let is_not_full = get_num_player_joined(tournament_address) < get_max_players(tournament_address);
        assert!(is_not_full, ETOURNAMENT_FULL);
    }

    inline fun assert_is_admin(caller_address: address, tournament_addressess: address) {
        assert!(
            is_admin_address(caller_address, tournament_addressess),
            ENOT_AUTHORIZED
        );
    }

    // -------------------------------------------------------------------------------------- //
    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                     View functions                                     //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //
    // -------------------------------------------------------------------------------------- //


    #[view]
    /// Note that these take the tournament director object address as input, not the tournament creator.
    public fun get_max_players(tournament_address: address): u64 acquires TournamentDirector {
        borrow_global<TournamentDirector>(tournament_address).max_players
    }

    #[view]
    /// Note that these take the tournament director object address as input, not the tournament creator.
    public fun get_num_player_joined(tournament_address: address): u64 acquires TournamentDirector {
        borrow_global<TournamentDirector>(tournament_address).players_joined
    }

    #[view]
    /// Note that these take the tournament director object address as input, not the tournament creator.
    public fun get_max_num_winners(tournament_address: address): u64 acquires TournamentDirector {
        borrow_global<TournamentDirector>(tournament_address).max_num_winners
    }

    #[view]
    /// Note that these take the tournament director object address as input, not the tournament creator.
    public fun get_tournament_state(
        tournament_addr: address
    ): ViewTournamentState acquires TournamentDirector, TournamentState, CurrentRound {
        let tournament_director = borrow_global<TournamentDirector>(tournament_addr);
        let tournament_state = borrow_global<TournamentState>(tournament_addr);
        let current_round = borrow_global<CurrentRound>(tournament_addr);
        ViewTournamentState {
            max_num_winners: tournament_director.max_num_winners,
            is_joinable: tournament_state.is_joinable,
            has_ended: tournament_state.has_ended,
            tournament_name: tournament_director.tournament_name,
            current_round_game_module: current_round.game_module,
            round_number: current_round.number,
            round_address: current_round.round_address,
        }
    }
}
