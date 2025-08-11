#[test_only]
module voting::governance_tests {
    use std::signer;
    use std::string;
    use std::timestamp;
    use voting::test_helpers;
    use voting::governance;
    use voting::ve_token;
    // Copy pasted from governance.move since constants are private and cannot be reused.
    const VOTING_DURATION: u64 = 604800; // 7 days
    const PROPOSAL_MINIMUM_VOTING_POWER: u128 = 100000000000000; // 1M tokens with 8 decimals
    const MIN_VOTING_THRESHOLD: u128 = 10000000000000000; // 100M tokens with 8 decimals
    const PROPOSAL_DESCRIPTION_KEY: vector<u8> = b"proposal_description";

    // We don't use native enums since enums cannot be created or read outside of the module that defines them.
    const REQUEST_TYPE_GLOBAL_LOAN_BOOK_FEES: u64 = 1;
    const REQUEST_TYPE_GLOBAL_FACILITIES_FEES: u64 = 2;
    const REQUEST_TYPE_TOKEN_LOCKER_PARAMS: u64 = 3;
    const REQUEST_TYPE_GOVERNANCE_PARAMS: u64 = 4;
    const REQUEST_TYPE_TOKEN_TREASURY_DISTRIBUTION: u64 = 5;
    const REQUEST_TYPE_STAKING_REWARD_PARAMS: u64 = 6;
    const REQUEST_TYPE_BUY_BACK_PARAMS: u64 = 7;
    const MODULE_UPGRADE: u64 = 8;

    #[test(user = @0x1234)]
    fun test_update_governance_parameters(user: &signer) {
        test_helpers::setup();

        let user_addr = signer::address_of(user);
        test_helpers::mint_vote_tokens(user_addr, 1000);

        let (voting_duration, proposal_minimum_voting_power, min_voting_threshold) =
            governance::governance_parameters();
        assert!(voting_duration == VOTING_DURATION);
        assert!(proposal_minimum_voting_power == PROPOSAL_MINIMUM_VOTING_POWER);
        assert!(min_voting_threshold == MIN_VOTING_THRESHOLD);

        governance::update_governance_parameters(
            governance::create_resource_request(4),
            2000,
            2000,
            2000,
        );
        let (new_voting_duration, new_proposal_minimum_voting_power, new_min_voting_threshold) =
            governance::governance_parameters();
        assert!(new_voting_duration == 2000);
        assert!(new_proposal_minimum_voting_power == 2000);
        assert!(new_min_voting_threshold == 2000);
    }

    #[test(proposer = @0x1234, user_1 = @0x1235, user_2 = @0x1236)]
    fun test_create_proposal_and_vote(proposer: &signer, user_1: &signer, user_2: &signer) {
        test_helpers::setup();

        let proposer_addr = signer::address_of(proposer);
        let user_1_addr = signer::address_of(user_1);
        let user_2_addr = signer::address_of(user_2);
        let user_1_token_amount = 1000;
        let user_2_token_amount = 500;
        test_helpers::mint_vote_tokens(proposer_addr, PROPOSAL_MINIMUM_VOTING_POWER as u64);
        test_helpers::mint_vote_tokens(user_1_addr, user_1_token_amount);
        test_helpers::mint_vote_tokens(user_2_addr, user_2_token_amount);
        ve_token::lock(proposer, PROPOSAL_MINIMUM_VOTING_POWER as u64);
        ve_token::lock(user_1, user_1_token_amount);
        ve_token::lock(user_2, user_2_token_amount);
        test_helpers::fast_forward_epochs(53);

        governance::create_proposal(
            proposer,
            string::utf8(b"test proposal"),
            b"123",
            true,
        );
        governance::vote(user_1, 0, true);
        governance::vote(user_2, 0, false);

        let (expiration_secs, _, min_vote_threshold, _, yes_votes, no_votes) =
            governance::proposal_data(0);
        assert!(expiration_secs == timestamp::now_seconds() + VOTING_DURATION);
        assert!(min_vote_threshold == MIN_VOTING_THRESHOLD);
        // Max multiplier is 5x (100% + 53 weeks / 12)
        assert!(yes_votes == 5000);
        assert!(no_votes == 2500);
    }
}
