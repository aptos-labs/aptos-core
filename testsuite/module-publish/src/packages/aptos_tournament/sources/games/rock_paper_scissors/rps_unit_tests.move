#[test_only]
module tournament::rps_unit_tests {
    use std::account;
    use std::hash;
    use std::option;
    use std::option::Option;
    use std::signer;
    use std::string;
    use std::vector;
    use aptos_framework::object::Object;

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::rock_paper_scissors::{Self, RockPaperScissorsGame};
    use tournament::test_utils;
    use tournament::token_manager;
    use tournament::token_manager::TournamentPlayerToken;
    use tournament::tournament_manager;

    fun setup_tournament(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        user3_signer: &Option<signer>,
    ): (address, Object<TournamentPlayerToken>, Object<TournamentPlayerToken>, Option<Object<TournamentPlayerToken>>) {
        test_utils::init_test_framework(aptos_framework, 1);
        test_utils::enable_features_for_test(aptos_framework);
        account::create_account_for_test(signer::address_of(user1_signer));
        account::create_account_for_test(signer::address_of(user2_signer));
        if (option::is_some(user3_signer)) {
            account::create_account_for_test(signer::address_of(option::borrow(user3_signer)));
        };

        token_manager::init_module_for_test(deployer);
        aptos_tournament::init_module_for_test(deployer);
        admin::set_admin_signer(deployer, signer::address_of(admin));

        let tournament_address = aptos_tournament::create_new_tournament_returning(deployer);
        aptos_tournament::set_tournament_joinable(admin, tournament_address);
        // Ensure this works
        let _admin2 = admin::get_admin_signer_as_admin(admin);

        let p1_token = tournament_manager::join_tournament_with_return(
            user1_signer,
            tournament_address,
            string::utf8(b"1")
        );
        let p2_token = tournament_manager::join_tournament_with_return(
            user2_signer,
            tournament_address,
            string::utf8(b"2")
        );

        let p3_token = if (option::is_some(user3_signer)) {
            option::some(
                tournament_manager::join_tournament_with_return(
                    option::borrow(user3_signer),
                    tournament_address,
                    string::utf8(b"3")
                )
            )
        } else {
            option::none<Object<TournamentPlayerToken>>()
        };

        (tournament_address, p1_token, p2_token, p3_token)
    }

    fun setup_new_round(
        admin: &signer,
        tournament_address: address,
        p1_token: Object<TournamentPlayerToken>,
        p2_token: Object<TournamentPlayerToken>,
        p3_token: &Option<Object<TournamentPlayerToken>>,
    ): (address, address, vector<address>) {
        aptos_tournament::start_new_round<RockPaperScissorsGame>(admin, tournament_address);

        let players = vector[p1_token, p2_token];
        if (option::is_some(p3_token)) {
            vector::push_back(&mut players, *option::borrow(p3_token));
        };

        let game_addresses = aptos_tournament::add_players_to_game_returning(
            admin,
            tournament_address,
            vector[p1_token]
        );
        assert!(vector::length(&game_addresses) == 0, 1001);
        test_utils::fast_forward_seconds(10);

        let game_addresses = aptos_tournament::add_players_to_game_returning(
            admin,
            tournament_address,
            vector[p2_token]
        );
        assert!(vector::length(&game_addresses) == 0, 1002);
        test_utils::fast_forward_seconds(10);

        let round_address = tournament_manager::get_round_address(tournament_address);

        let (game_signers, _) = aptos_tournament::end_matchmaking_returning(admin, tournament_address);
        let game_signers = option::extract(&mut game_signers);
        let game_addresses = tournament::misc_utils::signers_to_addresses(&game_signers);
        assert!(vector::length(&game_addresses) == 1, 1003);

        let tournament_signer = admin::get_tournament_owner_signer(tournament_address);
        vector::enumerate_ref(&game_signers, |i, signer| {
            assert!(test_utils::object_signer_to_owner(signer) == signer::address_of(&tournament_signer), 2000 + i);
        });

        (tournament_address, round_address, game_addresses)
    }

    fun setup_test(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        user3_signer: &Option<signer>,
    ): (address, address, vector<address>) {
        let (tournament_address, p1_token, p2_token, p3_token) = setup_tournament(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            user3_signer,
        );
        setup_new_round(
            admin,
            tournament_address,
            p1_token,
            p2_token,
            &p3_token,
        )
    }

    fun player_commit(player: &signer, game_address: address, action: vector<u8>, hash_addition: vector<u8>) {
        let combo = copy action;
        vector::append(&mut combo, hash_addition);
        rock_paper_scissors::commit_action(player, game_address, hash::sha3_256(combo));
    }

    fun full_e2e_test_play(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        player1: &signer,
        player2: &signer,
        player3: &Option<signer>,
        action1: vector<u8>,
        action2: vector<u8>,
        // 0: no one goes. 1: first goes. 2: second goes. 3: all go
        move_players: u8,
    ): (vector<address>, vector<address>, address) {
        let (_tournament_address, _round_address, game_addresses) = setup_test(
            aptos_framework,
            deployer,
            admin,
            player1,
            player2,
            player3,
        );

        play_two_players(
            player1,
            player2,
            action1,
            action2,
            move_players,
            game_addresses,
        )
    }

