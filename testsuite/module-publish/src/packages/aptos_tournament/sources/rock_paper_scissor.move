module tournament::rock_paper_scissor {
    use std::hash;
    use std::object::Object;
    use std::option::{Self, Option};
    use std::signer;
    use std::string::{String, utf8};
    use std::vector;
    use std::table::{Self, Table};
    use aptos_framework::object::Self;

    use aptos_token_objects::token::Token;

    use tournament::admin;
    use tournament::admin::get_tournament_owner_signer_from_object_owner;
    use tournament::room;
    use tournament::round;
    use tournament::token_manager;
    use tournament::tournament_manager;

    friend tournament::aptos_tournament;

    //// ERROR CODES
    /// Player is not in the game.
    const EPLAYER_UNKNOWN: u64 = 0;
    /// Player has not committed an action.
    const EACTION_NOT_COMMITTED: u64 = 1;
    /// Action does not match verified.
    const EACTION_DOES_NOT_MATCH: u64 = 2;
    /// The signer passed in must be an object.
    const ENOT_OBJECT: u64 = 3;
    /// Game is not completed
    const EGAME_NOT_COMPLETED: u64 = 4;
    /// The room must be a limited room
    const EROOM_NOT_LIMITED: u64 = 5;
    /// There must be two players in this room
    const EINVALID_PLAYER_COUNT: u64 = 6;

    struct Player has copy, drop, store {
        // hashed
        committed_action: Option<vector<u8>>,
        // plain action
        verified_action: Option<vector<u8>>,
        address: address,
        token_address: address,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct RockPaperScissorsGame has key {}

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct RockPaperScissor has key, store, drop {
        player1: Player,
        player2: Player,
    }

    struct GameState {
        game_state: String,
        player_state: String,
        opponent_state: String,
        player_action: Option<vector<u8>>,
        opponent_action: Option<vector<u8>>,
    }

    struct MyAddress has copy, drop, store {
        inner: address
    }

    public fun get_address(my_address: MyAddress): address {
        my_address.inner
    }

    public fun get_player_to_game_mapping(
        game_addresses: &vector<address>
    ): Table<address, MyAddress> acquires RockPaperScissor {
        let player_to_game_mapping = table::new();
        // let player_to_game_mapping = vector::empty<(address, address)>();
        vector::for_each_ref(game_addresses, |game_address| {
            let game = borrow_global<RockPaperScissor>(*game_address);
            // vector::push_back(&mut player_to_game_mapping, (game.player1.address, *game_address));
            // vector::push_back(&mut player_to_game_mapping, (game.player2.address, *game_address));
            table::upsert(&mut player_to_game_mapping, game.player2.address, MyAddress {
                inner: *game_address
            });
        });
        player_to_game_mapping
    }

    public fun update_player_to_game_mapping(
        game_addresses: &vector<address>,
        player_to_game_mapping: &mut Table<address, MyAddress>,
    ) acquires RockPaperScissor {
        // let player_to_game_mapping = vector::empty<(address, address)>();
        vector::for_each_ref(game_addresses, |game_address| {
            let game = borrow_global<RockPaperScissor>(*game_address);
            // vector::push_back(&mut player_to_game_mapping, (game.player1.address, *game_address));
            // vector::push_back(&mut player_to_game_mapping, (game.player2.address, *game_address));
            table::upsert(player_to_game_mapping, game.player2.address, MyAddress {
                inner: *game_address
            });
        });
    }

    public(friend) fun add_players_returning(
        tournament_address: address,
        players: vector<Object<Token>>
    ): vector<address> {
        let tournament_signer = admin::get_tournament_owner_signer(tournament_address);
        let round_address = tournament_manager::get_round_address(tournament_address);

        let room_signers = round::add_players<RockPaperScissorsGame>(&tournament_signer, round_address, players);
        assert!(option::is_some(&room_signers), EROOM_NOT_LIMITED);
        let room_signers = option::extract(&mut room_signers);
        vector::for_each_ref(&room_signers, |room_signer| {
            let room_address = signer::address_of(room_signer);
            let players = room::get_players<RockPaperScissorsGame>(room_address);
            let players = option::extract(&mut players);
            assert!(vector::length(&players) == 2, EINVALID_PLAYER_COUNT);

            let player_2 = vector::pop_back(&mut players);
            let player_1 = vector::pop_back(&mut players);
            let game = RockPaperScissor {
                player1: token_to_new_player(player_1),
                player2: token_to_new_player(player_2),
            };

            move_to<RockPaperScissor>(room_signer, game);
        });
        vector::map_ref(&room_signers, |room_signer|{ signer::address_of(room_signer) })
    }

    fun token_to_new_player(token: Object<Token>): Player {
        let token_address = object::object_address(&token);
        Player {
            committed_action: option::none(),
            verified_action: option::none(),
            address: object::owner(token),
            token_address,
        }
    }

    public entry fun commit_action(
        player: &signer,
        room_address: address,
        action_hash: vector<u8>
    ) acquires RockPaperScissor {
        let game = borrow_global_mut<RockPaperScissor>(room_address);
        let player_address = signer::address_of(player);

        let (player_index, _player_address) = room::assert_player_in_limited_room<RockPaperScissorsGame>(
            room_address,
            player_address
        );

        if (player_index == 0) {
            commit_player_action(&mut game.player1, action_hash);
        } else {
            commit_player_action(&mut game.player2, action_hash);
        };
    }

    public entry fun verify_action(
        player: &signer,
        game_address: address,
        action: vector<u8>,
        hash_addition: vector<u8>
    ) acquires RockPaperScissor {
        verify_action_returning(player, game_address, action, hash_addition);
    }

    // returns: (is_game_over: bool, winners: vector<address>, losers: vector<address>)
    public fun verify_action_returning(
        player: &signer,
        game_address: address,
        action: vector<u8>,
        hash_addition: vector<u8>
    ): (bool, vector<address>, vector<address>) acquires RockPaperScissor {
        let game = borrow_global_mut<RockPaperScissor>(game_address);
        let player_address = signer::address_of(player);

        let (player_index, _player_address) = room::assert_player_in_limited_room<RockPaperScissorsGame>(
            game_address,
            player_address
        );

        if (player_index == 0) {
            verify_player_action(&mut game.player1, action, hash_addition);
        } else {
            verify_player_action(&mut game.player2, action, hash_addition);
        };

        if (is_game_complete(game_address)) {
            let owner = get_tournament_owner_signer_from_object_owner(game_address);
            let (winners, losers) = force_close_game(&owner, game_address);
            return (true, winners, losers)
        };
        (false, vector[], vector[])
    }

    public entry fun handle_games_end(
        admin: &signer,
        game_addresses: vector<address>,
    ) acquires RockPaperScissor {
        handle_games_end_returning(admin, game_addresses);
    }

    public fun handle_games_end_returning(
        admin: &signer,
        game_addresses: vector<address>,
    ): vector<vector<vector<address>>> acquires RockPaperScissor {
        let admin = admin::get_admin_signer_as_admin(admin);
        vector::map(game_addresses, |game_address|{
            let winners_and_losers = vector[];
            if (exists<RockPaperScissor>(game_address)) {
                let (winners, losers) = force_close_game(&admin, game_address);
                vector::push_back(&mut winners_and_losers, winners);
                vector::push_back(&mut winners_and_losers, losers);
            };
            winners_and_losers
        })
    }

    // Returns (winners, losers): user addresess
    public(friend) inline fun force_close_game(
        owner: &signer,
        game_address: address
    ): (vector<address>, vector<address>) {
        let (winners, losers) = get_results_force(game_address);

        // Convert token addresses to user addresses
        let winners = vector::map<address, address>(winners, |winner| {
            let token_obj = object::address_to_object<Token>(winner);
            object::owner(token_obj)
        });
        let losers = vector::map<address, address>(losers, |loser| {
            let token_obj = object::address_to_object<Token>(loser);
            let loser_user = object::owner(token_obj);
            // Delete the tokens while we go
            token_manager::mark_token_loss(owner, loser);
            loser_user
        });

        // Clean up the object
        move_from<RockPaperScissor>(game_address);
        room::close_room<RockPaperScissorsGame>(owner, game_address);

        (winners, losers)
    }

    #[view]
    public fun get_game_address(creator_address: address, seed: vector<u8>): address {
        object::create_object_address(&creator_address, seed)
    }

    #[view]
    public fun is_game_committed(game_address: address): bool acquires RockPaperScissor {
        let game = borrow_global<RockPaperScissor>(game_address);
        option::is_some<vector<u8>>(&game.player1.committed_action)
            && option::is_some<vector<u8>>(&game.player2.committed_action)
    }

    #[view]
    public fun is_game_complete(game_address: address): bool acquires RockPaperScissor {
        let game = borrow_global<RockPaperScissor>(game_address);
        option::is_some<vector<u8>>(&game.player1.verified_action)
            && option::is_some<vector<u8>>(&game.player2.verified_action)
    }

    struct ViewGame has copy, drop, store {
        player1: Player,
        player2: Player,
    }

    #[view]
    public fun view_game(room_address: address): ViewGame acquires RockPaperScissor {
        let game = borrow_global<RockPaperScissor>(room_address);
        ViewGame {
            player1: game.player1,
            player2: game.player2,
        }
    }

    struct ViewRPSPlayerState has copy, drop, store {
        game_room: address,
        game_state: ViewGame,
    }

    #[view]
    public fun get_player_rps_state(
        room_address: address,
    ): Option<ViewRPSPlayerState> acquires RockPaperScissor {
        option::some(ViewRPSPlayerState {
            game_room: room_address,
            game_state: view_game(room_address),
        })
    }

    #[view]
    public fun game_status(player_address: address, game_address: address): GameState acquires RockPaperScissor {
        let game = borrow_global<RockPaperScissor>(game_address);
        assert!(
            player_address != game.player1.address || player_address != game.player2.address,
            EPLAYER_UNKNOWN,
        );
        let (player, opponent) = (&game.player1, &game.player2);
        if (game.player2.address == player_address) {
            (player, opponent) = (&game.player2, &game.player1)
        };

        let player_status = player_status(player);
        let opponent_status = player_status(opponent);
        let game_status = utf8(b"verified");

        if (player_status == utf8(b"started") || opponent_status == utf8(b"started")) {
            game_status = utf8(b"started");
        };

        if (player_status == utf8(b"committed") || opponent_status == utf8(b"committed")) {
            game_status = utf8(b"committed");
        };

        GameState {
            game_state: game_status,
            player_state: player_status,
            opponent_state: opponent_status,
            player_action: player.verified_action,
            opponent_action: opponent.verified_action,
        }
    }

    // View function for getting the results as the players instead of the tokens
    // TODO: Optimize this by moving win/lose code out of get_results
    #[view]
    public fun get_results_as_players(
        game_address: address,
    ): (vector<address>, vector<address>) acquires RockPaperScissor {
        let (winner_tokens, loser_tokens) = get_results(game_address);
        let winner_players = vector::map(winner_tokens, |token| {
            let token_obj = object::address_to_object<Token>(token);
            object::owner(token_obj)
        });
        let loser_players = vector::map(loser_tokens, |token| {
            let token_obj = object::address_to_object<Token>(token);
            object::owner(token_obj)
        });
        (winner_players, loser_players)
    }

    // Abort if game is not complete, otherwise (winners, losers)
    // NOTE: This returns the tokens, not the player addresses
    #[view]
    public fun get_results(
        game_address: address
    ): (vector<address>, vector<address>) acquires RockPaperScissor {
        assert!(is_game_complete(game_address), EGAME_NOT_COMPLETED);
        let game = borrow_global<RockPaperScissor>(game_address);

        let action1 = option::get_with_default(
            &game.player1.verified_action,
            b"invalid", // TODO: Can I do better?
        );

        let action2 = option::get_with_default(
            &game.player2.verified_action,
            b"invalid",
        );

        // TODO: check for valid moves, etc
        let winners = vector<address>[];
        let losers = vector<address>[];

        // TODO: Do better
        let player1 = game.player1.token_address;
        let player2 = game.player2.token_address;
        // tie means they both win
        if (action1 == action2) {
            vector::push_back(&mut winners, player1);
            vector::push_back(&mut winners, player2);
        };

        // TODO: Can I strongly type these in some way?
        let rock = b"Rock";
        let paper = b"Paper";
        let scissor = b"Scissor";

        // TODO: Remove after debugging
        // std::debug::print(&std::string::utf8(b"------------------------------------"));
        // let player_1_and_action = std::string_utils::to_string<address>(&player1);
        // // player_1_and_action = std::string::sub_string(&player_1_and_action, 1, 7);
        // std::string::append_utf8(&mut player_1_and_action, b": ");
        // std::string::append_utf8(&mut player_1_and_action, action1);
        // let player_2_and_action = std::string_utils::to_string<address>(&player2);
        // // player_2_and_action = std::string::sub_string(&player_2_and_action, 1, 7);
        // std::string::append_utf8(&mut player_2_and_action, b": ");
        // std::string::append_utf8(&mut player_2_and_action, action2);
        // std::debug::print(&player_1_and_action);
        // std::debug::print(&player_2_and_action);

        if (action1 == rock && action2 == paper) {
            vector::push_back(&mut winners, player2);
            vector::push_back(&mut losers, player1);
        };

        if (action1 == rock && action2 == scissor) {
            vector::push_back(&mut winners, player1);
            vector::push_back(&mut losers, player2);
        };

        if (action1 == paper && action2 == rock) {
            vector::push_back(&mut winners, player1);
            vector::push_back(&mut losers, player2);
        };

        if (action1 == paper && action2 == scissor) {
            vector::push_back(&mut winners, player2);
            vector::push_back(&mut losers, player1);
        };

        if (action1 == scissor && action2 == rock) {
            vector::push_back(&mut winners, player2);
            vector::push_back(&mut losers, player1);
        };

        if (action1 == scissor && action2 == paper) {
            vector::push_back(&mut winners, player1);
            vector::push_back(&mut losers, player2);
        };

        (winners, losers)
    }

    // Returns (winners, losers)' token addresses
    #[view]
    public fun get_results_force(
        game_address: address
    ): (vector<address>, vector<address>) acquires RockPaperScissor {
        let game = borrow_global<RockPaperScissor>(game_address);
        let (player1, player2) = (game.player1, game.player2);
        let player1_verified = option::is_some<vector<u8>>(&player1.verified_action);
        let player2_verified = option::is_some<vector<u8>>(&player2.verified_action);

        let player1 = game.player1.token_address;
        let player2 = game.player2.token_address;

        // TODO: if one committed and the other didn't then person should win
        let (winners, losers) = if (player1_verified && player2_verified) {
            // evaluate complete game
            get_results(game_address)
        } else if (player1_verified) {
            // player 1 wins if player 2 has not verified
            (vector<address> [player1], vector<address> [player2])
        } else if (player2_verified) {
            // player 2 wins if player 1 has not verified
            (vector<address> [player2], vector<address> [player1])
        } else {
            // both lose
            (vector<address> [], vector<address> [player1, player2])
        };
        (winners, losers)
    }

    inline fun commit_player_action(player: &mut Player, action_hash: vector<u8>) {
        player.committed_action = option::some(action_hash);
    }

    fun verify_player_action(player: &mut Player, action: vector<u8>, hash_addition: vector<u8>) {
        let combo = copy action;
        vector::append(&mut combo, hash_addition);

        // For now, we are going to assume if you verify, you want to auto commit + verify
        if (player.committed_action == option::none()) {
            commit_player_action(player, hash::sha3_256(combo));
            player.verified_action = option::some(action);
        } else {
            assert!(
                option::some(hash::sha3_256(combo)) == player.committed_action,
                EACTION_DOES_NOT_MATCH
            );
            player.verified_action = option::some(action);
        }
    }

    fun player_status(player: &Player): String {
        if (option::is_some<vector<u8>>(&player.verified_action)) {
            return utf8(b"verified")
        } else if (option::is_some<vector<u8>>(&player.committed_action)) {
            return utf8(b"committed")
        };

        utf8(b"started")
    }
}
