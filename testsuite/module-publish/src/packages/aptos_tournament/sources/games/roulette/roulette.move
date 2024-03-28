module tournament::roulette {
    use std::bcs;
    use std::option;
    use std::signer;
    use std::vector;
    use aptos_std::from_bcs;
    use aptos_framework::object::{Self, Object};
    use aptos_framework::timestamp;
    use aptos_framework::transaction_context;

    use tournament::admin;
    use tournament::room;
    use tournament::round;
    use tournament::token_manager::{Self, TournamentPlayerToken, has_player_token};
    use tournament::tournament_manager;
    use tournament::tournament_manager::get_current_round_number;

    friend tournament::aptos_tournament;

    #[test_only] friend tournament::roulette_unit_tests;
    #[test_only] friend tournament::main_unit_test;

    /// You are not the owner of the object.
    const E_NOT_OWNER: u64 = 0;
    /// There is no such game
    const E_NOT_A_GAME: u64 = 1;
    /// The object passed in does not have a Roulette Player resource. It is not playing this game.
    const E_NOT_A_ROULETTE_PLAYER: u64 = 2;
    /// The player index is out of range.
    const EINVALID_PLAYER_INDEX: u64 = 3;
    /// The player does not exist.
    const EINVALID_PLAYER_DOES_NOT_EXIST: u64 = 4;
    /// The room must be a limited room
    const EROOM_NOT_LIMITED: u64 = 5;
    /// Invalid index
    const EINVALID_INDEX: u64 = 6;


    struct RouletteGame has key {}

    struct Player has key, copy, drop, store {
        address: address,
        token_address: address,
        index: u64,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct Roulette has key, store, drop, copy {
        players: vector<Player>,
        revealed_index: u64,
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                 Core game functionality                                //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    fun get_current_round_signer(tournament_signer: &signer, tournament_address: address): (address, signer) {
        let round_address = tournament_manager::get_round_address(tournament_address);
        let round_signer = round::get_round_signer<RouletteGame>(tournament_signer, round_address);
        (round_address, round_signer)
    }

    public(friend) fun add_players_returning(
        tournament_address: address,
        players: vector<Object<TournamentPlayerToken>>
    ): vector<address> {
        let tournament_signer = admin::get_tournament_owner_signer(tournament_address);
        let round_address = tournament_manager::get_round_address(tournament_address);

        let room_signers = round::add_players<RouletteGame>(&tournament_signer, round_address, players);
        assert!(option::is_some(&room_signers), EROOM_NOT_LIMITED);
        let room_signers = option::extract(&mut room_signers);
        add_players_to_rooms_returning(&room_signers)
    }

    public(friend) fun add_players_to_rooms_returning(room_signers: &vector<signer>): vector<address> {
        vector::for_each_ref(room_signers, |room_signer| {
            let room_address = signer::address_of(room_signer);
            let players = room::get_players<RouletteGame>(room_address);
            let players = option::extract(&mut players);

            let token_players = vector::map(players, |player| { token_to_new_player(player) });
            let game = Roulette {
                players: token_players,
                revealed_index: 255,
            };
            move_to<Roulette>(room_signer, game);
        });
        tournament::misc_utils::signers_to_addresses(room_signers)
    }

    fun token_to_new_player(token: Object<TournamentPlayerToken>): Player {
        let token_address = object::object_address(&token);
        Player {
            address: object::owner(token),
            index: 255,
            token_address,
        }
    }

    public entry fun commit_index(
        player: &signer,
        tournament_address: address,
        room_address: address,
        index: u64
    ) acquires Roulette {
        commit_index_returning(player, tournament_address, room_address, index);
    }

    /// Returns index of winner
    public fun commit_index_returning(
        player: &signer,
        tournament_address: address,
        room_address: address,
        index: u64
    ): option::Option<u64> acquires Roulette {
        assert!(index < 4, EINVALID_INDEX);

        std::debug::print(&aptos_std::string_utils::format1(&b"ROOM ADDRESS: {}", room_address));

        let game = borrow_global_mut<Roulette>(room_address);
        let player_address = signer::address_of(player);

        let (player_index, _player_address) = room::assert_player_in_limited_room<RouletteGame>(
            room_address,
            player_address
        );

        // commit_player_action
        vector::borrow_mut(&mut game.players, player_index).index = index;

        // This is failing because the admin signer is not authorized?
        if (is_ready(room_address)) {
            let owner = admin::get_admin_signer();
            option::some(soft_game_end(&owner, tournament_address, room_address))
        } else {
            (option::none())
        }
    }

    fun soft_game_end(
        admin: &signer,
        tournament_address: address,
        game_address: address,
    ): u64 acquires Roulette {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(admin, tournament_address);

        if (exists<Roulette>(game_address)) {
            let revealed_index = view_roulette(game_address).revealed_index;
            if (revealed_index == 255) {
                revealed_index = random_u64();
                reveal_answer(admin, game_address, revealed_index);
            };
            let game = view_roulette(game_address);
            let current_round = get_current_round_number(tournament_address);
            vector::for_each(game.players, |player| {
                handle_player_game_end(&tournament_signer, player, game.revealed_index, current_round);
            });
            return revealed_index
        };
        255
    }

    // deprecated for handle_games_end
    public entry fun handle_game_end(
        admin: &signer,
        tournament_address: address,
        game_address: address,
    ) acquires Roulette {
        handle_game_end_returning(admin, tournament_address, game_address);
    }

    // deprecated for handle_games_end_returning
    public fun handle_game_end_returning(
        admin: &signer,
        tournament_address: address,
        game_address: address,
    ): (option::Option<u64>) acquires Roulette {
        let idxs = handle_games_end_returning(admin, tournament_address, vector[game_address]);
        option::some(vector::pop_back(&mut idxs))
    }

    public entry fun handle_games_end(
        admin: &signer,
        tournament_address: address,
        game_addresses: vector<address>,
    ) acquires Roulette {
        handle_games_end_returning(admin, tournament_address, game_addresses);
    }

    public fun random_u64(): u64 {
        let to_hash = transaction_context::get_transaction_hash();
        vector::append(&mut to_hash, bcs::to_bytes(&timestamp::now_seconds()));
        let hash = std::hash::sha3_256(to_hash);

        let bytes: vector<u8> = vector[];
        let i = 0;
        while (i < 8) {
            vector::push_back(&mut bytes, vector::pop_back(&mut hash));
            i = i + 1;
        };
        from_bcs::to_u64(bytes) % 4
    }

    // return winner index
    public fun handle_games_end_returning(
        admin: &signer,
        tournament_address: address,
        game_addresses: vector<address>,
    ): vector<u64> acquires Roulette {
        let idxs = vector[];
        let admin = admin::get_admin_signer_as_admin(admin);

        vector::for_each(game_addresses, |game_address| {
            if (exists<Roulette>(game_address)) {
                let revealed_index = soft_game_end(&admin, tournament_address, game_address);
                vector::push_back(&mut idxs, revealed_index);

                move_from<Roulette>(game_address);
                room::close_room<RouletteGame>(&admin, game_address);
            }
        });
        idxs
    }

    inline fun handle_player_game_end(
        tournament_signer: &signer,
        player: Player,
        revealed_index: u64,
        current_round: u64
    ) {
        if (has_player_token(player.token_address)) {
            let player_hit = player.index == revealed_index;

            // Player dies if they have not picked an index
            if (player_hit || player.index == 255) {
                let player_token_addr = player.token_address;
                if (token_manager::has_player_token(player_token_addr)) {
                    token_manager::mark_token_loss(
                        tournament_signer,
                        player_token_addr,
                        current_round,
                    );
                }
            };
        }
    }

    public entry fun reveal_answer(
        _admin: &signer,
        game_address: address,
        revealed_index: u64,
    ) acquires Roulette {
        assert!(revealed_index < 4, EINVALID_INDEX);
        let roulette = borrow_global_mut<Roulette>(game_address);
        roulette.revealed_index = revealed_index;
    }

    public fun destroy_and_cleanup_round(admin_signer: &signer, round_address: address) acquires Roulette {
        let admin = admin::get_admin_signer_as_admin(admin_signer);
        if (exists<Roulette>(round_address)) {
            move_from<Roulette>(round_address);
        };
        round::destroy_and_cleanup_round<RouletteGame>(&admin, round_address);
    }

    public fun destroy_and_cleanup_current_round(
        admin_signer: &signer,
        tournament_address: address
    ) acquires Roulette {
        let round_address = tournament_manager::get_round_address(tournament_address);
        destroy_and_cleanup_round(admin_signer, round_address);
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                  Views and test helpers                                //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //


    #[view]
    /// Viewing the Object<Roulette> with its address as input
    public fun view_roulette(room_address: address): Roulette acquires Roulette {
        *borrow_global<Roulette>(room_address)
    }

    #[view]
    /// Checking to see if the address is an Object<Roulette>
    public fun is_roulette(round_address: address): bool {
        object::is_object(round_address) && exists<Roulette>(round_address)
    }

    #[view]
    /// Checking to see if all players commited
    public fun is_ready(room_address: address): bool acquires Roulette {
        let game = view_roulette(room_address);
        let ready = true;
        vector::for_each(game.players, |player| {
            if (!is_player_ready(player)) {
                ready = false;
            }
        });
        ready
    }

    inline fun is_player_ready(player: Player): bool {
        player.index != 255
    }

    #[view]
    /// Viewing the Object<Player> with its address as input
    public fun view_player(player_token_addr: address): Player acquires Player {
        assert!(exists<Player>(player_token_addr), EINVALID_PLAYER_DOES_NOT_EXIST);
        *borrow_global<Player>(player_token_addr)
    }
}