    fun play_two_players(
        player1: &signer,
        player2: &signer,
        action1: vector<u8>,
        action2: vector<u8>,
        // 0: no one goes. 1: first goes. 2: second goes. 3: all go
        move_players: u8,
        game_addresses: vector<address>,
    ): (vector<address>, vector<address>, address) {
        let game_address = *vector::borrow(&game_addresses, 0);
        let hash_addition1 = b"random uuid 1";
        let hash_addition2 = b"random uuid 2";

        player_commit(player1, game_address, action1, hash_addition1);
        player_commit(player2, game_address, action2, hash_addition2);
        if (move_players == 1 || move_players == 3) {
            let (is_game_over, _winners, _losers) = rock_paper_scissors::verify_action_returning(
                player1,
                game_address,
                action1,
                hash_addition1
            );
            assert!(!is_game_over, 0);
        };

        let winners = vector[];
        let losers = vector[];
        if (move_players == 2 || move_players == 3) {
            let (_is_game_over, winnersi, losersi) = rock_paper_scissors::verify_action_returning(
                player2,
                game_address,
                action2,
                hash_addition2
            );
            winners = winnersi;
            losers = losersi;
        };

        (winners, losers, game_address)
    }

    inline fun assert_get_winner(winners: vector<address>): address {
        assert!(vector::length(&winners) == 1, 123456789);
        *vector::borrow(&winners, 0)
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    #[expected_failure]
    fun test_hash_mismatch(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (_tournament_address, _round_address, game_addresses) = setup_test(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
        );
        let game_address = *vector::borrow(&game_addresses, 0);

        let hash_addition1 = b"random uuid 1";
        let hash_addition2 = b"random uuid 2";

        let bad_hash_addition = b"bad hash addition";

        let action1 = b"Rock";
        let action2 = b"Paper";

        player_commit(user1_signer, game_address, action1, hash_addition1);
        player_commit(user2_signer, game_address, action2, hash_addition2);

        rock_paper_scissors::verify_action_returning(user1_signer, game_address, action1, bad_hash_addition);
        rock_paper_scissors::verify_action_returning(user2_signer, game_address, action2, bad_hash_addition);
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    fun test_first_player_didnt_move(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (_winners, _losers, game_address) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
            b"Rock",
            b"Paper",
            2,
        );
        let results = rock_paper_scissors::handle_games_end_returning(admin, vector[game_address]);
        let winners_and_losers = vector::pop_back(&mut results);
        let _losers = vector::pop_back(&mut winners_and_losers);
        let winners = vector::pop_back(&mut winners_and_losers);

        let winner = assert_get_winner(winners);
        assert!(winner == signer::address_of(user2_signer), 1);
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    fun test_second_player_didnt_move(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (_winners, _losers, game_address) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
            b"Rock",
            b"Paper",
            1,
        );
        let results = rock_paper_scissors::handle_games_end_returning(admin, vector[game_address]);
        let winners_and_losers = vector::pop_back(&mut results);
        let _losers = vector::pop_back(&mut winners_and_losers);
        let winners = vector::pop_back(&mut winners_and_losers);

        let winner = assert_get_winner(winners);
        assert!(winner == signer::address_of(user1_signer), 1);
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    fun test_neither_player_moved(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (_winners, _losers, game_address) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
            b"Rock",
            b"Paper",
            0,
        );
        let results = rock_paper_scissors::handle_games_end_returning(admin, vector[game_address]);
        let winners_and_losers = vector::pop_back(&mut results);
        let losers = vector::pop_back(&mut winners_and_losers);
        let winners = vector::pop_back(&mut winners_and_losers);
        assert!(vector::length(&winners) == 0, 1);
        assert!(vector::length(&losers) == 2, 2);
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    fun test_rock_paper(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (winners, _losers, game_address) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
            b"Rock",
            b"Paper",
            3,
        );
        rock_paper_scissors::handle_games_end(admin, vector[game_address]);

        let winner = assert_get_winner(winners);
        assert!(winner == signer::address_of(user2_signer), 1);
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    fun test_rock_scissors(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (winners, _losers, game_address) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
            b"Rock",
            b"Scissors",
            3,
        );
        rock_paper_scissors::handle_games_end(admin, vector[game_address]);

        let winner = assert_get_winner(winners);
        assert!(winner == signer::address_of(user1_signer), 2);
    }

    #[test(
        admin = @0x777,
        deployer = @tournament,
        user1_signer = @0x123,
        user2_signer = @0x456,
        aptos_framework = @0x1
    )]
    fun test_rock_rock(
        admin: &signer,
        deployer: &signer,
        user1_signer: &signer,
        user2_signer: &signer,
        aptos_framework: &signer,
    ) {
        let (winners, losers, game_address) = full_e2e_test_play(
            aptos_framework,
            deployer,
            admin,
            user1_signer,
            user2_signer,
            &option::none(),
            b"Rock",
            b"Rock",
            3,
        );
        rock_paper_scissors::handle_games_end(admin, vector[game_address]);

        assert!(vector::length(&losers) == 0, 10);
        assert!(vector::length(&winners) == 2, 11);
        assert!(vector::contains(&winners, &signer::address_of(user1_signer)), 12);
        assert!(vector::contains(&winners, &signer::address_of(user2_signer)), 13);
    }
}
