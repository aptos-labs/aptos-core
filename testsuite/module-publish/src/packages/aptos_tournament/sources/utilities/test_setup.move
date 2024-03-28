#[test_only]
module tournament::test_setup {
    use std::signer;
    use std::simple_map::{Self, SimpleMap};
    use std::string::{Self, String};
    use std::vector;

    use tournament::test_utils;
    use tournament::token_manager;

    friend tournament::trivia_unit_tests;
    friend tournament::main_unit_test;

    public fun setup_test(
        deployer: &signer,
        aptos_framework: &signer,
        start_time_seed: u64,
    ) {
        test_utils::enable_features_for_test(aptos_framework);
        token_manager::init_module_for_test(deployer);
        test_utils::init_test_framework(aptos_framework, start_time_seed);
    }


    struct QuestionBank has key, copy, drop, store {
        map: SimpleMap<String, String>,
    }

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

    // Gets the n-th question in the question bank and its possible answers, then returns the hashed answer of the first answer in the bank for convenience.
    public fun get_question_answer_and_hash(n: u64): (String, vector<String>) acquires QuestionBank {
        let question_bank = borrow_global<QuestionBank>(@tournament).map;
        let (questions, possible_answers) = simple_map::to_vec_pair(question_bank);
        let question = *vector::borrow(&questions, n);
        // let answer = *vector::borrow(&possible_answers, 0);
        // let answer_bytes = bcs::to_bytes(&possible_answers);
        // vector::append(&mut answer_bytes, CHALLENGE);
        // let hashed_answer = hash::sha3_256(answer_bytes);

        (question, possible_answers)
    }

    public(friend) fun setup_trivia(tournament_manager: &signer) {
        assert!(signer::address_of(tournament_manager) == @tournament, 0);
        let question_bank = vector::map<vector<u8>, String>(QUESTION_BANK, |question| {
            string::utf8(question)
        });
        let answer_bank = vector::map<vector<u8>, String>(ANSWER_BANK, |answer| {
            string::utf8(answer)
        });

        move_to(
            tournament_manager,
            QuestionBank {
                map: simple_map::new_from(question_bank, answer_bank),
            }
        );
    }
}
