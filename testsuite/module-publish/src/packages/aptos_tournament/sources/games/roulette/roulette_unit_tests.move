#[test_only]
module tournament::roulette_unit_tests {
    use std::account;
    use std::option;
    use std::signer;
    use std::string;
    use std::vector;
    use aptos_framework::object::{Self, Object};

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::roulette::{Self, RouletteGame};
    use tournament::test_utils;
    use tournament::token_manager;
    use tournament::token_manager::TournamentPlayerToken;
    use tournament::tournament_manager;

    fun setup_tournament(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        players: &vector<signer>,
    ): (address, vector<Object<TournamentPlayerToken>>) {
        test_utils::init_test_framework(aptos_framework, 1);
        test_utils::enable_features_for_test(aptos_framework);
        vector::for_each_ref(players, |player| {
            account::create_account_for_test(signer::address_of(player));
        });

        token_manager::init_module_for_test(deployer);
        aptos_tournament::init_module_for_test(deployer);
        admin::set_admin_signer(deployer, signer::address_of(admin));

        let tournament_address = aptos_tournament::create_new_tournament_returning(deployer);
        aptos_tournament::set_tournament_joinable(admin, tournament_address);
        // Ensure this works
        let _admin2 = admin::get_admin_signer_as_admin(admin);

        let tokens = vector::map_ref(players, |player| {
            tournament_manager::join_tournament_with_return(
                player,
                tournament_address,
                string::utf8(b"1")
            )
        });

        (tournament_address, tokens)
    }

    fun setup_new_round(
        admin: &signer,
        tournament_address: address,
        tokens: vector<Object<TournamentPlayerToken>>,
    ): (address, address, vector<address>, vector<Object<TournamentPlayerToken>>) {
        aptos_tournament::start_new_round<RouletteGame>(admin, tournament_address);

        let game_addresses = aptos_tournament::add_players_to_game_returning(
            admin,
            tournament_address,
            vector[vector::pop_back(&mut tokens)],
        );
        assert!(vector::length(&game_addresses) == 0, 1001);
        test_utils::fast_forward_seconds(10);

        let game_addresses = aptos_tournament::add_players_to_game_returning(
            admin,
            tournament_address,
            tokens,
        );
        assert!(vector::length(&game_addresses) == 0, 1002);

        let round_address = tournament_manager::get_round_address(tournament_address);
        let (game_signers, _) = aptos_tournament::end_matchmaking_returning(admin, tournament_address);
        let game_signers = std::option::extract(&mut game_signers);

        let game_addresses = tournament::misc_utils::signers_to_addresses(&game_signers);
        assert!(vector::length(&game_addresses) == 1, 1003);

        std::debug::print(&aptos_std::string_utils::format1(&b"game_addresses: {}", game_addresses));

        (tournament_address, round_address, game_addresses, tokens)
    }

    fun setup_test(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        players: &vector<signer>,
    ): (address, address, vector<address>, vector<Object<TournamentPlayerToken>>) {
        let (tournament_address, tokens) = setup_tournament(
            aptos_framework,
            deployer,
            admin,
            players
        );
        setup_new_round(
            admin,
            tournament_address,
            tokens
        )
    }

    fun full_e2e_test_play(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        players: vector<signer>,
        indexes: vector<u64>,
    ): (vector<address>, address, option::Option<u64>) {
        let (tournament_address, _round_address, game_addresses, player_tokens) = setup_test(
            aptos_framework,
            deployer,
            admin,
            &players
        );

        // play
        let game_address = *vector::borrow(&game_addresses, 0);
        let addresses = vector::map_ref(&player_tokens, |token| {
            object::object_address(token)
        });

        let i = 0;
        let index_opt = option::none<u64>();
        vector::for_each_ref(&players, |player| {
            let index = *vector::borrow(&indexes, i);
            index_opt = roulette::commit_index_returning(player, tournament_address, game_address, index);
            i = i + 1;
        });

        (addresses, game_address, index_opt)
    }


    #[test(
        admin = @0x777,
        deployer = @tournament,
        aptos_framework = @0x1
    )]
    fun test_happy_path_4_players(
        admin: &signer,
        deployer: &signer,
        aptos_framework: &signer,
    ) {
        let signers = aptos_framework::unit_test::create_signers_for_testing(4);
        let (token_addresses, _game_address, index_opt) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            signers,
            vector[0, 1, 2, 3],
        );

        let correct_index = option::extract(&mut index_opt);
        let index = 0;
        vector::for_each(token_addresses, |token_address| {
            let does_exist = object::object_exists<token_manager::TournamentPlayerToken>(token_address);
            std::debug::print(
                &aptos_std::string_utils::format4(
                    &b"token_address: {}, index: {}, exists: {}, correct: {}",
                    token_address,
                    correct_index,
                    does_exist,
                    correct_index
                )
            );
            assert!(does_exist == (correct_index != index), index);

            index = index + 1;
        });
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        aptos_framework = @0x1
    )]
    fun test_happy_path_2_players(
        admin: &signer,
        deployer: &signer,
        aptos_framework: &signer,
    ) {
        let signers = aptos_framework::unit_test::create_signers_for_testing(2);
        let (token_addresses, _game_address, index_opt) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            signers,
            vector[0, 1],
        );

        let correct_index = option::extract(&mut index_opt);
        let index = 0;
        vector::for_each(token_addresses, |token_address| {
            let does_exist = object::object_exists<token_manager::TournamentPlayerToken>(token_address);
            std::debug::print(
                &aptos_std::string_utils::format4(
                    &b"token_address: {}, index: {}, exists: {}, correct: {}",
                    token_address,
                    correct_index,
                    does_exist,
                    correct_index
                )
            );
            assert!(does_exist == (correct_index != index), index);

            index = index + 1;
        });
    }
}
