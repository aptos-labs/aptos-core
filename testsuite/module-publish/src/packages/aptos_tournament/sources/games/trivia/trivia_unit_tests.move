#[test_only]
module tournament::trivia_unit_tests {
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_std::simple_map::{Self, SimpleMap};
    use aptos_framework::account;
    use aptos_framework::object::Self;

    use tournament::admin;
    use tournament::aptos_tournament;
    use tournament::test_utils;
    use tournament::token_manager::{Self, TournamentPlayerToken};
    use tournament::tournament_manager;
    use tournament::trivia::{Self, TriviaGame};

    const CHALLENGE: vector<u8> = b"Blah blah blah.";


    const QUESTION_BANK: vector<vector<u8>> = vector<vector<u8>> [
        b"What's up with airline food anyway?",
        b"question 2",
        b"question 3",
        b"question 4",
    ];

    // These would obviously not be on-chain. But they could all have the same challenge.
    const ANSWER_BANK: vector<vector<u8>> = vector<vector<u8>> [
        b"It stinks.",
        b"It's bad.",
        b"It is gross.",
        b"I hate airplane food.",
    ];

    public fun get_question_answer_bank(): SimpleMap<String, String> {
        let question_bank = vector::map<vector<u8>, String>(QUESTION_BANK, |question| {
            string::utf8(question)
        });
        let answer_bank = vector::map<vector<u8>, String>(ANSWER_BANK, |answer| {
            string::utf8(answer)
        });

        simple_map::new_from(question_bank, answer_bank)
    }

    public fun get_question_answer_and_hash(n: u64): (String, vector<String>) {
        let question_bank = get_question_answer_bank();
        let (questions, possible_answers) = simple_map::to_vec_pair(question_bank);
        let question = *vector::borrow(&questions, n);
        // let answer = *vector::borrow(&possible_answers, 0);
        // let answer_bytes = bcs::to_bytes(&possible_answers);
        // vector::append(&mut answer_bytes, CHALLENGE);
        // let hashed_answer = hash::sha3_256(answer_bytes);

        (question, possible_answers)
    }

    fun setup_test(
        aptos_framework: &signer,
        deployer: &signer,
        admin: &signer,
        // This player chooses an invalid choice
        user1_signer: &signer,
        // This player is right
        user2_signer: &signer,
        // This player doesn't choose
        user3_signer: &signer,
        // This player doesn't join
        user4_signer: &signer,
    ): (address, address, vector<address>, vector<address>) {
        test_utils::init_test_framework(aptos_framework, 1);
        test_utils::enable_features_for_test(aptos_framework);
        account::create_account_for_test(signer::address_of(user1_signer));
        account::create_account_for_test(signer::address_of(user2_signer));
        account::create_account_for_test(signer::address_of(user3_signer));
        account::create_account_for_test(signer::address_of(user4_signer));

        token_manager::init_module_for_test(deployer);
        aptos_tournament::init_module_for_test(deployer);
        admin::set_admin_signer(deployer, signer::address_of(admin));

        let tournament_address = aptos_tournament::create_new_tournament_returning(deployer);

        let admin2 = admin::get_admin_signer_as_admin(admin);

        tournament_manager::set_tournament_joinable(&admin2, tournament_address);

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
        let p3_token = tournament_manager::join_tournament_with_return(
            user3_signer,
            tournament_address,
            string::utf8(b"2")
        );
        let p4_token = tournament_manager::join_tournament_with_return(
            user4_signer,
            tournament_address,
            string::utf8(b"2")
        );
        aptos_tournament::start_new_round<TriviaGame>(admin, tournament_address);

        let player_tokens = vector[p1_token, p2_token, p3_token, p4_token];
        let player_token_addresses = vector::map_ref(&player_tokens, |player_token| {
            object::object_address(player_token)
        });

        // Player 4 never joins
        p4_token = vector::pop_back(&mut player_tokens);
        let game_addresses = aptos_tournament::add_players_to_game_by_address_returning(
            admin,
            tournament_address,
            player_token_addresses,
        );
        assert!(vector::length(&game_addresses) == 0, 101);
        vector::push_back(&mut player_tokens, p4_token);

        aptos_tournament::end_matchmaking(admin, tournament_address);


        let round_address = tournament_manager::get_round_address(tournament_address);
        (tournament_address, round_address, game_addresses, player_token_addresses)
    }


    #[test(admin = @0x777,
        deployer = @tournament,
        player_1 = @0x111,
        player_2 = @0x222,
        player_3 = @0x333,
        player_4 = @0x444,
        aptos_framework = @0x1
    )]
    fun test_two_player_happy_path_trivia(
        admin: &signer,
        deployer: &signer,
        player_1: &signer,
        player_2: &signer,
        player_3: &signer,
        player_4: &signer,
        aptos_framework: &signer,
    ) {
        let (tournament_address, _round_address, _game_addresses, player_token_addresses) = setup_test(
            aptos_framework,
            deployer,
            admin,
            player_1,
            player_2,
            player_3,
            player_4,
        );

        let round_address = tournament_manager::get_round_address(tournament_address);

        let (question, possible_answers) = get_question_answer_and_hash(0);

        let answer: u8 = 0;

        let player_1_token_address = *vector::borrow(&player_token_addresses, 0);
        let player_2_token_address = *vector::borrow(&player_token_addresses, 1);

        let player_3_token_address = *vector::borrow(&player_token_addresses, 2);
        let player_4_token_address = *vector::borrow(&player_token_addresses, 3);

        trivia::set_trivia_question(
            admin,
            tournament_address,
            question,
            possible_answers,
        );

        std::debug::print(&trivia::view_trivia(tournament_address)); // for DEBUG printing.

        trivia::commit_answer(
            player_1,
            object::address_to_object<TournamentPlayerToken>(player_1_token_address),
            // Test out of bounds, too!
            55,
        );
        std::debug::print(&trivia::view_trivia_player(player_1_token_address)); // for DEBUG printing.

        trivia::commit_answer(
            player_2,
            object::address_to_object<TournamentPlayerToken>(player_2_token_address),
            answer,
        );

        std::debug::print(&trivia::view_trivia_player(player_2_token_address)); // for DEBUG printing.

        trivia::reveal_answer(
            admin,
            tournament_address,
            answer,
        );

        std::debug::print(&trivia::view_trivia(tournament_address)); // for DEBUG printing.
        assert!(
            &trivia::view_revealed_answer(tournament_address) == vector::borrow(&possible_answers, (answer as u64)),
            0
        );

        assert!(object::is_object(player_1_token_address), 1001);
        assert!(object::is_object(player_2_token_address), 1002);
        assert!(object::is_object(player_3_token_address), 1003);
        assert!(object::is_object(player_4_token_address), 1004);

        trivia::handle_players_game_end_by_address(
            admin,
            tournament_address,
            vector[player_1_token_address, player_2_token_address, player_3_token_address, player_4_token_address],
        );

        assert!(!object::object_exists<token_manager::TournamentPlayerToken>(player_1_token_address), 1001);
        assert!(object::object_exists<token_manager::TournamentPlayerToken>(player_2_token_address), 1002);
        assert!(!object::object_exists<token_manager::TournamentPlayerToken>(player_3_token_address), 1003);
        assert!(!object::object_exists<token_manager::TournamentPlayerToken>(player_4_token_address), 1004);

        aptos_tournament::cleanup_current_round(admin, tournament_address);
        assert!(!trivia::is_trivia(round_address), 1);

        aptos_tournament::start_new_round<TriviaGame>(admin, tournament_address);
    }
}
