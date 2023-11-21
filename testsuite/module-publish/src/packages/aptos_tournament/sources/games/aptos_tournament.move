module tournament::aptos_tournament {
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_std::type_info::type_name;
    use aptos_framework::object;
    use aptos_framework::object::Object;

    use tournament::admin;
    use tournament::admin::assert_admin;
    use tournament::rewards;
    use tournament::rock_paper_scissors::{Self, RockPaperScissorsGame};
    use tournament::roulette::{Self, RouletteGame};
    use tournament::round;
    use tournament::token_manager::TournamentPlayerToken;
    use tournament::tournament_manager;
    use tournament::trivia::{Self, TriviaGame};

    /// You are not authorized to do that
    const ENOT_AUTHORIZED: u64 = 0;
    /// Unrecognized game name
    const EUNRECOGNIZED_GAME: u64 = 1;

    fun init_module(deployer: &signer) {
        admin::setup_admin_signer(deployer);
    }

    #[test_only]
    public fun init_module_for_test(deployer: &signer) {
        init_module(deployer);
    }

    public entry fun create_new_tournament(caller: &signer) {
        create_new_tournament_returning(caller);
    }

    public entry fun create_new_tournament_with_config(
        caller: &signer,
        name: String,
        max_players: u64,
        max_winners: u64
    ) {
        create_new_tournament_returning_with_config(caller, name, max_players, max_winners);
    }

    public entry fun set_tournament_joinable(caller: &signer, tournament_address: address) {
        let admin_signer = admin::get_admin_signer_as_admin(caller);

        tournament_manager::set_tournament_joinable(
            &admin_signer,
            tournament_address,
        );
    }

    public fun create_new_tournament_returning(caller: &signer): address {
        create_new_tournament_returning_with_config(caller, string::utf8(b"Aptos Tournament"), 1_000, 1)
    }

    public fun create_new_tournament_returning_with_config(
        caller: &signer,
        name: String,
        max_players: u64,
        max_winners: u64
    ): address {
        let admin_signer = admin::get_admin_signer_as_admin(caller);

        let (_, tournament_address) = tournament_manager::initialize_tournament_with_return(
            &admin_signer,
            name,
            max_players,
            max_winners,
        );
        tournament_address
    }

    public entry fun end_matchmaking(caller: &signer, tournament_address: address) {
        end_matchmaking_returning(caller, tournament_address);
    }

    public fun end_matchmaking_returning(
        caller: &signer,
        tournament_address: address
    ): (Option<vector<signer>>, vector<Object<TournamentPlayerToken>>) {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);

        let game_module = tournament_manager::get_current_game_module(tournament_address);

        let round_address = tournament_manager::get_round_address(tournament_address);

        let (room_signers, matched_players) = if (game_module == type_name<RockPaperScissorsGame>()) {
            let (room_signers, matched_players) = round::end_matchmaking_returning<RockPaperScissorsGame>(
                &tournament_signer,
                round_address
            );
            rock_paper_scissors::add_players_to_rooms_returning(option::borrow(&mut room_signers));
            (room_signers, matched_players)
        }
        else if (game_module == type_name<TriviaGame>()) {
            round::end_matchmaking_returning<TriviaGame>(&tournament_signer, round_address)
        }
        else if (game_module == type_name<RouletteGame>()) {
            let (room_signers, matched_players) = round::end_matchmaking_returning<RouletteGame>(
                &tournament_signer,
                round_address
            );
            roulette::add_players_to_rooms_returning(option::borrow(&mut room_signers));
            (room_signers, matched_players)
        }
        else {
            abort EUNRECOGNIZED_GAME
        };

        tournament_manager::set_tournament_not_joinable(
            &tournament_signer,
            tournament_address,
        );

        (room_signers, matched_players)
    }

    public entry fun end_tournament(
        caller: &signer,
        tournament_address: address,
    ) {
        let admin_signer = admin::get_admin_signer_as_admin(caller);
        tournament_manager::end_tournament(&admin_signer, tournament_address);
    }

    #[deprecated]
    /// Do not use this! Use separate initialize_and_fund_coin_reward_pool or initialize_and_fund_token_pool
    public entry fun initialize_reward_pool<CoinType>(
        _caller: &signer,
        _tournament_address: address,
        _coin_reward_amount: u64,
    ) {
        abort 0
    }

    public entry fun initialize_and_fund_coin_reward_pool<CoinType>(
        caller: &signer,
        tournament_address: address,
        coin_reward_amount: u64,
        amount_to_fund: u64,
    ) {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);
        rewards::intitialize_coin_reward_pool<CoinType>(&tournament_signer, coin_reward_amount);
        rewards::deposit_coin_rewards<CoinType>(caller, tournament_address, amount_to_fund);
    }

    public entry fun initialize_and_fund_token_pool(
        caller: &signer,
        tournament_address: address,
        // The address of the creator, eg: 0xcafe
        creators: vector<address>,
        // The names of collections; this is unique under the same account, eg: "Aptos Animal Collection"
        collections: vector<String>,
        // The names of the tokens; this is the same as the name field of TokenData
        token_names: vector<String>,
        // The property versions of the tokens
        property_versions: vector<u64>,
    ) {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);
        rewards::intitialize_token_v1_reward_pool(&tournament_signer);
        rewards::deposit_token_v1_rewards(
            caller,
            tournament_address,
            creators,
            collections,
            token_names,
            property_versions
        );
    }

    public entry fun withdraw_rewards<CoinType>(
        caller: &signer,
        tournament_address: address,
        target_address: address,
    ) {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);
        let tournament_address = signer::address_of(&tournament_signer);

        if (rewards::reward_pool_exists<CoinType>(tournament_address)) {
            rewards::withdraw_coin_rewards<CoinType>(&tournament_signer, target_address);
        };
        if (rewards::token_v1_reward_pool_exists(tournament_address)) {
            rewards::withdraw_token_v1_rewards(&tournament_signer, target_address);
        }
    }

    // There is an issue with the way we do `vector<Object<TournamentPlayerToken>>` deserialization such that only
    // 10 items end up being allowed; this is a workaround for that
    public entry fun add_players_to_game_by_address(
        caller: &signer,
        tournament_address: address,
        player_addresses: vector<address>
    ) {
        add_players_to_game_by_address_returning(caller, tournament_address, player_addresses);
    }

    public fun add_players_to_game_by_address_returning(
        caller: &signer,
        tournament_address: address,
        player_addresses: vector<address>,
    ): vector<address> {
        let player_tokens = vector::map(
            player_addresses,
            |player_address| {
                object::address_to_object<TournamentPlayerToken>(player_address)
            }
        );
        add_players_to_game_returning(caller, tournament_address, player_tokens)
    }

    public entry fun add_players_to_game(
        caller: &signer,
        tournament_address: address,
        players: vector<Object<TournamentPlayerToken>>
    ) {
        add_players_to_game_returning(caller, tournament_address, players);
    }

    public fun add_players_to_game_returning(
        caller: &signer,
        tournament_address: address,
        players: vector<Object<TournamentPlayerToken>>
    ): vector<address> {
        assert_admin(caller);

        let game_module = tournament_manager::get_current_game_module(tournament_address);

        if (game_module == type_name<RockPaperScissorsGame>()) {
            rock_paper_scissors::add_players_returning(
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
        else if (game_module == type_name<RouletteGame>()) {
            roulette::add_players_returning(
                tournament_address,
                players,
            )
        }
        else {
            abort EUNRECOGNIZED_GAME
        }
    }

    public entry fun end_current_round(caller: &signer, tournament_address: address) {
        let owner_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);

        let round_address = tournament_manager::get_round_address(tournament_address);
        if (round_address != @0x0) {
            let game_module = tournament_manager::get_current_game_module(tournament_address);
            if (game_module == type_name<RockPaperScissorsGame>()) {
                round::end_play<RockPaperScissorsGame>(&owner_signer, round_address);
            }
            else if (game_module == type_name<TriviaGame>()) {
                round::end_play<TriviaGame>(&owner_signer, round_address);
            }
            else if (game_module == type_name<RouletteGame>()) {
                round::end_play<RouletteGame>(&owner_signer, round_address);
            }
            else {
                abort EUNRECOGNIZED_GAME
            }
        }
    }

    public entry fun cleanup_current_round(caller: &signer, tournament_address: address) {
        let round_address = tournament_manager::get_round_address(tournament_address);
        if (round_address != @0x0) {
            let game_module = tournament_manager::get_current_game_module(tournament_address);
            if (game_module == type_name<RockPaperScissorsGame>()) {
                let owner_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);
                round::destroy_and_cleanup_round<RockPaperScissorsGame>(&owner_signer, round_address);
            }
            else if (game_module == type_name<TriviaGame>()) {
                trivia::destroy_and_cleanup_round(caller, round_address);
            }
            else if (game_module == type_name<RouletteGame>()) {
                let owner_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);
                round::destroy_and_cleanup_round<RouletteGame>(&owner_signer, round_address);
            }
            else {
                abort EUNRECOGNIZED_GAME
            }
        }
    }

    public entry fun start_new_round<GameType>(caller: &signer, tournament_address: address) {
        let admin_signer = admin::get_admin_signer_as_admin(caller);

        cleanup_current_round(caller, tournament_address);

        let game_module = type_name<GameType>();
        if (game_module == type_name<RockPaperScissorsGame>()) {
            let round_address = tournament_manager::start_new_round<RockPaperScissorsGame>(
                &admin_signer,
                tournament_address,
                2,
                2
            );
            round::start_play<RockPaperScissorsGame>(&admin_signer, round_address);
        }
        else if (game_module == type_name<TriviaGame>()) {
            let round_address = tournament_manager::start_new_round<TriviaGame>(
                &admin_signer,
                tournament_address,
                0,
                0
            );
            round::start_play<TriviaGame>(&admin_signer, round_address);
        }
        else if (game_module == type_name<RouletteGame>()) {
            let round_address = tournament_manager::start_new_round<RouletteGame>(
                &admin_signer,
                tournament_address,
                2,
                4
            );
            round::start_play<RouletteGame>(&admin_signer, round_address);
        }
        else {
            abort EUNRECOGNIZED_GAME
        }
    }
}
