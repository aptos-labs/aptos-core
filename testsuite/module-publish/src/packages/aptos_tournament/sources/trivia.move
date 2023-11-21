module tournament::trivia {
    use std::option;
    use std::signer;
    use std::string::{Self, String};
    use std::vector;
    use aptos_framework::object::{Self, Object};

    use aptos_token_objects::token::Token;

    use tournament::admin;
    use tournament::round;
    use tournament::token_manager;
    use tournament::tournament_manager;

    friend tournament::aptos_tournament;

    #[test_only] friend tournament::trivia_unit_tests;
    #[test_only] friend tournament::main_unit_test;

    /// You are not the owner of the object.
    const E_NOT_OWNER: u64 = 0;
    /// The sha3_256(answer + challenge) you submitted don't match the original hashed answer.
    const E_INVALID_REVEALED_ANSWER: u64 = 1;
    /// The answer has not been revealed yet.
    const E_ANSWER_HAS_NOT_BEEN_REVEALED: u64 = 2;
    /// There is no such trivia game
    const E_NOT_A_GAME: u64 = 3;
    /// The object passed in does not have a TriviaPlayer resource. It is not playing this game.
    const E_NOT_A_TRIVIA_PLAYER: u64 = 4;
    /// The room must not be a limited room
    const E_ROOM_IS_LIMITED: u64 = 5;


    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TriviaGame has key {}

    // // Updated by the admin- this contains the questions and answers for the game.
    // this can't be on the `Game` object because there'd be circular dependencies with the `TriviaPlayer` struct
    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TriviaQuestion has key, drop, store {
        // we don't want to reveal the question too early
        // but instead of using an `Option<String>` let's keep it simple and just use an empty string
        // vs a non-empty string to indicate whether or not the question has been revealed.
        question: String,
        possible_answers: vector<String>,
        // this is the vector of possible answers
        // this is the revealed answer index
        // 255 = answer is not revealed yet!
        revealed_answer_index: u8,
    }

    #[resource_group_member(group = aptos_framework::object::ObjectGroup)]
    struct TriviaAnswer has key, drop, store, copy {
        answer_index: u8,
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                 Core game functionality                                //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    fun get_current_round_signer(tournament_signer: &signer, tournament_address: address): (address, signer) {
        let round_address = tournament_manager::get_round_address(tournament_address);
        let round_signer = round::get_round_signer<TriviaGame>(tournament_signer, round_address);
        (round_address, round_signer)
    }

    public(friend) fun add_players_returning(
        tournament_address: address,
        players: vector<Object<Token>>
    ): vector<address> {
        let admin_signer = admin::get_admin_signer();
        let tournament_signer = tournament_manager::get_tournament_signer(&admin_signer, tournament_address);
        let round_address = tournament_manager::get_round_address(tournament_address);

        let room_signers = round::add_players<TriviaGame>(&tournament_signer, round_address, players);
        assert!(option::is_none(&room_signers), E_ROOM_IS_LIMITED);

        vector::for_each_ref(&players, |player| {
            let token_address = object::object_address(player);
            let token_signer = token_manager::get_token_signer(&tournament_signer, token_address);
            move_to<TriviaAnswer>(&token_signer, TriviaAnswer { answer_index: 255 })
        });

        vector::empty()
    }

    public entry fun set_trivia_question(
        caller: &signer,
        tournament_address: address,
        question: String,
        answers: vector<String>,
    ) acquires TriviaQuestion {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(caller, tournament_address);

        let (round_address, round_signer) = get_current_round_signer(&tournament_signer, tournament_address);

        if (!exists<TriviaQuestion>(round_address)) {
            // create a Trivia object on the Round object
            move_to(&round_signer,
                TriviaQuestion {
                    question,
                    possible_answers: answers,
                    revealed_answer_index: 255,
                },
            );
        };

        let trivia_question = borrow_global_mut<TriviaQuestion>(round_address);
        trivia_question.question = question;
        trivia_question.possible_answers = answers;
        trivia_question.revealed_answer_index = 255;
    }

    public entry fun commit_answer(
        // the user
        user: &signer,
        // the user's player object (presumably, we verify it is)
        player_obj: Object<Token>,
        submitted_answer_index: u8
    ) acquires TriviaAnswer {
        let player_address = signer::address_of(user);
        assert!(object::owner(player_obj) == player_address, E_NOT_OWNER);

        let player_obj_addr = object::object_address(&player_obj);
        assert!(exists<TriviaAnswer>(player_obj_addr), E_NOT_A_TRIVIA_PLAYER);

        let player_obj = object::convert<Token, TriviaAnswer>(player_obj);
        let user_addr = signer::address_of(user);
        assert!(object::is_owner(player_obj, user_addr), E_NOT_OWNER);

        // assert!(exists<TriviaPlayer>(player_obj_addr), error::invalid_argument(ENOT_A_TRIVIA_PLAYER))
        let answer = borrow_global_mut<TriviaAnswer>(player_obj_addr);
        // TODO; FIX THIS!
        answer.answer_index = submitted_answer_index;
    }

    public fun handle_players_game_end(
        admin: &signer,
        tournament_address: address,
        players: vector<Object<Token>>,
    ) acquires TriviaAnswer, TriviaQuestion {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(admin, tournament_address);

        let (round_address, _round_signer) = get_current_round_signer(&tournament_signer, tournament_address);

        assert!(exists<TriviaQuestion>(round_address), E_NOT_A_GAME);
        let revealed_answer_index = borrow_global<TriviaQuestion>(round_address).revealed_answer_index;
        assert!(revealed_answer_index != 255, E_ANSWER_HAS_NOT_BEEN_REVEALED);

        vector::for_each_ref(&players, |player| {
            handle_player_game_end(&tournament_signer, player, revealed_answer_index);
        });
    }

    inline fun handle_player_game_end(
        tournament_signer: &signer,
        player: &Object<Token>,
        expected_answer: u8,
    ) acquires TriviaAnswer {
        let token_address = object::object_address(player);

        let player_is_correct = false;
        if (exists<TriviaAnswer>(token_address)) {
            let answer = move_from<TriviaAnswer>(token_address);
            player_is_correct = answer.answer_index == expected_answer;
        };

        if (!player_is_correct) {
            token_manager::mark_token_loss(tournament_signer, token_address);
        };
    }

    public entry fun reveal_answer(
        admin: &signer,
        tournament_address: address,
        revealed_answer_index: u8,
    ) acquires TriviaQuestion {
        let tournament_signer = admin::get_tournament_owner_signer_as_admin(admin, tournament_address);
        let (round_address, _round_signer) = get_current_round_signer(&tournament_signer, tournament_address);

        let trivia = borrow_global_mut<TriviaQuestion>(round_address);

        // let original_hashed_answer = trivia.hashed_answer;
        // let bytes_to_hash = bcs::to_bytes<String>(&revealed_answer);
        // vector::append(&mut bytes_to_hash, challenge);
        // let verified_hashed_answer = hash::sha3_256(bytes_to_hash);
        // assert!(original_hashed_answer == verified_hashed_answer, E_INVALID_REVEALED_ANSWER);

        assert!(vector::length(&trivia.possible_answers) > (revealed_answer_index as u64), E_INVALID_REVEALED_ANSWER);

        // it's valid, update/set the revealed answer
        trivia.revealed_answer_index = revealed_answer_index;
    }

    public fun destroy_and_cleanup_round(admin_signer: &signer, round_address: address) acquires TriviaQuestion {
        let admin = admin::get_admin_signer_as_admin(admin_signer);
        move_from<TriviaQuestion>(round_address);
        round::destroy_and_cleanup_round<TriviaGame>(&admin, round_address);
    }

    public fun destroy_and_cleanup_current_round(admin_signer: &signer, tournament_address: address) acquires TriviaQuestion {
        let round_address = tournament_manager::get_round_address(tournament_address);
        destroy_and_cleanup_round(admin_signer, round_address);
    }

    // -------------------------------------------------------------------------------------- //
    //                                                                                        //
    //                                  Views and test helpers                                //
    //                                                                                        //
    // -------------------------------------------------------------------------------------- //

    struct GameView has copy, drop, store {
        trivia_players: vector<Object<TriviaAnswer>>,
    }

    struct TriviaView has copy, drop, store {
        question: String,
        possible_answers: vector<String>,
        // hashed_answer: vector<u8>,
        revealed_answer_index: u8,
    }


    #[view]
    /// Viewing the Object<Trivia> with its address as input
    public fun view_trivia(tournament_address: address): TriviaView acquires TriviaQuestion {
        let round_address = tournament_manager::get_round_address(tournament_address);
        let trivia = borrow_global<TriviaQuestion>(round_address);
        TriviaView {
            question: trivia.question,
            possible_answers: trivia.possible_answers,
            // hashed_answer: trivia.hashed_answer,
            revealed_answer_index: trivia.revealed_answer_index
        }
    }

    #[view]
    /// Viewing the revealed answer of the Object<Trivia> with its address as input
    public fun view_revealed_answer(tournament_address: address): String acquires TriviaQuestion {
        let round_address = tournament_manager::get_round_address(tournament_address);
        let trivia = borrow_global<TriviaQuestion>(round_address);
        if ((trivia.revealed_answer_index as u64) >= vector::length(&trivia.possible_answers)) {
            string::utf8(b"")
        } else {
            *vector::borrow(&trivia.possible_answers, (trivia.revealed_answer_index as u64))
        }
    }

    #[view]
    /// Checking to see if the address is an Object<Trivia>
    public fun is_trivia(round_address: address): bool {
        object::is_object(round_address) && exists<TriviaQuestion>(round_address)
    }

    #[view]
    /// Viewing the Object<TriviaPlayer> with its address as input
    public fun view_trivia_player(trivia_player_token_addr: address): TriviaAnswer acquires TriviaAnswer {
        if (exists<TriviaAnswer>(trivia_player_token_addr)) {
            *borrow_global<TriviaAnswer>(trivia_player_token_addr)
        } else {
            TriviaAnswer {
                answer_index: 255
            }
        }
    }
}
