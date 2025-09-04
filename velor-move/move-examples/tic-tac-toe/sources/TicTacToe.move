module tic_tac_toe::ttt {
    use velor_framework::account;
    use velor_framework::event;
    use std::error;
    use std::option::{Self, Option};
    use std::signer;
    use std::vector;

    //// GAME CONSTANTS
    const GRID_SIZE: u64 = 9;
    const WIDTH_AND_HEIGHT: u64 = 3;
    const EMPTY_CELL: u64 = 3;

    //// PLAYER CONSTANTS
    const PLAYER_X_TYPE: u64 = 1;
    const PLAYER_O_TYPE: u64 = 2;

    //// ERROR CODES
    /// Placing a move on a (x,y) position that is already occupied
    const EINVALID_MOVE: u64 = 0;
    /// An address trying to claim a player that is already taken
    const EPLAYER_TAKEN: u64 = 1;
    /// Trying to destroy game before game has been finished
    const EGAME_NOT_DONE: u64 = 2;
    /// An address can only join as one player, not two
    const ECANNOT_JOIN_AS_TWO_PLAYERS: u64 = 3;
    /// A (x,y) move is out of bounds of the 3x3 grid
    const EOUT_OF_BOUNDS_MOVE: u64 = 4;
    /// Address doesn't exist in the current Game
    const EPLAYER_NOT_IN_GAME: u64 = 5;
    /// An attempt to place moves even though a Game is over
    const EGAME_HAS_ALREADY_FINISHED: u64 = 6;
    /// User tries to make two games
    const EGAME_ALREADY_EXISTS_FOR_USER: u64 = 7;
    /// Game doesn't exist under address
    const EGAME_DOESNT_EXIST: u64 = 8;
    /// Out of turn move or player not in game
    const EOUT_OF_TURN_MOVE: u64 = 9;


    #[event]
    struct GameOverEvent has drop, store {
        game_address: address,
        is_game_over: bool,
    }

    struct Player has copy, drop, store {
        type: u64,
        owner: address,
    }

    struct Board has drop, store {
        vec: vector<u64>,
        row: u64,
        col: u64,
    }

    //// to access game records after games are over.
    struct Game has key, store {
        board: Board,
        player_x: Option<Player>,
        player_o: Option<Player>,
        is_player_x_turn: bool,
        is_game_over: bool,
    }

    /*
     * @notice initializes a valid, playable Game
     * @dev stores the Game into global storage
     */
    //// TODO: have Game as its own object, with its own address
    public entry fun start_game(creator: &signer) {
        // check game doesn't already exist under creator address
        assert!(!exists<Game>(signer::address_of(creator)), error::already_exists(EGAME_ALREADY_EXISTS_FOR_USER));
        let game = initalize_game(creator);
        let creator_addr = signer::address_of(creator);
        choose_player_x(&mut game, creator_addr);
        move_to<Game>(creator, game);
    }

    /*
     * @notice lets another user join given a valid game address
     */
    public entry fun join_as_player_o(new_user: &signer, game_addr: address) acquires Game {
        let new_user_addr = signer::address_of(new_user);
        assert!(new_user_addr != game_addr, error::invalid_argument(ECANNOT_JOIN_AS_TWO_PLAYERS));

        assert!(exists<Game>(game_addr), error::not_found(EGAME_DOESNT_EXIST));
        let game = borrow_global_mut(game_addr);
        choose_player_o(game, new_user_addr);
    }

    /*
     * @notice places a move at a given (x,y) coordinate on a 3x3 board
     * @dev checks to ensure a player can make a valid move
     */
    public entry fun choose_move(player: &signer, game_addr: address, x: u64, y: u64) acquires Game {
        assert!(x < 3, error::out_of_range(EOUT_OF_BOUNDS_MOVE));
        assert!(y < 3, error::out_of_range(EOUT_OF_BOUNDS_MOVE));
        let game: &mut Game = borrow_global_mut(game_addr);
        let player_x = option::borrow_mut(&mut game.player_x);
        let player_o = option::borrow_mut(&mut game.player_o);

        let player_addr = signer::address_of(player);
        assert!(
            player_addr != player_x.owner || player_addr != player_o.owner,
            error::permission_denied(EPLAYER_NOT_IN_GAME),
        );

        if (player_addr == player_x.owner) {
            place_move(game, x, y, *player_x);
        } else {
            place_move(game, x, y, *player_o);
        };
    }

    /*
     * @notice destroy Game at the end of session / forfeit
     */
    public entry fun cleanup(creator: &signer) acquires Game {
        let creator_addr: address = signer::address_of(creator);
        // abort if no such game exists under creator
        let game: Game = move_from<Game>(creator_addr);
        cleanup_game(game);
    }

    /*
     * @notice voluntarily give up, the other player wins
     */
    public entry fun forfeit(player: &signer, game_addr: address) acquires Game {
        let player_addr = signer::address_of(player);
        let game: &mut Game = borrow_global_mut(game_addr);
        let player_x = option::borrow_mut(&mut game.player_x);
        let player_o = option::borrow_mut(&mut game.player_o);

        assert!(
            player_addr != player_x.owner || player_addr != player_o.owner,
            error::permission_denied(EPLAYER_NOT_IN_GAME)
        );

        game.is_game_over = true;

        event::emit(GameOverEvent { game_address: game_addr, is_game_over: true, });
    }

    /*
     * @notice initialize Game struct with base values for a 3x3 game
     */
    fun initalize_game(creator: &signer): Game {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < GRID_SIZE) {
            vector::push_back(&mut v, EMPTY_CELL);
            i = i + 1;
        };

        Game {
            board: Board {
                vec: v,
                row: WIDTH_AND_HEIGHT,
                col: WIDTH_AND_HEIGHT,
            },
            player_x: option::none(),
            player_o: option::none(),
            is_player_x_turn: true,
            is_game_over: false,
        }
    }

    /*
     * @notice user who initiates game is automatically player_x
     */
    fun choose_player_x(game: &mut Game, user: address) {
        assert!(!game.is_game_over, error::invalid_argument(EGAME_HAS_ALREADY_FINISHED));
        assert!(option::is_none(&game.player_x), error::already_exists(EPLAYER_TAKEN));

        game.player_x = option::some(Player {
            type: PLAYER_X_TYPE,
            owner: user,
        });
    }

    /*
     * @notice another user whose not the creator may join as player_o
     */
    fun choose_player_o(game: &mut Game, user: address) {
        assert!(!game.is_game_over, error::invalid_argument(EGAME_HAS_ALREADY_FINISHED));
        assert!(option::is_none(&game.player_o), error::already_exists(EPLAYER_TAKEN));

        game.player_o = option::some(Player {
            type: PLAYER_O_TYPE,
            owner: user,
        });
    }

    /*
     * @notice place (x,y) move on a 3x3 board
     */
    fun place_move(game: &mut Game, x: u64, y: u64, player: Player) {
        // validate game state
        assert!(!game.is_game_over, error::invalid_argument(EGAME_HAS_ALREADY_FINISHED));

        // validate player move
        let player_type = player.type;
        if (game.is_player_x_turn) {
            assert!(player_type == PLAYER_X_TYPE, error::unauthenticated(EOUT_OF_TURN_MOVE));
        } else {
            assert!(player_type == PLAYER_O_TYPE, error::unauthenticated(EOUT_OF_TURN_MOVE));
        };

        let position = WIDTH_AND_HEIGHT * x + y;
        let cell = vector::borrow_mut(&mut game.board.vec, position);

        // validate cell is empty
        assert!(*cell == EMPTY_CELL, error::invalid_state(EINVALID_MOVE));
        *cell = player_type;

        // update turn after placing move
        if (game.is_player_x_turn) {
            game.is_player_x_turn = false;
        } else {
            game.is_player_x_turn = true;
        };

        // check if game won
        let is_game_over = check_player_win(game);
        if (is_game_over) game.is_game_over = true;
    }

    /*
     * @notice exhaustive search to validate all winning game states
     */
    fun check_player_win(game: &mut Game): bool {
        // check rows
        let row = 0;
        while (row < WIDTH_AND_HEIGHT) {
            let r0 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * row + 0);
            let r1 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * row + 1);
            let r2 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * row + 2);
            if ((r0 == r1 && r1 == r2 && *r2 == PLAYER_X_TYPE) ||
                (r0 == r1 && r1 == r2 && *r2 == PLAYER_O_TYPE)
            ) {
                return true
            };

            row = row + 1;
        };

        // check cols
        let col = 0;
        while (col < WIDTH_AND_HEIGHT) {
            let c0 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 0 + col);
            let c1 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 1 + col);
            let c2 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 2 + col);
            if ((c0 == c1 && c1 == c2 && *c2 == PLAYER_X_TYPE) ||
                (c0 == c1 && c1 == c2 && *c2 == PLAYER_O_TYPE)
            ) {
                return true
            };

            col = col + 1;
        };

        // check diagonals
        let e00 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 0 + 0);
        let e11 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 1 + 1);
        let e22 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 2 + 2);
        if ((e00 == e11 && e11 == e22 && *e22 == PLAYER_X_TYPE) ||
            (e00 == e11 && e11 == e22 && *e22 == PLAYER_O_TYPE)
        ) {
            return true
        };

        let e02 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 0 + 2);
        let e20 = vector::borrow(&game.board.vec, WIDTH_AND_HEIGHT * 2 + 0);
        if ((e02 == e11 && e11 == e20 && *e20 == PLAYER_X_TYPE) ||
            (e02 == e11 && e11 == e20 && *e20 == PLAYER_O_TYPE)
        ) {
            return true
        };

        false
    }

    /*
     * @notice check status of game
     */
    fun check_is_game_over(game: &Game): bool {
        game.is_game_over
    }

    /*
     * @notice helper function to destroy Game at the end of session / forfeit
     */
    fun cleanup_game(game: Game) {
        let Game {
            board: Board {
                vec,
                row: _,
                col: _,
            },
            player_x,
            player_o,
            is_player_x_turn: _,
            is_game_over: _,
        } = game;
        option::destroy_some(player_x);
        option::destroy_some(player_o);
        while (!vector::is_empty(&vec)) {
            vector::pop_back(&mut vec);
        };
    }


    //// Tests
    #[test_only]
    use velor_framework::account::create_account_for_test;

    #[test(creator = @0x123)]
    fun test_start_game(creator: &signer) {
        create_account_for_test(signer::address_of(creator));
        start_game(creator);
    }

    #[test(creator = @0x123, new_user = @0x223, game_addr = @0x123)]
    fun test_join_as_player_o(creator: &signer, new_user: &signer, game_addr: address) acquires Game {
        create_account_for_test(signer::address_of(creator));
        start_game(creator);
        join_as_player_o(new_user, game_addr);
    }

    #[test(creator = @0x123, new_user = @0x223, third_user = @0x323, game_addr = @0x123)]
    #[expected_failure]
    fun test_invalid_join_as_player_o(
        creator: &signer,
        new_user: &signer,
        third_user: &signer,
        game_addr: address,
    ) acquires Game {
        create_account_for_test(signer::address_of(creator));
        start_game(creator);
        join_as_player_o(new_user, game_addr);
        join_as_player_o(third_user, game_addr);
    }

    #[test(player_x = @0x123, player_o = @0x223, game_addr = @0x123)]
    fun test_valid_game(
        player_x: &signer,
        player_o: &signer,
        game_addr: address,
    ) acquires Game {
        create_account_for_test(signer::address_of(player_x));
        start_game(player_x);
        join_as_player_o(player_o, game_addr);
        choose_move(player_x, game_addr, 0, 0);
        choose_move(player_o, game_addr, 2, 0);
        choose_move(player_x, game_addr, 0, 1);
        choose_move(player_o, game_addr, 2, 1);
        choose_move(player_x, game_addr, 0, 2);

        let game = borrow_global(game_addr);
        assert!(check_is_game_over(game), error::invalid_state(EGAME_NOT_DONE));
        cleanup(player_x);
    }

    #[test(player_x = @0x123, player_o = @0x223, game_addr = @0x123)]
    #[expected_failure]
    fun test_out_of_bound_move(
        player_x: &signer,
        player_o: &signer,
        game_addr: address,
    ) acquires Game {
        create_account_for_test(signer::address_of(player_x));
        start_game(player_x);
        join_as_player_o(player_o, game_addr);
        choose_move(player_x, game_addr, 3, 4);

        let game = borrow_global(game_addr);
        assert!(check_is_game_over(game), error::invalid_state(EGAME_NOT_DONE));
        cleanup(player_x);
    }

    #[test(player_x = @0x123, player_o = @0x223, game_addr = @0x123)]
    fun test_forfeit(
        player_x: &signer,
        player_o: &signer,
        game_addr: address,
    ) acquires Game {
        create_account_for_test(signer::address_of(player_x));
        start_game(player_x);
        join_as_player_o(player_o, game_addr);
        choose_move(player_x, game_addr, 0, 0);

        forfeit(player_x, game_addr);
        let game = borrow_global(game_addr);
        assert!(check_is_game_over(game), error::invalid_state(EGAME_NOT_DONE));
        cleanup(player_x);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_row_win_1(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 2, 0, player_o);
        place_move(&mut game, 0, 1, player_x);
        place_move(&mut game, 2, 1, player_o);
        place_move(&mut game, 0, 2, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_row_win_2(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 1, 0, player_x);
        place_move(&mut game, 2, 0, player_o);
        place_move(&mut game, 1, 1, player_x);
        place_move(&mut game, 2, 1, player_o);
        place_move(&mut game, 1, 2, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_row_win_3(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 2, 0, player_x);
        place_move(&mut game, 1, 0, player_o);
        place_move(&mut game, 2, 1, player_x);
        place_move(&mut game, 1, 1, player_o);
        place_move(&mut game, 2, 2, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_col_win_1(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 0, 2, player_o);
        place_move(&mut game, 1, 0, player_x);
        place_move(&mut game, 1, 2, player_o);
        place_move(&mut game, 2, 0, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_col_win_2(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 1, 0, player_o);
        place_move(&mut game, 2, 0, player_x);
        place_move(&mut game, 1, 1, player_o);
        place_move(&mut game, 2, 1, player_x);
        place_move(&mut game, 1, 2, player_o);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_col_win_3(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 2, 0, player_o);
        place_move(&mut game, 1, 0, player_x);
        place_move(&mut game, 2, 1, player_o);
        place_move(&mut game, 0, 1, player_x);
        place_move(&mut game, 2, 2, player_o);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_diagonal_win_1(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 2, 0, player_o);
        place_move(&mut game, 1, 1, player_x);
        place_move(&mut game, 2, 1, player_o);
        place_move(&mut game, 2, 2, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    fun test_valid_diagonal_win_2(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 2, player_x);
        place_move(&mut game, 1, 0, player_o);
        place_move(&mut game, 1, 1, player_x);
        place_move(&mut game, 2, 1, player_o);
        place_move(&mut game, 2, 0, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123)]
    #[expected_failure]
    fun test_invalid_player_x(user1_signer: &signer, user1: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_x(&mut game, user1);
        cleanup_game(game);
    }

    #[test(user2_signer = @0x223, user2 = @0x223)]
    #[expected_failure]
    fun test_invalid_player_o(user2_signer: &signer, user2: address) {
        create_account_for_test(signer::address_of(user2_signer));
        let game = initalize_game(user2_signer);
        choose_player_o(&mut game, user2);
        choose_player_o(&mut game, user2);
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    #[expected_failure]
    fun test_out_of_turn(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 0, 1, player_x);
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    #[expected_failure]
    fun test_place_move_after_game_win(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 2, 0, player_o);
        place_move(&mut game, 0, 1, player_x);
        place_move(&mut game, 2, 1, player_o);
        place_move(&mut game, 0, 2, player_x);
        assert!(check_is_game_over(&game), error::invalid_state(EGAME_NOT_DONE));

        place_move(&mut game, 1, 1, player_x);
        cleanup_game(game);
    }

    #[test(user1_signer = @0x123, user1 = @0x123, user2 = @0x223)]
    #[expected_failure]
    fun test_place_invalid_move(user1_signer: &signer, user1: address, user2: address) {
        create_account_for_test(signer::address_of(user1_signer));
        let game = initalize_game(user1_signer);
        choose_player_x(&mut game, user1);
        choose_player_o(&mut game, user2);

        let player_x = option::get_with_default(
            &game.player_x,
            Player { type: PLAYER_X_TYPE, owner: user1 }
        );
        let player_o = option::get_with_default(
            &game.player_o,
            Player { type: PLAYER_O_TYPE, owner: user2 }
        );

        place_move(&mut game, 0, 0, player_x);
        place_move(&mut game, 0, 0, player_o);
        cleanup_game(game);
    }
}
