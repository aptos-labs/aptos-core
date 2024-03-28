#[test_only]
module tournament::main_unit_test {
/*
    use tournament::misc_utils;
    use tournament::token_manager;
    use tournament::tournament_manager;
    use tournament::rock_paper_scissor;
    use tournament::object_refs;
    use tournament::trivia::{Self};
    use tournament::lobby::{Self, RockPaperScissorsGame, TriviaGame};
    use tournament::test_setup;
    use aptos_framework::account;
    use std::signer;
    use std::vector;
    use std::option;
    use std::hash;
    use std::string_utils;
    use aptos_token_objects::token::{Token};
    use aptos_framework::object::{Self, Object, ConstructorRef};
    use std::string::{Self, String};

    const ROCK: vector<u8> = b"Rock";
    const PAPER: vector<u8> = b"Paper";
    const SCISSOR: vector<u8> = b"Scissor";
    const ROCK_SALT: vector<u8> = b"Rock solid.";
    const PAPER_SALT: vector<u8> = b"Paper thin.";
    const SCISSOR_SALT: vector<u8> = b"Scissor sharp.";
    const CHALLENGE: vector<u8> = b"Blah blah blah.";

    // the deterministic "random" object from initialize_tournament
    const TOURNAMENT_ADDRESS: address = @0xfab16b00983f01e5c2b7682472a4f4c3e5929fbba987958570b6290c02817df2;

    // acts as a seed to pseudo-randomize things that call `misc_utils::rand_range`
    const START_TIME_SEED: u64 = 1;

    fun action_helper(
        action: vector<u8>,
        challenge: vector<u8>,
    ): (vector<u8>, vector<u8>, vector<u8>) {
        let with_challenge = action;
        vector::append(&mut with_challenge, challenge);
        (action, challenge, hash::sha3_256(with_challenge))
    }

    fun get_random_action(): (vector<u8>, vector<u8>) {
        let rand_idx = misc_utils::rand_range(0, 2);
        if (rand_idx == 0) {
            (ROCK, ROCK_SALT)
        } else if (rand_idx == 1) {
            (PAPER, PAPER_SALT)
        } else if (rand_idx == 2) {
            (SCISSOR, SCISSOR_SALT)
        } else {
            abort 0
        }
    }

    fun commit_and_verify(
        player: &signer,
        player_obj: Object<TournamentPlayerToken>,
        action: vector<u8>,
        challenge: vector<u8>,
    ) {
        let (action, challenge, action_hashed) = action_helper(action, challenge);
        rock_paper_scissor::commit_action(player, player_obj, action_hashed);
        rock_paper_scissor::verify_action(player, player_obj, action, challenge);
    }

    fun filter_tokens_and_return_signers(
        player_tokens: vector<Object<TournamentPlayerToken>>,
    ): (vector<signer>, vector<Object<TournamentPlayerToken>>) {
        let player_tokens = vector::filter(player_tokens, |player_token| {
            object::is_object(object::object_address<Token>(player_token))

        });
        let players = vector::map<Object<TournamentPlayerToken>, signer>(player_tokens, |player_token| {
            let player_addr = object::owner(player_token);
            object_refs::get_signer(player_addr)
        });
        (players, player_tokens)
    }

    #[test(deployer=@tournament, tournament_creator=@0xFFFFFFFFFFF, p1=@0xA, p2=@0xB, aptos_framework=@0x1)]
    fun test_rps_tournament_happy_path(
        deployer: &signer,
        tournament_creator: &signer,
        p1: &signer,
        p2: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(deployer, aptos_framework, START_TIME_SEED);
        let p1_addr = signer::address_of(p1);
        let p2_addr = signer::address_of(p2);
        account::create_account_for_test(p1_addr);
        account::create_account_for_test(p2_addr);
        tournament_manager::initialize_tournament(
            tournament_creator,
            string::utf8(b"test_tournament"),
            2, // max players
            1, // num winners
            1, // time between round seconds
        );

        tournament_manager::join_tournament(p1, TOURNAMENT_ADDRESS, string::utf8(b"player_1"));
        tournament_manager::join_tournament(p2, TOURNAMENT_ADDRESS, string::utf8(b"player_2"));

        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
        let players = lobby::get_current_player_objects_indexing(TOURNAMENT_ADDRESS);
        let player_1_token = *vector::borrow(&players, 0);
        let player_2_token = *vector::borrow(&players, 1);

        let rock = b"Rock";
        let challenge_1: vector<u8> = b"Rock solid.";
        let rock_with_challenge = rock;
        vector::append(&mut rock_with_challenge, challenge_1);
        let rock_action_hashed = hash::sha3_256(rock_with_challenge);

        let paper = b"Paper";
        let challenge_2: vector<u8> = b"Paper thin.";
        let paper_with_challenge = paper;
        vector::append(&mut paper_with_challenge, challenge_2);
        let paper_action_hashed = hash::sha3_256(paper_with_challenge);
        let room_addr_player_1 = token_manager::get_room_address(player_1_token);
        let room_addr_player_2 = token_manager::get_room_address(player_2_token);
        assert!(room_addr_player_1 == room_addr_player_2, 0);
        rock_paper_scissor::commit_action(p1, player_1_token, rock_action_hashed);
        rock_paper_scissor::commit_action(p2, player_2_token, paper_action_hashed);
        rock_paper_scissor::verify_action(p1, player_1_token, rock, challenge_1);
        rock_paper_scissor::verify_action(p2, player_2_token, paper, challenge_2);
        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
    }

    // uses objects to get signers easier
    #[test(deployer=@tournament, tournament_creator=@0xFFFFFFFFFFF, aptos_framework=@0x1)]
    fun test_tournament_players_as_objects(
        deployer: &signer,
        tournament_creator: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(deployer, aptos_framework, START_TIME_SEED);
        tournament_manager::initialize_tournament(
            tournament_creator,
            string::utf8(b"test_tournament"),
            2, // max players
            1, // num winners
            1, // time between round seconds
        );

        let p1_cref = object::create_object(@0xA);
        let p2_cref = object::create_object(@0xA);
        let p1 = &object::generate_signer(&p1_cref);
        let p2 = &object::generate_signer(&p2_cref);

        tournament_manager::join_tournament(p1, TOURNAMENT_ADDRESS, string::utf8(b"player_1"));
        tournament_manager::join_tournament(p2, TOURNAMENT_ADDRESS, string::utf8(b"player_2"));

        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
        let players = lobby::get_current_player_objects_indexing(TOURNAMENT_ADDRESS);
        let player_1_token = *vector::borrow(&players, 0);
        let player_2_token = *vector::borrow(&players, 1);

        commit_and_verify(p1, player_1_token, ROCK, ROCK_SALT);
        commit_and_verify(p2, player_2_token, PAPER, PAPER_SALT);
        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
    }

    #[test(deployer=@tournament, tournament_creator=@0xFFFFFFFFFFF, aptos_framework=@0x1)]
    fun test_4_tournament_players(
        deployer: &signer,
        tournament_creator: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(deployer, aptos_framework, START_TIME_SEED);
        tournament_manager::initialize_tournament(
            tournament_creator,
            string::utf8(b"test_tournament"),
            4, // max players
            2, // num winners
            1, // time between round seconds
        );

        let player_constructor_refs = vector<ConstructorRef> [];
        let player_token_addresses = vector<address> [];
        let i = 0;
        while(i < 4) {
            let p_cref = object::create_object(@0xA);
            let p = &object::generate_signer(&p_cref);
            let player_token_addr = tournament_manager::join_tournament_for_test(p, TOURNAMENT_ADDRESS, string_utils::to_string(&i));
            vector::push_back(&mut player_token_addresses, player_token_addr);
            vector::push_back(&mut player_constructor_refs, p_cref);
            i = i + 1;
        };

        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);

        vector::zip(player_constructor_refs, player_token_addresses, |constructor_ref, player_token_address| {
            let player_token = object::address_to_object<Token>(player_token_address);
            let (action, challenge) = get_random_action();
            let player = &object::generate_signer(&constructor_ref);
            commit_and_verify(player, player_token, action, challenge);
        });

        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
    }

    #[test(deployer=@tournament, tournament_creator=@0xFFFFFFFFFFF, aptos_framework=@0x1)]
    fun test_100_tournament_players(
        deployer: &signer,
        tournament_creator: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(deployer, aptos_framework, START_TIME_SEED);
        let num_players: u64 = 100;
        let num_winners: u64 = num_players / 2;
        tournament_manager::initialize_tournament(
            tournament_creator,
            string::utf8(b"test_tournament"),
            num_players, // max players
            num_winners, // num winners
            1, // time between round seconds
        );

        let player_constructor_refs = vector<ConstructorRef> [];
        let player_token_addresses = vector<address> [];
        let players = vector<signer> [];
        let i = 0;
        while(i < num_players) {
            let p_cref = object::create_object(@0xA);
            let p = object::generate_signer(&p_cref);
            object_refs::create_refs<Token>(&p_cref);
            let player_token_addr = tournament_manager::join_tournament_for_test(&p, TOURNAMENT_ADDRESS, string_utils::to_string(&i));
            vector::push_back(&mut player_token_addresses, player_token_addr);
            vector::push_back(&mut player_constructor_refs, p_cref);
            vector::push_back(&mut players, p);
            i = i + 1;
        };

        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
        let player_tokens = lobby::get_current_player_objects_indexing(TOURNAMENT_ADDRESS);
        // align players with player tokens with this next call, lest they aren't matched up correctly bc of out of order deletion
        let (players, player_tokens) = filter_tokens_and_return_signers(player_tokens);

        let (players, player_tokens) = commit_and_verify_all(players, player_tokens);

        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);

        let (players, player_tokens) = commit_and_verify_all(players, player_tokens);
        // print_all_objs(player_tokens);
        tournament_manager::start_new_round<RockPaperScissorsGame>(tournament_creator, TOURNAMENT_ADDRESS, vector<String> []);
        commit_and_verify_all(players, player_tokens);
        // print_all_objs(player_tokens);
    }

    inline fun print_all_objs(
        player_tokens: vector<Object<TournamentPlayerToken>>,
    ) {
        vector::for_each(player_tokens, |player_token| {
            if (object::is_object(object::object_address<Token>(&player_token))) {
                let rps_state = rock_paper_scissor::get_player_rps_state(player_token);
                if (option::is_some(&rps_state)) {
                };
            };
        });
    }

    // Takes in all player tokens and ignores those that have been deleted
    inline fun commit_and_verify_all(
        players: vector<signer>,
        player_tokens: vector<Object<TournamentPlayerToken>>,
    ): (vector<signer>, vector<Object<TournamentPlayerToken>>) {
        vector::zip(players, player_tokens, |player, player_token| {
            let player_token_addr = object::object_address<Token>(&player_token);
            if (object::is_object(player_token_addr) && rock_paper_scissor::player_exists(player_token_addr)) {
                let (action, challenge) = get_random_action();
                commit_and_verify(&player, player_token, action, challenge);
            };
        });
        filter_tokens_and_return_signers(player_tokens)
    }

    // test if we can view the tournament state info if we don't have any players who've joined yet
    #[test(deployer=@tournament, tournament_creator=@0xFFFFFFFFFFF, aptos_framework=@0x1)]
    fun test_lobby_not_exist(
        deployer: &signer,
        tournament_creator: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(deployer, aptos_framework, START_TIME_SEED);
        let num_players: u64 = 50;
        let num_winners: u64 = num_players / 2;
        tournament_manager::initialize_tournament(
            tournament_creator,
            string::utf8(b"test_tournament"),
            num_players, // max players
            num_winners, // num winners
            1, // time between round seconds
        );
        // let tournament_info = tournament_manager::get_tournament_state(TOURNAMENT_ADDRESS);
        // std::debug::print(&tournament_info);
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                    Trivia Tournament                                   //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    #[test(tournament_manager=@tournament, p1=@0xA, p2=@0xB, aptos_framework=@0x1)]
    fun test_trivia_tournament_happy_path(
        tournament_manager: &signer,
        p1: &signer,
        p2: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(tournament_manager, aptos_framework, START_TIME_SEED);
        test_setup::setup_trivia(tournament_manager);
        let p1_addr = signer::address_of(p1);
        let p2_addr = signer::address_of(p2);
        account::create_account_for_test(p1_addr);
        account::create_account_for_test(p2_addr);
        tournament_manager::initialize_tournament(
            tournament_manager,
            string::utf8(b"test_tournament"),
            2, // max players
            1, // num winners
            1, // time between round seconds
        );

        tournament_manager::join_tournament(p1, TOURNAMENT_ADDRESS, string::utf8(b"player_1"));
        tournament_manager::join_tournament(p2, TOURNAMENT_ADDRESS, string::utf8(b"player_2"));

        let (question, possible_answers) = test_setup::get_question_answer_and_hash(0);
        let answer = *vector::borrow(&possible_answers, 0);
        let args = vector<String> [ question ];
        vector::append(&mut args, possible_answers);

        tournament_manager::start_new_round<TriviaGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            args,
        );

        let players = lobby::get_current_player_objects_indexing(TOURNAMENT_ADDRESS);
        let player_1_token = *vector::borrow(&players, 0);
        let player_2_token = *vector::borrow(&players, 1);
        trivia::answer_for_test(p1, player_1_token, string::utf8(b"I'm wrong."));
        trivia::answer_for_test(p2, player_2_token, answer);

        // all will have the same room address
        let room_addr = token_manager::get_room_address(player_1_token);

        trivia::reveal_answer(
            tournament_manager,
            room_addr,
            // CHALLENGE,
            answer,
        );

        assert!(trivia::view_revealed_answer(room_addr) == answer, 0);
        // let (winners, _losers) = trivia::get_results(room_addr);
        trivia::cleanup_game(room_addr);
        assert!(!trivia::is_game(room_addr), 0);
        assert!(!trivia::is_trivia(room_addr), 0);
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                  Multi-game tournament                                 //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    #[test(tournament_manager=@tournament, p1=@0xA, p2=@0xB, p3=@0xC, aptos_framework=@0x1)]
    fun test_multi_game_tournament_happy_path(
        tournament_manager: &signer,
        p1: &signer,
        p2: &signer,
        p3: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(tournament_manager, aptos_framework, START_TIME_SEED);
        test_setup::setup_trivia(tournament_manager);
        let p1_addr = signer::address_of(p1);
        let p2_addr = signer::address_of(p2);
        let p3_addr = signer::address_of(p3);
        account::create_account_for_test(p1_addr);
        account::create_account_for_test(p2_addr);
        account::create_account_for_test(p3_addr);
        tournament_manager::initialize_tournament(
            tournament_manager,
            string::utf8(b"test_tournament"),
            10, // max players
            1, // num winners
            1, // time between round seconds
        );

        tournament_manager::join_tournament(p1, TOURNAMENT_ADDRESS, string::utf8(b"player_1"));
        tournament_manager::join_tournament(p2, TOURNAMENT_ADDRESS, string::utf8(b"player_2"));
        tournament_manager::join_tournament(p3, TOURNAMENT_ADDRESS, string::utf8(b"player_3"));

        let (question, possible_answers) = test_setup::get_question_answer_and_hash(0);
        let answer = *vector::borrow(&possible_answers, 0);
        let args = vector<String> [ question ];
        vector::append(&mut args, possible_answers);

        tournament_manager::start_new_round<TriviaGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            args,
        );

        let players = lobby::get_current_player_objects_indexing(TOURNAMENT_ADDRESS);
        let player_1_token = *vector::borrow(&players, 0);
        let player_2_token = *vector::borrow(&players, 1);
        let player_3_token = *vector::borrow(&players, 2);
        trivia::answer_for_test(p1, player_1_token, string::utf8(b"I'm wrong."));
        trivia::answer_for_test(p2, player_2_token, answer);
        trivia::answer_for_test(p3, player_3_token, answer);

        // all will have the same room address
        let room_addr = token_manager::get_room_address(player_1_token);

        trivia::reveal_answer(
            tournament_manager,
            room_addr,
            // CHALLENGE,
            answer,
        );

        assert!(trivia::view_revealed_answer(room_addr) == answer, 0);
        // let (winners, _losers) = trivia::get_results(room_addr);
        let (question, possible_answers) = test_setup::get_question_answer_and_hash(0);
        let answer = *vector::borrow(&possible_answers, 0);
        let args = vector<String> [ question ];
        vector::append(&mut args, possible_answers);

        tournament_manager::start_new_round<TriviaGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            args,
        );
        trivia::answer_for_test(p2, player_2_token, answer);
        trivia::answer_for_test(p3, player_3_token, answer);
        let room_addr = token_manager::get_room_address(player_2_token);
        trivia::reveal_answer(
            tournament_manager,
            room_addr,
            answer,
        );
        tournament_manager::start_new_round<RockPaperScissorsGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            vector<String> [],
        );
        let rock = b"Rock";
        let challenge_1: vector<u8> = b"Rock solid.";
        let rock_with_challenge = rock;
        vector::append(&mut rock_with_challenge, challenge_1);
        let rock_action_hashed = hash::sha3_256(rock_with_challenge);

        let paper = b"Paper";
        let challenge_2: vector<u8> = b"Paper thin.";
        let paper_with_challenge = paper;
        vector::append(&mut paper_with_challenge, challenge_2);
        let paper_action_hashed = hash::sha3_256(paper_with_challenge);
        let room_addr_player_2 = token_manager::get_room_address(player_2_token);
        let room_addr_player_3 = token_manager::get_room_address(player_3_token);
        assert!(room_addr_player_2 == room_addr_player_3, 0);
        rock_paper_scissor::commit_action(p2, player_2_token, rock_action_hashed);
        rock_paper_scissor::commit_action(p3, player_3_token, paper_action_hashed);
        rock_paper_scissor::verify_action(p2, player_2_token, rock, challenge_1);
        rock_paper_scissor::verify_action(p3, player_3_token, paper, challenge_2);

        tournament_manager::start_new_round<RockPaperScissorsGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            vector<String> [],
        );

        assert!(object::is_object(object::object_address<Token>(&player_3_token)), 0);
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //           Multi-game tournament with back and forth rounds between two games           //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    #[test(tournament_manager=@tournament, p1=@0xA, p2=@0xB, p3=@0xC, p4=@0xD, aptos_framework=@0x1)]
    fun test_back_and_forth_multi_game_tournament_complex(
        tournament_manager: &signer,
        p1: &signer,
        p2: &signer,
        p3: &signer,
        p4: &signer,
        aptos_framework: &signer
    ) {
        test_setup::setup_test(tournament_manager, aptos_framework, START_TIME_SEED);
        test_setup::setup_trivia(tournament_manager);
        let p1_addr = signer::address_of(p1);
        let p2_addr = signer::address_of(p2);
        let p3_addr = signer::address_of(p3);
        let p4_addr = signer::address_of(p4);
        account::create_account_for_test(p1_addr);
        account::create_account_for_test(p2_addr);
        account::create_account_for_test(p3_addr);
        account::create_account_for_test(p4_addr);
        tournament_manager::initialize_tournament(
            tournament_manager,
            string::utf8(b"test_tournament"),
            10, // max players
            1, // num winners
            1, // time between round seconds
        );

        tournament_manager::join_tournament(p1, TOURNAMENT_ADDRESS, string::utf8(b"player_1"));
        tournament_manager::join_tournament(p2, TOURNAMENT_ADDRESS, string::utf8(b"player_2"));
        tournament_manager::join_tournament(p3, TOURNAMENT_ADDRESS, string::utf8(b"player_3"));
        tournament_manager::join_tournament(p4, TOURNAMENT_ADDRESS, string::utf8(b"player_4"));

        let (question, possible_answers) = test_setup::get_question_answer_and_hash(0);
        let answer = *vector::borrow(&possible_answers, 0);
        let args = vector<String> [ question ];
        vector::append(&mut args, possible_answers);

        tournament_manager::start_new_round<TriviaGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            args,
        );

        let players = lobby::get_current_player_objects_indexing(TOURNAMENT_ADDRESS);
        let player_actual_addresses = vector<address> [ p1_addr, p2_addr, p3_addr, p4_addr ];
        let players = vector::map(player_actual_addresses, |player_addr| {
            let (found, index) = vector::find(&players, |player| {
                object::owner(*player) == player_addr
            });
            assert!(found, 0);
            *vector::borrow(&players, index)
        });
        let player_1_token = *vector::borrow(&players, 0);
        let player_2_token = *vector::borrow(&players, 1);
        let player_3_token = *vector::borrow(&players, 2);
        let player_4_token = *vector::borrow(&players, 3);
        trivia::answer_for_test(p1, player_1_token, string::utf8(b"I'm wrong."));
        trivia::answer_for_test(p2, player_2_token, answer);
        trivia::answer_for_test(p3, player_3_token, answer);
        trivia::answer_for_test(p4, player_4_token, answer);

        // all will have the same room address
        let room_addr = token_manager::get_room_address(player_1_token);

        trivia::reveal_answer(
            tournament_manager,
            room_addr,
            // CHALLENGE,
            answer,
        );
        assert!(trivia::view_revealed_answer(room_addr) == answer, 0);
        // let (winners, _losers) = trivia::get_results(room_addr);
        let (question, possible_answers) = test_setup::get_question_answer_and_hash(0);
        let answer = *vector::borrow(&possible_answers, 0);
        let args = vector<String> [ question ];
        vector::append(&mut args, possible_answers);

        tournament_manager::start_new_round<TriviaGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            args,
        );
        trivia::answer_for_test(p2, player_2_token, answer);
        trivia::answer_for_test(p3, player_3_token, answer);
        trivia::answer_for_test(p4, player_4_token, answer);
        let room_addr = token_manager::get_room_address(player_2_token);
        trivia::reveal_answer(
            tournament_manager,
            room_addr,
            answer,
        );

        tournament_manager::start_new_round<RockPaperScissorsGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            vector<String> [],
        );

        let rock = b"Rock";
        let challenge_1: vector<u8> = b"Rock solid.";
        let rock_with_challenge = rock;
        vector::append(&mut rock_with_challenge, challenge_1);
        let rock_action_hashed = hash::sha3_256(rock_with_challenge);

        let paper = b"Paper";
        let challenge_2: vector<u8> = b"Paper thin.";
        let paper_with_challenge = paper;
        vector::append(&mut paper_with_challenge, challenge_2);
        let paper_action_hashed = hash::sha3_256(paper_with_challenge);
        let room_addr_player_2 = token_manager::get_room_address(player_2_token);
        let room_addr_player_3 = token_manager::get_room_address(player_3_token);
        assert!(room_addr_player_2 == room_addr_player_3, 0);
        rock_paper_scissor::commit_action(p2, player_2_token, rock_action_hashed);
        rock_paper_scissor::commit_action(p3, player_3_token, paper_action_hashed);
        rock_paper_scissor::verify_action(p2, player_2_token, rock, challenge_1);
        rock_paper_scissor::verify_action(p3, player_3_token, paper, challenge_2);

        // PLAYER 3 WINS and continues, PLAYER 4 has a bye

        tournament_manager::start_new_round<RockPaperScissorsGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            vector<String> [],
        );

        let rock = b"Rock";
        let challenge_1: vector<u8> = b"Rock solid.";
        let rock_with_challenge = rock;
        vector::append(&mut rock_with_challenge, challenge_1);
        let rock_action_hashed = hash::sha3_256(rock_with_challenge);

        let room_addr_player_3 = token_manager::get_room_address(player_3_token);
        let room_addr_player_4 = token_manager::get_room_address(player_4_token);
        assert!(room_addr_player_3 == room_addr_player_4, 0);
        rock_paper_scissor::commit_action(p3, player_3_token, rock_action_hashed);
        rock_paper_scissor::commit_action(p4, player_4_token, rock_action_hashed);
        rock_paper_scissor::verify_action(p3, player_3_token, rock, challenge_1);
        rock_paper_scissor::verify_action(p4, player_4_token, rock, challenge_1);

        // PLAYER 3 and PLAYER 4 TIE
        let (question, possible_answers) = test_setup::get_question_answer_and_hash(0);
        let answer = *vector::borrow(&possible_answers, 0);
        let args = vector<String> [ question ];
        vector::append(&mut args, possible_answers);

        tournament_manager::start_new_round<TriviaGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            args,
        );
        trivia::answer_for_test(p3, player_3_token, answer);
        trivia::answer_for_test(p4, player_4_token, string::utf8(b" i stupid "));
        let room_addr = token_manager::get_room_address(player_3_token);
        trivia::reveal_answer(
            tournament_manager,
            room_addr,
            answer,
        );

        tournament_manager::start_new_round<RockPaperScissorsGame>(
            tournament_manager,
            TOURNAMENT_ADDRESS,
            vector<String> [],
        );

        assert!(object::is_object(object::object_address<Token>(&player_3_token)), 0);
    }
    */
}
