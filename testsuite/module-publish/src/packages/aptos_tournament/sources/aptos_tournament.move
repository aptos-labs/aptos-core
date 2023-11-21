module tournament::aptos_tournament {
    use std::string::Self;
    use aptos_std::type_info::type_name;
    use aptos_framework::object::Object;

    use aptos_token_objects::token::Token;

    use tournament::admin;
    use tournament::admin::assert_admin;
    use tournament::rock_paper_scissor::{Self, RockPaperScissorsGame};
    use tournament::round;
    use tournament::tournament_manager;
    use tournament::trivia::{Self, TriviaGame};

    /// You are not authorized to do that
    const ENOT_AUTHORIZED: u64 = 0;
    /// Unrecognized game name
    const EUNRECOGNIZED_GAME: u64 = 1;

    fun init_module(deployer: &signer) {
        admin::setup_admin_signer(deployer);
    }

    public fun init_module_for_test(deployer: &signer) {
        init_module(deployer);
    }

    public entry fun create_new_tournament(caller: &signer) {
        create_new_tournament_returning(caller);
    }

    public fun create_new_tournament_returning(caller: &signer): address {
        let admin_signer = admin::get_admin_signer_as_admin(caller);

        let (_, tournament_address) = tournament_manager::initialize_tournament_with_return(
            &admin_signer,
            string::utf8(b"Aptos Tournament"),
            1_000,
            10,
        );

        tournament_manager::set_tournament_joinable(
            &admin_signer,
            tournament_address,
        );

        tournament_address
    }

    public entry fun add_players_to_game(
        caller: &signer,
        tournament_address: address,
        players: vector<Object<Token>>
    ) {
        add_players_to_game_returning(caller, tournament_address, players);
    }

    public fun add_players_to_game_returning(
        caller: &signer,
        tournament_address: address,
        players: vector<Object<Token>>
    ): vector<address> {
        assert_admin(caller);

        let game_module = tournament_manager::get_current_game_module(tournament_address);

        if (game_module == type_name<RockPaperScissorsGame>()) {
            rock_paper_scissor::add_players_returning(
                tournament_address,
                players,
            )
        }
        else if (game_module == type_name<TriviaGame>()) {
            trivia::add_players_returning(
                tournament_address,
                players,
            )
        }
        else {
            abort EUNRECOGNIZED_GAME
        }
    }

    public entry fun start_new_round<GameType>(caller: &signer, tournament_address: address) {
        let admin_signer = admin::get_admin_signer_as_admin(caller);

        let round_address = tournament_manager::get_round_address(tournament_address);
        if (round_address != @0x0) {
            let game_module = tournament_manager::get_current_game_module(tournament_address);
            if (game_module == type_name<RockPaperScissorsGame>()) {
                round::destroy_and_cleanup_round<RockPaperScissorsGame>(&admin_signer, round_address);
            }
            else if (game_module == type_name<TriviaGame>()) {
                trivia::destroy_and_cleanup_round(&admin_signer, round_address);
            }
            else {
                abort EUNRECOGNIZED_GAME
            }
        };
        let game_module = type_name<GameType>();
        if (game_module == type_name<RockPaperScissorsGame>()) {
            tournament_manager::start_new_round<RockPaperScissorsGame>(&admin_signer, tournament_address, 2, 2);
        }
        else if (game_module == type_name<TriviaGame>()) {
            tournament_manager::start_new_round<TriviaGame>(&admin_signer, tournament_address, 0, 0);
        }
        else {
            abort EUNRECOGNIZED_GAME
        }
    }
}
